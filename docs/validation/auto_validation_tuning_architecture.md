# Automatic Pipeline Validation and Tuning Architecture

**Document ID**: ARCH-AUTO-VAL-001
**Version**: 1.0
**Date**: 2025-10-22
**Status**: Implementation

## Purpose

Automated runtime validation and performance tuning system for the WKMP audio pipeline. Ensures sample integrity while adapting to different hardware environments (Desktop, Laptop, Raspberry Pi, etc.).

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                       PlaybackEngine                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │   Decoder    │→ │    Buffer    │→ │    Mixer     │→ Output │
│  │   Worker     │  │   Manager    │  │  (w/counter) │         │
│  └──────────────┘  └──────────────┘  └──────────────┘         │
│         ↓                  ↓                  ↓                  │
│  ┌──────────────────────────────────────────────────────┐      │
│  │          ValidationService (Background Task)          │      │
│  │  • Periodic validation (every 10s during playback)   │      │
│  │  • Conservation law checking                          │      │
│  │  • Event emission on failure                          │      │
│  │  • History tracking (last 100 checks)                 │      │
│  └──────────────────────────────────────────────────────┘      │
│         ↓                                                        │
│  ┌──────────────────────────────────────────────────────┐      │
│  │          AutoTuningService (Background Task)          │      │
│  │  • Hardware profiling on startup                      │      │
│  │  • Parameter adjustment based on validation           │      │
│  │  • Underrun frequency monitoring                      │      │
│  │  • Adaptive buffer sizing                             │      │
│  └──────────────────────────────────────────────────────┘      │
└─────────────────────────────────────────────────────────────────┘
                              ↓
                    ┌─────────────────────┐
                    │   Event Broadcast   │
                    │  • SSE to UI        │
                    │  • Logging          │
                    │  • Metrics          │
                    └─────────────────────┘
```

## Components

### 1. ValidationService

**Purpose**: Periodic pipeline integrity validation during playback

**Responsibilities**:
- Run validation every 10 seconds during active playback
- Collect metrics from engine via `get_pipeline_metrics()`
- Execute conservation law validation with tolerance
- Emit events on validation failure
- Maintain validation history (ring buffer, last 100 checks)
- Provide API access to current status and history

**Configuration**:
```rust
pub struct ValidationConfig {
    /// Validation interval in seconds (default: 10s)
    pub interval_secs: u64,

    /// Sample tolerance for validation (default: 8192 samples)
    pub tolerance_samples: u64,

    /// Enable automatic validation (default: true)
    pub enabled: bool,

    /// Maximum history entries to keep (default: 100)
    pub history_size: usize,
}
```

**Events Emitted**:
- `ValidationSuccess` - Periodic successful validation
- `ValidationFailure` - Conservation law violated
- `ValidationWarning` - Close to tolerance threshold (>80%)

### 2. Hardware Profiles

| Profile | CPU Cores | RAM | Platform | Buffer Size | Chunk Size |
|---------|-----------|-----|----------|-------------|------------|
| Desktop | 8+ | 16GB+ | x86_64 | 661,941 (15s) | 32768 |
| Laptop | 4-8 | 8-16GB | x86_64 | 441,000 (10s) | 16384 |
| RPi 4 | 4 | 4-8GB | aarch64 | 220,500 (5s) | 8192 |
| RPi Zero | 1-2 | 512MB-1GB | armv7 | 88,200 (2s) | 4096 |

## Implementation Phases

### Phase 1: Validation Service (✅ COMPLETE)
- [x] Core validation logic (`diagnostics.rs` - 351 lines, 6 tests)
- [x] Engine metrics API (`get_pipeline_metrics()`)
- [x] ValidationService background task (`validation_service.rs` - 331 lines, 2 tests)
- [x] API endpoints for diagnostics (`GET /playback/diagnostics`)
- [x] Event emission (ValidationSuccess, ValidationFailure, ValidationWarning)
- [x] Database settings integration (3 configurable parameters)
- [x] Automatic startup on engine initialization
- [x] SSE event broadcasting tested and verified

### Phase 2: Auto-Tuning (Future)
- [ ] Hardware detection
- [ ] Profile-based defaults
- [ ] Underrun monitoring
- [ ] Parameter adjustment logic

### Phase 3: UI Integration (Future)
- [ ] Validation status in developer UI
- [ ] Real-time validation chart
- [ ] Parameter tuning controls

## Configuration Storage

**Database Settings** (added to `settings` table):

| Key | Default | Description |
|-----|---------|-------------|
| `validation_enabled` | `true` | Enable periodic validation |
| `validation_interval_secs` | `10` | Validation check interval |
| `validation_tolerance_samples` | `8192` | Sample count tolerance |

## Implementation Details (Phase 1)

### File Structure

```
wkmp-common/
  src/db/init.rs              # Database settings initialization
  src/events.rs               # Validation SSE events

