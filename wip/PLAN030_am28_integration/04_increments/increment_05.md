# Increment 5: Single-Track Discriminator

**Estimated Effort:** 3 hours
**Dependencies:** Increment 4
**Deliverables:** matching/single_track.rs

---

## Objective

Create single-track discriminator to reject files that contain only one track (not albums).

---

## Source Files

| am28 File | Lines | Action |
|-----------|-------|--------|
| single_track_discriminator.rs | 370 | Adapt to library |

---

## Tasks

### 5.1 Create matching/single_track.rs

```rust
//! Single-Track Discriminator
//!
//! Detects files that contain only a single track (not full albums).
//! Uses heuristics before and after audio decode.

use std::path::Path;

/// Result of single-track analysis
#[derive(Debug, Clone)]
pub struct SingleTrackAnalysis {
    /// Is this likely a single track?
    pub is_single_track: bool,
    /// Confidence in the decision (0.0-1.0)
    pub confidence: f64,
    /// Reason for decision
    pub reason: SingleTrackReason,
    /// File duration in seconds (if known)
    pub duration_secs: Option<f64>,
    /// Number of silence regions detected (if known)
    pub silence_count: Option<usize>,
}

/// Reason for single-track classification
#[derive(Debug, Clone)]
pub enum SingleTrackReason {
    /// File too short to be album (< 12 minutes)
    TooShort,
    /// No silence regions detected
    NoSilence,
    /// Only 0-1 silence regions (single track with intro/outro)
    FewSilenceRegions,
    /// Likely an album (multiple silence regions)
    LikelyAlbum,
    /// Unknown - needs more analysis
    Unknown,
}

/// Thresholds for single-track detection
pub struct SingleTrackThresholds {
    /// Minimum duration for album consideration (seconds)
    pub min_album_duration_secs: f64,
    /// Minimum silence regions for album
    pub min_silence_regions: usize,
    /// File size threshold for pre-decode (bytes)
    pub min_file_size_bytes: u64,
}

impl Default for SingleTrackThresholds {
    fn default() -> Self {
        Self {
            min_album_duration_secs: 12.0 * 60.0, // 12 minutes
            min_silence_regions: 2,               // At least 2 silence gaps
            min_file_size_bytes: 15_000_000,      // ~15 MB (rough album threshold)
        }
    }
}

/// Analyze file before decoding (fast heuristics)
pub fn analyze_pre_decode(
    file_path: &Path,
    duration_hint: Option<f64>,
    thresholds: &SingleTrackThresholds,
) -> SingleTrackAnalysis {
    // Check file size as quick heuristic
    let file_size = std::fs::metadata(file_path)
        .map(|m| m.len())
        .unwrap_or(0);

    if file_size < thresholds.min_file_size_bytes {
        return SingleTrackAnalysis {
            is_single_track: true,
            confidence: 0.6,
            reason: SingleTrackReason::TooShort,
            duration_secs: duration_hint,
            silence_count: None,
        };
    }

    // If we have duration hint, use it
    if let Some(duration) = duration_hint {
        if duration < thresholds.min_album_duration_secs {
            return SingleTrackAnalysis {
                is_single_track: true,
                confidence: 0.8,
                reason: SingleTrackReason::TooShort,
                duration_secs: Some(duration),
                silence_count: None,
            };
        }
    }

    // Need more analysis
    SingleTrackAnalysis {
        is_single_track: false,
        confidence: 0.3,
        reason: SingleTrackReason::Unknown,
        duration_secs: duration_hint,
        silence_count: None,
    }
}

/// Analyze after decoding (with silence detection)
pub fn analyze_post_decode(
    pre_analysis: &SingleTrackAnalysis,
    total_duration_secs: f64,
    silence_regions: usize,
    thresholds: &SingleTrackThresholds,
) -> SingleTrackAnalysis {
    // Duration check
    if total_duration_secs < thresholds.min_album_duration_secs {
        return SingleTrackAnalysis {
            is_single_track: true,
            confidence: 0.95,
            reason: SingleTrackReason::TooShort,
            duration_secs: Some(total_duration_secs),
            silence_count: Some(silence_regions),
        };
    }

    // Silence region check
    if silence_regions == 0 {
        return SingleTrackAnalysis {
            is_single_track: true,
            confidence: 0.9,
            reason: SingleTrackReason::NoSilence,
            duration_secs: Some(total_duration_secs),
            silence_count: Some(silence_regions),
        };
    }

    if silence_regions < thresholds.min_silence_regions {
        return SingleTrackAnalysis {
            is_single_track: true,
            confidence: 0.7,
            reason: SingleTrackReason::FewSilenceRegions,
            duration_secs: Some(total_duration_secs),
            silence_count: Some(silence_regions),
        };
    }

    // Likely an album
    SingleTrackAnalysis {
        is_single_track: false,
        confidence: 0.85,
        reason: SingleTrackReason::LikelyAlbum,
        duration_secs: Some(total_duration_secs),
        silence_count: Some(silence_regions),
    }
}
```

### 5.2 Add Module to matching/mod.rs

```rust
pub mod single_track;
pub use single_track::{SingleTrackAnalysis, SingleTrackReason, SingleTrackThresholds, analyze_pre_decode, analyze_post_decode};
```

---

## Tests

| Test ID | Description | Pass Criteria |
|---------|-------------|---------------|
| TC-U-005-01 | Pre-decode analysis (file size) | Small files flagged |
| TC-U-005-02 | Post-decode analysis (silence count) | 0 silence = single track |
| TC-U-005-03 | Single-track rejection decision | is_single_track = true |
| TC-U-005-04 | Album acceptance decision | is_single_track = false |

---

## Acceptance Criteria

- [ ] single_track.rs created
- [ ] Pre-decode heuristics working
- [ ] Post-decode analysis working
- [ ] Confidence levels reasonable
- [ ] All 4 tests pass
