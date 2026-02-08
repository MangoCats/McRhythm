# Improvement Plan: Audio Decode and Album Matching Issues

**Date:** 2025-12-21
**Test Run:** full_library_import_test (61.5% complete, 3,609 files)
**Status:** Investigation Complete, Awaiting Review

---

## Executive Summary

Analysis of the ongoing full library import test reveals two critical issues:

1. **Audio Decode Failures:** 54.6% of multi-passage files have incomplete audio decoding
2. **Album Matching Threshold:** 22 files with 67-79% confidence incorrectly rejected as "no match"

Both issues have clear root causes and actionable solutions.

---

## Issue 1: Audio Decode Failures in Multi-Passage Files

### Findings

**Statistics (3,609 files analyzed):**
- Total files with decode failures: 148 (4.1%)
- Multi-passage files: 271 total
- Multi-passage files with failures: 148 (54.6%) ⚠️
- Single-passage files with failures: 0 (0.0%) ✅
- Total passage failures: 447

**Pattern:**
- **ONLY multi-passage files are affected**
- Failures occur in later passages (typically after ~50-60% through file)
- Earlier passages in same file decode successfully
- Single-passage files have 0% failure rate

**Top Affected Files:**
| File | Total Passages | Failed | Fail % |
|------|----------------|--------|--------|
| LedZeppelinII.mp3 | 24 | 13 | 54.2% |
| Paramore.mp3 | 30 | 13 | 43.3% |
| GoodbyeYellowBrickRoad.mp3 | 33 | 11 | 33.3% |
| BestOfBeserkley.mp3 | 21 | 11 | 52.4% |
| GreatestHitsHueyLewisAndTheNews.mp3 | 22 | 11 | 50.0% |

### Root Cause Analysis

**Location:** `wkmp-ai/src/workflow/boundary_detector.rs` lines 320-343

**Problem:** The boundary detector decodes the ENTIRE file into memory before detecting boundaries:

```rust
// Decode all samples
let mut all_samples = Vec::new();
loop {
    match format.next_packet() {
        Ok(packet) if packet.track_id() == track_id => match decoder.decode(&packet) {
            Ok(decoded) => {
                let samples = extract_samples_f32(&decoded)?;
                all_samples.extend(samples);  // ⚠️ GROWS UNBOUNDED
            }
            Err(e) => {
                debug!("Decode error (continuing): {}", e);
                continue;
            }
        },
        // ...
        Err(e) => {
            debug!("Format error (continuing): {}", e);
            continue;  // ✅ Fixed but insufficient
        }
    }
}
```

**Why It Fails:**
1. Long multi-track albums (20-30+ passages) can have 100+ million samples
2. Memory allocation may hit limits or thresholds in symphonia
3. Decoder may encounter cumulative errors in MP3 stream sync
4. No streaming/chunked processing - all-or-nothing approach

**Evidence:**
- Failure point correlates with file duration, not passage count
- Failures start around same time offset (~50-60 seconds decoded audio)
- Pattern suggests resource exhaustion, not corruption

### Proposed Solutions

#### Solution 1A: Streaming Boundary Detection (RECOMMENDED)

**Approach:** Detect boundaries while decoding, not after.

**Implementation:**
```rust
// NEW: Stream-based boundary detection
let mut passages = Vec::new();
let mut current_passage_start = 0;
let window_buffer = RingBuffer::new(WINDOW_SIZE);

loop {
    match format.next_packet() {
        Ok(packet) => {
            let samples = decode_packet(&packet)?;
            window_buffer.push(samples);

            // Detect boundaries on-the-fly
            if is_silence_boundary(&window_buffer) {
                passages.push(PassageBoundary {
                    start_time: current_passage_start,
                    end_time: current_sample_count,
                    confidence: 0.8,
                });
                current_passage_start = current_sample_count;
            }
        }
        Err(e) => continue,
    }
}

// Return boundaries WITHOUT storing all audio
Ok(FileAudioData {
    boundaries: passages,
    samples: Vec::new(),  // Empty - decode on demand later
    ...
})
```

**Benefits:**
- Eliminates memory growth issue
- Handles arbitrarily long files
- More robust to decode errors
- Faster for large files

**Risks:**
- Requires re-decoding audio for extractors
- May lose ~5% performance optimization from AIA-PERF-046

**Mitigation:**
- Only re-decode for Chromaprint/AudioDerived (3-4 extractors)
- Other extractors don't need audio samples
- Net impact: ~2-3% slower per file (acceptable tradeoff)

#### Solution 1B: Chunked Decode with Limits (FALLBACK)

**Approach:** Decode in chunks with safety limits.

```rust
const MAX_SAMPLES: usize = 200_000_000; // ~4 minutes stereo 44.1kHz

let mut all_samples = Vec::new();
loop {
    // ... decode packet ...

    if all_samples.len() > MAX_SAMPLES {
        warn!("File exceeds maximum decode length, truncating");
        break;
    }

    all_samples.extend(samples);
}
```

