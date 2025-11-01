//! Internal playback events (not exposed via SSE)
//!
//! This module defines internal event types used for communication between
//! playback components (mixer → engine). These events are NOT exposed via SSE;
//! they are converted to WkmpEvent types before broadcasting.
//!
//! **Traceability:**
//! - [SPEC011-event_system.md] Internal vs External Events
//! - [REV002] Event-driven architecture update
//! - [ARCH-SNGC-030] Event-driven position tracking

use uuid::Uuid;

/// Internal playback events for mixer → engine communication
///
/// These events are emitted by the mixer during audio frame generation
/// and consumed by the position event handler in the playback engine.
///
/// **Design Note:** These are internal-only events. For SSE events visible
/// to clients, see `wkmp_common::events::WkmpEvent`.
#[derive(Debug, Clone)]
pub enum PlaybackEvent {
    /// Position update from mixer
    ///
    /// Emitted periodically by mixer during frame generation.
    /// Frequency controlled by `position_event_interval_ms` database setting.
    ///
    /// **Traceability:**
    /// - [ADDENDUM-interval_configurability.md] Position event interval configuration
    /// - [ARCH-SNGC-030] Configurable interval specification
    ///
    /// # Fields
    /// * `queue_entry_id` - UUID of currently playing queue entry
    /// * `position_ms` - Current playback position in milliseconds
    PositionUpdate {
        queue_entry_id: Uuid,
        position_ms: u64,
    },

    /// Passage playback completed
    ///
    /// Emitted when mixer reaches end of passage (PassageComplete marker fires).
    /// Engine uses this to advance queue to next passage.
    ///
    /// **[SUB-INC-4B]** Added for marker-driven queue advancement
    ///
    /// # Fields
    /// * `queue_entry_id` - UUID of completed queue entry
    PassageComplete {
        queue_entry_id: Uuid,
    },

    /// Playback state changed (reserved for future use)
    ///
    /// This variant is reserved for future implementation of additional
    /// state change notifications beyond position updates.
    ///
    /// **Phase 4:** StateChanged variant reserved for future state transition events
    #[allow(dead_code)]
    StateChanged {
        // Reserved for future implementation
        // Possible fields: PlaybackState, timestamp, reason
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_update_creation() {
        let queue_entry_id = Uuid::new_v4();
        let position_ms = 5000;

        let event = PlaybackEvent::PositionUpdate {
            queue_entry_id,
            position_ms,
        };

        match event {
            PlaybackEvent::PositionUpdate { queue_entry_id: id, position_ms: pos } => {
                assert_eq!(id, queue_entry_id);
                assert_eq!(pos, 5000);
            }
            _ => panic!("Expected PositionUpdate variant"),
        }
    }

    #[test]
    fn test_state_changed_creation() {
        let event = PlaybackEvent::StateChanged {};

        match event {
            PlaybackEvent::StateChanged { .. } => {
                // Successfully created
            }
            _ => panic!("Expected StateChanged variant"),
        }
    }

    #[test]
    fn test_event_clone() {
        let queue_entry_id = Uuid::new_v4();
        let event = PlaybackEvent::PositionUpdate {
            queue_entry_id,
            position_ms: 1000,
        };

        let cloned = event.clone();

        match (&event, &cloned) {
            (
                PlaybackEvent::PositionUpdate { queue_entry_id: id1, position_ms: pos1 },
                PlaybackEvent::PositionUpdate { queue_entry_id: id2, position_ms: pos2 },
            ) => {
                assert_eq!(id1, id2);
                assert_eq!(pos1, pos2);
            }
            _ => panic!("Clone failed"),
        }
    }

    #[test]
    fn test_event_debug() {
        let queue_entry_id = Uuid::new_v4();
        let event = PlaybackEvent::PositionUpdate {
            queue_entry_id,
            position_ms: 2500,
        };

        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("PositionUpdate"));
        assert!(debug_str.contains("2500"));
    }
}
