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

### Phase 1: Validation Service (Current)
- [x] Core validation logic (`diagnostics.rs`)
- [x] Engine metrics API (`get_pipeline_metrics()`)
- [ ] ValidationService background task
- [ ] API endpoints for diagnostics
- [ ] Event emission

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

## Safety Considerations

1. **FIFO Violation = Critical**:
   - Rule 2 violation (buffer read > write) indicates memory corruption
   - Immediate playback stop and logging

2. **Conservative Approach**:
   - Start with observation only (no auto-tuning)
   - Log all validation results for analysis