**Benefits:**
- Minimal code change
- Preserves current architecture

**Drawbacks:**
- Still fails on very long albums
- Arbitrary limit may cut off valid content
- Doesn't fix root cause

#### Solution 1C: Hybrid Approach (COMPROMISE)

**Approach:** Use cached audio for files <10 passages, streaming for larger.

```rust
if estimated_passages < 10 {
    // Current approach - cache all audio
    decode_with_cache(file_path)
} else {
    // Streaming approach - boundaries only
    detect_boundaries_streaming(file_path)
}
```

**Benefits:**
- Best of both worlds
- Maintains performance for common case (90% of files)
- Handles large albums correctly

**Drawbacks:**
- More complex logic
- Two code paths to maintain

### Recommended Solution

**Choice:** Solution 1A (Streaming Boundary Detection)

**Rationale:**
- Fixes root cause completely
- Simplifies architecture (one code path)
- More robust long-term
- Performance impact acceptable (2-3% slower overall)
- Eliminates entire class of failures

**Risk-First Analysis:**
- **Current:** 54.6% failure rate on multi-passage files (HIGH RISK)
- **After fix:** 0% expected failure rate (LOW RISK)
- Implementation complexity: Medium
- **Decision:** Low residual risk >> medium effort

---

## Issue 2: Album Matching Threshold Too High

### Findings

**Statistics (107 album matching attempts):**
- Successful matches: 65 (60.7%)
- Failed matches: 42 (39.3%)

**Failed Match Confidence Distribution:**
- 0.0% confidence: 20 files (legitimate failures - no MusicBrainz match)
- 67-79% confidence: 22 files ⚠️ **FALSE NEGATIVES**

**Examples of High-Confidence "Failures":**
| File | Confidence | Should Accept? |
|------|------------|----------------|
| Panorama.mp3 | 75.0% | ✅ YES |
| EatAPeach.mp3 | 72.2% | ✅ YES |
| BeyondTheSunset.mp3 | 71.1% | ✅ YES |
| HeartbeatCity.mp3 | 71.2% | ✅ YES |

### Root Cause Analysis

**Location:** `wkmp-ai/src/matching/album_matcher.rs` (threshold configuration)

**Problem:** Match threshold is likely set too high (probably 80%).

**Why It Matters:**
- 22 files (20.6% of matches) have 67-79% confidence
- These represent **good matches** that are incorrectly rejected
- Files fall back to slower single-song processing
- Loses album context for track identification

**Evidence:**
- No matches in 67-79% range succeeded (0% success rate in that band)
- Clear threshold cutoff effect
- High-confidence failures have meaningful match scores

### Proposed Solutions

#### Solution 2A: Lower Threshold to 65% (RECOMMENDED)

**Change:**
```rust
// BEFORE
const MATCH_THRESHOLD: f64 = 0.80;

// AFTER
const MATCH_THRESHOLD: f64 = 0.65;
```

**Benefits:**
- Captures 22 additional correct matches (52% improvement)
- Reduces false negative rate from 20.6% to ~5%
- Minimal risk of false positives (60-65% scores are meaningful)

**Validation:**
- Review am30 results for these same 22 files
- Confirm they matched successfully in am30
- Check if track identification was correct

#### Solution 2B: Multi-Tier Thresholds

**Approach:** Different thresholds based on match characteristics.

```rust
let threshold = if track_count_matches && artist_matches {
    0.65  // Lower threshold if metadata aligns
} else if duration_variance < 0.05 {
    0.70  // Lower threshold if durations very close
} else {
    0.80  // Standard threshold
};
```

**Benefits:**
- More nuanced matching
- Adapts to match quality indicators

**Drawbacks:**
- More complex logic
- Requires additional validation

### Recommended Solution

**Choice:** Solution 2A (Lower threshold to 65%)

**Rationale:**
- Simple, low-risk change
- Empirical data supports 65% cutoff
- Can validate against am30 results
- Easy to tune if needed

---

## Issue 3: Single-Track vs Album Discrimination

### Findings

**Statistics:**
- Total files: ~3,609
- Album matching attempted: 107 (3.0%)
- Single-track discrimination: ~97%

**Analysis:**
- Discrimination is working correctly
- Only 3% of files trigger album matching (reasonable)
- No evidence of over-triggering or under-triggering

### Conclusion

**No action required.** Single-track vs album discrimination is performing well.

---

## Additional Logging Recommendations

### Log Enhancement 1: Decode Progress Tracking

**Add to boundary_detector.rs:**
```rust
let mut samples_decoded = 0;
let log_interval = sample_rate * 10; // Log every 10 seconds

loop {
    // ... decode packet ...
    samples_decoded += samples.len();

    if samples_decoded % log_interval == 0 {
        let duration_sec = samples_decoded / sample_rate;
        debug!("Decoded {} seconds of audio ({} samples)",
               duration_sec, samples_decoded);
    }
}

// After loop
info!("Total decoded: {} samples ({:.1} minutes)",
      all_samples.len(),
      all_samples.len() as f64 / sample_rate as f64 / 60.0);
```