wkmp-ap/src/playback/
  validation_service.rs       # ValidationService (331 lines)
  diagnostics.rs              # Validation logic (351 lines)
  engine.rs                   # Service integration

wkmp-ap/src/
  main.rs                     # Automatic startup
```

### Database Settings

| Setting | Default | Type | Description |
|---------|---------|------|-------------|
| `validation_enabled` | `true` | boolean | Enable/disable automatic validation |
| `validation_interval_secs` | `10` | integer | Validation check interval (seconds) |
| `validation_tolerance_samples` | `8192` | integer | Sample count tolerance (~0.18s @ 44.1kHz) |

### API Endpoints

**GET `/playback/diagnostics`**
- Returns current pipeline validation results
- Response includes all metrics and any errors
- Used for manual/on-demand validation checks

### SSE Events

**ValidationSuccess**
```json
{
  "type": "ValidationSuccess",
  "timestamp": "2025-10-22T16:09:51Z",
  "passage_count": 3,
  "total_decoder_samples": 4770638,
  "total_buffer_written": 4770638,
  "total_buffer_read": 880000,
  "total_mixer_frames": 440000
}
```

**ValidationFailure**
```json
{
  "type": "ValidationFailure",
  "timestamp": "2025-10-22T16:09:51Z",
  "passage_count": 1,
  "total_decoder_samples": 1000,
  "total_buffer_written": 1000,
  "total_buffer_read": 1500,
  "total_mixer_frames": 750,
  "errors": [
    "Buffer FIFO Violation (Rule 2): Read 500 more samples than written"
  ]
}
```

**ValidationWarning**
```json
{
  "type": "ValidationWarning",
  "timestamp": "2025-10-22T16:09:51Z",
  "passage_count": 1,
  "total_decoder_samples": 100000,
  "total_buffer_written": 107000,
  "total_buffer_read": 50000,
  "total_mixer_frames": 25000,
  "warnings": [
    "Approaching tolerance threshold: 7000 samples (85% of limit)"
  ]
}
```

### Test Results

**Verified Functionality:**
- ✅ Database settings loaded correctly
- ✅ Service starts automatically on engine init
- ✅ Validation runs every 10 seconds during playback
- ✅ SSE events emitted successfully
- ✅ Conservation laws validated correctly
- ✅ No performance impact (lock-free metrics)

**Test Output:**
```
[INFO] Initialized setting 'validation_enabled' with default value: true
[INFO] Initialized setting 'validation_interval_secs' with default value: 10
[INFO] Initialized setting 'validation_tolerance_samples' with default value: 8192
[INFO] Starting ValidationService (interval: 10s, tolerance: 8192 samples)
[DEBUG] ValidationService: PASS (passages: 3, errors: 0)
```

## Safety Considerations

1. **FIFO Violation = Critical**:
   - Rule 2 violation (buffer read > write) indicates memory corruption
   - Immediate playback stop and logging

2. **Conservative Approach**:
   - Start with observation only (no auto-tuning)
   - Log all validation results for analysis

3. **Zero-Overhead Design**:
   - Lock-free atomic counters (Relaxed ordering)
   - Validation only runs during playback
   - No impact on audio callback thread
