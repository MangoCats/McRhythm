# SPEC008 Usage Guide - Buffer Auto-Tuning

## Quick Start

**To create an implementation plan, run:**
```
/plan spec/SPEC008-buffer_autotuning.md
```

This will analyze the specification and create a detailed implementation plan with:
- Requirements breakdown
- Test specifications
- Traceability matrix
- Implementation guidance

## What This Specification Defines

**Problem:** Audio gaps persist despite manual tuning of buffer parameters.

**Solution:** Automated tool that:
1. Tests different combinations of `mixer_check_interval_ms` and `audio_buffer_size`
2. Measures buffer underruns for each combination
3. Finds the optimal balance (minimum latency + maximum stability)
4. Recommends safe values for the current system

## Key Features

### Two-Phase Search Strategy
- **Phase 1:** Quick sweep to find viable mixer intervals (6-8 test points)
- **Phase 2:** Binary search to find minimum stable buffer size for each interval

### Characterization Curve
The algorithm discovers the relationship between parameters:
```
mixer_check_interval_ms → Minimum stable audio_buffer_size

Example curve:
1ms  → UNSTABLE (even with large buffers)
2ms  → UNSTABLE
5ms  → 256 frames
10ms → 128 frames
20ms → 128 frames
50ms → 64 frames
```

### Actionable Output
- Recommended values with justification
- Expected latency calculation
- Confidence level
- JSON export for hardware profiles

## Requirements Summary

**Total:** 39 requirements across 8 categories

**Detection (3):** How to measure buffer underruns and audio health
**Search (4):** Parameter space exploration strategy
**Output (3):** Report generation and settings update
**Integration (3):** Standalone utility using existing infrastructure
**Algorithm (5):** Two-phase search with binary search refinement
**UI (2):** Command-line interface and progress feedback
**Architecture (3):** Standalone binary structure
**Safety (3):** Preserve settings, detect problems, validate results

## Implementation Size Estimate

**Approximate scope:**
- New binary: `wkmp-ap/src/bin/tune_buffers.rs` (~800 lines)
- Test harness: Simplified playback loop (~200 lines)
- Metrics collector: Underrun monitoring (~150 lines)
- Search algorithm: Binary search + curve fitting (~200 lines)
- Report generator: JSON + CLI output (~150 lines)
- Unit tests: Algorithm validation (~300 lines)
- Integration tests: E2E tuning runs (~100 lines)

**Total estimate:** ~1,900 lines (moderate complexity)

## Usage Examples

### Quick Tuning (5 minutes)
```bash
wkmp-ap tune-buffers --quick
```
Tests fewer points, faster results, good for initial setup.

### Thorough Tuning (15 minutes)
```bash
wkmp-ap tune-buffers --thorough --export profile.json
```
Comprehensive testing, saves results for future comparison.

### Automatic Application
```bash
wkmp-ap tune-buffers --apply
```
Automatically applies recommended values to database.

### Compare Across Hardware
```bash
# On development machine
wkmp-ap tune-buffers --export dev_machine.json

# On Raspberry Pi Zero 2W
wkmp-ap tune-buffers --export pi_zero_2w.json

# Compare results
wkmp-ap tune-buffers --compare dev_machine.json --export pi_zero_2w.json
```

## Expected Results

### Current System (Development)
Based on manual tuning, expected recommendations:
- **mixer_check_interval_ms:** 5-10ms
- **audio_buffer_size:** 128-256 frames
- **Expected latency:** ~3-6ms

### Raspberry Pi Zero 2W (Future)
Lower-powered system will likely need:
- **mixer_check_interval_ms:** 10-20ms (slower CPU)
- **audio_buffer_size:** 512-1024 frames (more safety margin)
- **Expected latency:** ~12-24ms

## Integration with WKMP

### Standalone Operation
- No HTTP server needed
- No queue management
- Minimal UI interaction
- Just: Database → Audio → Metrics → Results

### Uses Existing Code
- Ring buffer implementation (ring_buffer.rs)
- Audio output (output.rs)
- Database settings (settings.rs)
- Decoder/resampler for test audio

### Safe Operation
- Backs up current settings before starting
- Restores on abort (Ctrl+C, error, panic)
- All tested values within safe parameter ranges
- Sanity checks on results

## Success Criteria

The implementation will be considered successful when:

1. ✓ Completes tuning in <10 minutes (quick mode)
2. ✓ Recommended values produce <0.1% underruns
3. ✓ Results are reproducible (±10% variation)
4. ✓ Clear progress indication during operation
5. ✓ Exports valid JSON for comparison
6. ✓ Applies values correctly to database
7. ✓ At least 1 hour of stable playback with recommended values

## Open Questions for /plan

When running `/plan`, these questions should be addressed:

1. **Test audio source:** Use real passage from DB or generate test tone?
2. **Device selection:** Test default device only or allow selection?
3. **Recommendation strategy:** Conservative (more latency) or aggressive (less latency)?
4. **Integration point:** Part of setup wizard or on-demand only?

## Next Steps

1. Run `/plan spec/SPEC008-buffer_autotuning.md`
2. Review generated implementation plan
3. Resolve any CRITICAL specification issues
4. Implement according to plan with test-first approach
5. Validate on current hardware
6. Document results in project management/

## Related Documents

- **SPEC016:** Decoder buffer design (defines DBD-PARAM-110, DBD-PARAM-111)
- **PLAN001:** Error handling implementation (underrun detection patterns)
- **IMPL001:** Database schema (settings table structure)
- **GOV002:** Requirements enumeration (traceability format)