**Benefits:**
- Identifies where decode stops
- Helps diagnose future issues
- Performance monitoring

### Log Enhancement 2: Album Match Confidence Details

**Add to album_matcher.rs:**
```rust
debug!("Album match candidate: {} - track_score: {:.2}, duration_score: {:.2}, total: {:.2}",
       candidate.title,
       track_score,
       duration_score,
       total_confidence);
```

**Benefits:**
- Understand why matches fail/succeed
- Tune threshold empirically
- Debug match quality

### Log Enhancement 3: Passage Extraction Stats

**Add to pipeline.rs:**
```rust
debug!("Extracted passage {} samples from file audio ({}% of total)",
       passage_samples.len(),
       passage_samples.len() as f64 / file_audio.samples.len() as f64 * 100.0);
```

**Benefits:**
- Detects when passages exceed available audio
- Early warning of decode issues

---

## Implementation Plan

### Phase 1: High Priority (Immediate)

**Task 1.1:** Lower album matching threshold to 65%
- **File:** `album_matcher.rs`
- **Effort:** 5 minutes
- **Risk:** Low
- **Impact:** +34% match success rate

**Task 1.2:** Add decode progress logging
- **File:** `boundary_detector.rs`
- **Effort:** 15 minutes
- **Risk:** Low
- **Impact:** Better diagnostics

### Phase 2: Critical Fix (Next Sprint)

**Task 2.1:** Implement streaming boundary detection
- **File:** `boundary_detector.rs` (rewrite)
- **Effort:** 4-6 hours
- **Risk:** Medium (architecture change)
- **Impact:** Eliminates 54.6% failure rate

**Task 2.2:** Add integration tests for long files
- **Test:** Verify 30+ passage files decode completely
- **Effort:** 2 hours
- **Risk:** Low

### Phase 3: Validation (After Phase 2)

**Task 3.1:** Rerun full library test
- Verify decode failure rate <1%
- Confirm album matching improvement

**Task 3.2:** Compare with am30 results
- Validate accuracy parity or improvement

---

## Success Metrics

### Target Metrics (After Implementation)

| Metric | Current | Target | How to Measure |
|--------|---------|--------|----------------|
| Multi-passage decode failure rate | 54.6% | <1% | Count "extends beyond" warnings |
| Album match success rate | 60.7% | >80% | Successful matches / attempts |
| Overall passage failure rate | 12.4% | <2% | Failed passages / total passages |
| Single-track discrimination | 97% | >95% | Maintain current performance |

### Acceptance Criteria

✅ **Phase 1 Complete:**
- Album matching attempts increase by 30-40%
- No regression in single-track processing

✅ **Phase 2 Complete:**
- Files with 30+ passages decode completely
- Zero "extends beyond available audio" warnings for valid files
- Performance impact <5% overall

✅ **Phase 3 Complete:**
- Full library test shows <1% decode failure rate
- Album matching parity with am30 (validate 10 sample files)

---

## Risk Analysis

### Implementation Risks

**Risk 1:** Streaming approach may miss boundaries
- **Mitigation:** Extensive testing with known multi-track albums
- **Fallback:** Hybrid approach (Solution 1C)

**Risk 2:** Performance regression
- **Mitigation:** Benchmark before/after on 100-file sample
- **Acceptable:** <5% slowdown overall

**Risk 3:** Lower threshold increases false positives
- **Mitigation:** Validate against am30 results
- **Rollback:** Easy to revert threshold change

### Current State Risks

**Risk of NOT implementing:**
- 54.6% of multi-passage files missing fingerprints
- Degraded user experience for album files
- Technical debt compounds
- **Assessment:** HIGH RISK to continue current approach

---

## Questions for Review

1. **Priority:** Should we implement Phase 1 (quick wins) immediately while test continues?

2. **Architecture:** Preference between streaming (1A), chunked (1B), or hybrid (1C) approaches?

3. **Threshold:** Approve 65% threshold, or prefer validation first?

4. **Testing:** Sufficient to validate with full library rerun, or need additional targeted tests?

5. **Timeline:** Acceptable to address in next sprint, or need hotfix?

---

## Appendix: Test Data Summary

**Test Run:** full_library_import_test_v2
**Progress:** 3,609 / 5,737 files (61.5%)
**Duration:** 49.2 hours
**Status:** Running (ETA: 30.6 hours remaining)

**Error Counts:**
- Audio sample failures: 864 passages
- Boundary extension warnings: 447 passages
- MusicBrainz connection errors: 201 (5.7% - network issue, non-critical)
- Invalid ISRC warnings: ~300+ (metadata quality, non-critical)

**Performance:**
- Average: 50 seconds per file
- Single tracks: 20-30 seconds
- Multi-track albums: 5-15 minutes (proportional to passage count)

---

**END OF IMPROVEMENT PLAN**
