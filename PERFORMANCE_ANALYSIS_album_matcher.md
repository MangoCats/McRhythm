# Album Matcher Performance Analysis
**Date:** 2026-01-11
**Test:** 200 album full baseline comparison with debug logging
**Log File:** `wkmp-ai/test_run29f_full_20260110_144201.log`

## Executive Summary

The 200-album test is running ~80x slower than expected (67 hours projected vs 40 minutes expected with cache). Analysis reveals significant variance in per-album processing time (4-60 minutes), with the primary bottleneck being **boundary refinement in Stage 2** for long audio files.

## Performance Data

### Album Processing Times (Sample)

| Album | Duration | Time | Issue |
|-------|----------|------|-------|
| Bon Jovi | ~40min | 4m 29s | ✅ Expected |
| Bears' Den | ~35min | 10m | ✅ Expected |
| Bjork - Debut | ~40min | 12m | ✅ Expected |
| Aerosmith - Pump | ~50min | 25m 10s | ⚠️ Slow |
| Allman Brothers - Fillmore | ~95min | 21m 32s | ⚠️ Slow |
| **Allman Brothers - Eat A Peach** | ~90min | **35m 46s** | 🔴 Very Slow |
| **Cars - Heartbeat City** | ~40min | **43m 42s** | 🔴 Very Slow |
| **Bjork - Body Talk** | ~70min | **51m 52s** | 🔴 Very Slow |
| **Cars - Panorama** | ~40min | **54m 26s** | 🔴 Very Slow |
| **Blackmore's Night** | ~50min | **58m 53s** | 🔴 Very Slow |

**Pattern:** Albums taking >20 minutes have high variance that doesn't correlate directly with audio duration.

## Root Cause Analysis

### Primary Bottleneck: Boundary Refinement in Stage 2

**Location:** `wkmp-ai/src/matching/stages/stage2.rs:207-213`

```rust
detected_durations = apply_refinement_to_durations(
    &detected_durations,
    &edition_durations_secs,
    audio_samples,
    sample_rate as f64,
    tolerance_secs,
);
```

**Issue:** This function is called for EVERY edition tested in Stage 2.

#### Example: Cars - Panorama (54 minutes total)
- Started testing editions: 02:17:24
- Started Stage 6: 03:07:07
- **Time in Stages 2-5: 50 minutes**
- Editions tested: 3
- Average per edition: ~17 minutes

### Secondary Factor: Audio File Length

Boundary refinement calls `find_local_quiet_spot()` which:
1. Slides a 0.5s RMS window through the search region
2. Search region is ±15 seconds around expected boundary
3. For albums with many tracks, this happens multiple times

**Cost per boundary search:**
- Window size: 22,050 samples (0.5s @ 44.1kHz)
- Search region: ~30 seconds = 1,323,000 samples
- Windows to process: ~60 (with 50% overlap)
- RMS calculations: ~60 per boundary

**For a 10-track album:**
- Potential boundaries to search: up to 10 (if all fail detection)
- Total RMS calculations: up to 600
- Time cost: Minimal for CPU, but adds up when audio decoding overhead is included

## Performance Opportunities

### 1. **Skip Boundary Refinement in Stage 2 for Perfect Matches** (HIGH IMPACT)

**Current Behavior:** Boundary refinement runs even when match percentage is 100%

**Optimization:**
```rust
// Only apply refinement if match is imperfect
if !detected_durations.is_empty() && best_percentage < 100.0 {
    detected_durations = apply_refinement_to_durations(
        &detected_durations,
        &edition_durations_secs,
        audio_samples,
        sample_rate as f64,
        tolerance_secs,
    );
}
```

**Expected Impact:**
- Albums with perfect matches skip refinement entirely
- Estimated 30-50% of albums could skip this step
- Saves ~5-10 minutes per album on average

### 2. **Early Exit After First Perfect Match in Stage 2** (MEDIUM IMPACT)

**Current Behavior:** Continues testing remaining editions even after 100% match

**Optimization:** Already has early exit logic, but only after grace period

**Enhancement:** For editions ranked by name similarity, if #1 edition gets 100% match, skip remaining editions immediately

**Expected Impact:**
- Saves testing 2+ additional editions when first edition is perfect
- Saves ~10-20 minutes for albums with obvious matches

### 3. **Limit Search Region Based on Track Count** (LOW-MEDIUM IMPACT)

**Current Behavior:** Always searches ±15 seconds around expected boundary

**Optimization:**
```rust
// Scale search window based on confidence
let window_samples = if abs_error_i < 60.0 {
    (10.0 * sample_rate) as usize  // ±5s for moderate errors
} else {
    (15.0 * sample_rate) as usize  // ±15s for severe errors
};
```

**Expected Impact:**
- Reduces RMS calculations by ~33% for moderate split failures
- Saves 1-3 minutes per album

### 4. **Cache Boundary Refinement Results** (MEDIUM IMPACT)

**Current Behavior:** Boundary refinement is recalculated for each edition, even if detected durations are identical

**Optimization:** Hash the detected durations and cache refinement results

**Expected Impact:**
- For albums with multiple similar editions, reuse refinement
- Saves 5-10 minutes for albums with 10+ editions tested

### 5. **Parallelize Edition Testing in Stage 2** (HIGH IMPACT, HIGH RISK)

**Current Behavior:** Tests editions sequentially

**Optimization:** Use `rayon` to test editions in parallel

**Risk:**
- Memory usage scales with parallelism (audio samples duplicated)
- May not help if I/O bound
- Requires careful synchronization

**Expected Impact:**
- Could reduce Stage 2 time by 2-3x on multi-core systems
- Risky: needs benchmarking to confirm benefit

## Recommended Implementation Order

1. ✅ **Skip refinement for perfect matches** (Quick win, low risk)
2. ✅ **Strict early exit after perfect match** (Quick win, low risk)
3. ⏸️ **Cache refinement results** (Medium effort, medium risk)
4. ⏸️ **Scale search window by error magnitude** (Medium effort, low risk)
5. ⏸️ **Parallelize edition testing** (High effort, high risk - needs benchmarking)

## Expected Overall Impact

With optimizations #1 and #2 implemented:
- **Best case:** 67 hours → 10-15 hours (4-6x speedup)
- **Average case:** 67 hours → 20-25 hours (2.5-3x speedup)
- **Worst case:** 67 hours → 35-40 hours (1.7x speedup)

## Test Status

**Current Progress:** 25/200 albums processed (12.5%)
**Elapsed Time:** ~8.5 hours
**Projected Completion:** ~67 hours total (Sunday 1/12 @ 14:00)

**Test continues running to completion for baseline data.**
