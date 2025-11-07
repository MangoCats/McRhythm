#!/bin/bash
# Script to update remaining DEBUG logs to TRACE in wkmp-ap

# playout_ring_buffer.rs - can_decoder_resume
sed -i '589s/debug!/trace!/' wkmp-ap/src/playback/playout_ring_buffer.rs

# playout_ring_buffer.rs - add trace import
sed -i 's/use tracing::{debug, warn};/use tracing::{debug, trace, warn};/' wkmp-ap/src/playback/playout_ring_buffer.rs

# buffer_manager.rs - ring buffer full
sed -i '481s/debug!($/trace!(/' wkmp-ap/src/playback/buffer_manager.rs

# buffer_manager.rs - add trace import
sed -i 's/use tracing::{debug, info, warn};/use tracing::{debug, info, trace, warn};/' wkmp-ap/src/playback/buffer_manager.rs

# engine/playback.rs - watchdog logs (multiple occurrences)
sed -i '667s/debug!(/trace!(/' wkmp-ap/src/playback/engine/playback.rs
sed -i '686s/debug!(/trace!(/' wkmp-ap/src/playback/engine/playback.rs
sed -i '1048s/debug!(/trace!(/' wkmp-ap/src/playback/engine/playback.rs
sed -i '1052s/debug!(/trace!(/' wkmp-ap/src/playback/engine/playback.rs

# engine/playback.rs - add trace import
sed -i 's/use tracing::{debug, error, info, warn};/use tracing::{debug, error, info, trace, warn};/' wkmp-ap/src/playback/engine/playback.rs

# engine/diagnostics.rs - buffer ready log
sed -i '684s/debug!(/trace!(/' wkmp-ap/src/playback/engine/diagnostics.rs

# engine/diagnostics.rs - target_sample_rate logs
sed -i '215s/debug!(/trace!(/' wkmp-ap/src/playback/engine/diagnostics.rs

# engine/diagnostics.rs - add trace import
sed -i 's/use tracing::{debug, error, info, warn};/use tracing::{debug, error, info, trace, warn};/' wkmp-ap/src/playback/engine/diagnostics.rs

# audio/decoder.rs - decoded chunk logs
sed -i '977s/debug!(/trace!(/' wkmp-ap/src/audio/decoder.rs

# audio/decoder.rs - add trace import
sed -i 's/use tracing::{debug, warn};/use tracing::{debug, trace, warn};/' wkmp-ap/src/audio/decoder.rs

echo "Log level updates complete"
