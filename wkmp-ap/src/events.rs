//! Event system for wkmp-ap Audio Player
//!
//! Implements event-driven communication per SPEC011-event_system.md
//!
//! # Architecture
//!
//! WKMP uses hybrid communication:
//! - **EventBus** (tokio::broadcast): One-to-many event broadcasting
//! - **Command channels** (tokio::mpsc): Request → single handler
//! - **Shared state** (Arc<RwLock<T>>): Read-heavy access
//!
//! This module re-exports shared event types from wkmp-common and defines
//! wkmp-ap-specific internal event types.

use uuid::Uuid;

// ========================================
// Re-exports from wkmp-common
// ========================================

pub use wkmp_common::events::{
    BufferStatus, EnqueueSource, EventBus, PlaybackState, QueueChangeTrigger, UserActionType,
    WkmpEvent,
};

// ========================================
// Internal Events (wkmp-ap only)
// ========================================

/// Mixer state context for position events
///
/// Per SPEC011 EVT-CTX-010: Added in v1.4 specification update
#[derive(Debug, Clone)]
pub enum MixerStateContext {
    /// Single passage playing (no crossfade active)
    Immediate,

    /// Crossfade in progress
    Crossfading {
        /// Queue entry ID of incoming passage
        incoming_queue_entry_id: Uuid,
    },
}

/// Internal playback events (not exposed via SSE)
///
/// These events are private implementation details within wkmp-ap:
/// - NOT serialized or sent via SSE
/// - One-to-one MPSC pattern (mixer → handler)
/// - Non-blocking emission (`try_send()` to avoid blocking audio thread)
///
/// Per SPEC011: Used for event-driven position tracking without timer polling
#[derive(Debug, Clone)]
pub enum PlaybackEvent {
    /// Position update from mixer
    ///
    /// Emitted at configurable interval (database setting: position_event_interval_ms, default: 1000ms)
    /// Example: At 44.1kHz with 1000ms interval → every 44,100 frames (~1 second of audio)
    PositionUpdate {
        /// Queue entry ID of current passage
        queue_entry_id: Uuid,

        /// Frame position within buffer
        position_frames: usize,

        /// Sample rate (for ms conversion)
        sample_rate: u32,

        /// Mixer state context
        state: MixerStateContext,
    },

    /// Mixer state changed (e.g., started crossfade)
    StateChanged {
        queue_entry_id: Uuid,
        new_state: MixerStateContext,
    },
}

// ========================================
// Tests
// ========================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_eventbus_new() {
        let bus = EventBus::new(100);
        assert_eq!(bus.capacity(), 100);
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[test]
    fn test_eventbus_subscribe() {
        let bus = EventBus::new(100);
        let _rx = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 1);

        let _rx2 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 2);
    }

    #[tokio::test]
    async fn test_eventbus_emit_no_subscribers() {
        let bus = EventBus::new(100);
        let event = WkmpEvent::PlaybackStateChanged {
            old_state: PlaybackState::Paused,
            new_state: PlaybackState::Playing,
            timestamp: chrono::Utc::now(),
        };

        // Should return error when no subscribers
        assert!(bus.emit(event).is_err());
    }

    #[tokio::test]
    async fn test_eventbus_emit_with_subscriber() {
        let bus = Arc::new(EventBus::new(100));
        let mut rx = bus.subscribe();

        let event = WkmpEvent::PlaybackStateChanged {
            old_state: PlaybackState::Paused,
            new_state: PlaybackState::Playing,
            timestamp: chrono::Utc::now(),
        };

        // Should succeed with subscriber
        assert!(bus.emit(event.clone()).is_ok());

        // Should receive event
        let received = rx.recv().await.unwrap();
        match received {
            WkmpEvent::PlaybackStateChanged {
                old_state,
                new_state,
                ..
            } => {
                assert_eq!(old_state, PlaybackState::Paused);
                assert_eq!(new_state, PlaybackState::Playing);
            }
            _ => panic!("Wrong event type received"),
        }
    }

    #[tokio::test]
    async fn test_eventbus_emit_lossy() {
        let bus = EventBus::new(100);
        let event = WkmpEvent::PlaybackProgress {
            passage_id: Uuid::new_v4(),
            position_ms: 1000,
            duration_ms: 60000,
            timestamp: chrono::Utc::now(),
        };

        // Should not panic even without subscribers
        bus.emit_lossy(event);
    }

    #[test]
    fn test_playback_state_equality() {
        assert_eq!(PlaybackState::Playing, PlaybackState::Playing);
        assert_ne!(PlaybackState::Playing, PlaybackState::Paused);
    }

    #[test]
    fn test_buffer_status_equality() {
        assert_eq!(BufferStatus::Decoding, BufferStatus::Decoding);
        assert_ne!(BufferStatus::Decoding, BufferStatus::Ready);
    }

    #[test]
    fn test_mixer_state_context() {
        let immediate = MixerStateContext::Immediate;
        let crossfading = MixerStateContext::Crossfading {
            incoming_queue_entry_id: Uuid::new_v4(),
        };

        // Just verify they can be constructed
        match immediate {
            MixerStateContext::Immediate => {}
            _ => panic!("Expected Immediate"),
        }

        match crossfading {
            MixerStateContext::Crossfading { .. } => {}
            _ => panic!("Expected Crossfading"),
        }
    }
}
