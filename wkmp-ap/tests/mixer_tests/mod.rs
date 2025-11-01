//! Mixer unit tests (Increment 5)
//!
//! Tests for SPEC016-compliant mixer isolated from PlaybackEngine.
//! Per Option B testing strategy, validates marker system and position tracking
//! before full integration.

mod helpers;

// Test suites (Increment 5 - Unit Tests)
mod test_marker_storage;
mod test_position_tracking;
mod test_marker_events;
mod test_event_types;
mod test_eof_handling;

// Integration test suites (Increment 6)
mod test_integration_extended;
mod test_integration_crossfade;
mod test_integration_state;
mod test_integration_volume;
