# Scope Statement - Buffer Auto-Tuning

## In Scope

### Core Functionality
- ✓ Standalone utility (`wkmp-ap tune-buffers`) for automatic buffer parameter tuning
- ✓ Two-phase search algorithm (coarse sweep + binary search refinement)
- ✓ Detection and measurement of ring buffer underruns during test playback
- ✓ Systematic testing of mixer_check_interval_ms (1-100ms range)
- ✓ Systematic testing of audio_buffer_size (64-8192 frames range)
- ✓ Characterization curve mapping interval → minimum stable buffer size
- ✓ Recommendation generation (primary + conservative options)
- ✓ JSON export for hardware profile comparison
- ✓ Database settings update with user confirmation
- ✓ Command-line interface with multiple modes (--quick, --thorough, --apply, --export)
- ✓ Progress indication and real-time feedback during tuning

### Test Execution
- ✓ 30-60 second playback tests per parameter combination
- ✓ Actual audio content playback (music passage or test tone)
- ✓ Full decode→resample→mix→output pipeline exercised
- ✓ Underrun rate calculation and classification (fail/warning/success)
- ✓ Callback timing statistics collection (jitter, regularity)
- ✓ Buffer occupancy monitoring (min/max/mean/percentiles)
- ✓ CPU usage measurement (if available)

### Safety and Reliability
- ✓ Settings backup before tuning begins
- ✓ Settings restore on abort (Ctrl+C, panic, error)
- ✓ Parameter range validation (enforce minimum/maximum values)
- ✓ System responsiveness detection (abort if hangs >60s)
- ✓ Sanity checks on results (detect anomalies)

### Code Reuse
- ✓ Ring buffer implementation from wkmp-ap/src/playback/ring_buffer.rs
- ✓ Audio output from wkmp-ap/src/audio/output.rs
- ✓ Database settings from wkmp-ap/src/db/settings.rs
- ✓ Decoder/resampler for test audio generation

### Testing and Validation
- ✓ Unit tests for search algorithm (mocked test results)
- ✓ Unit tests for curve fitting logic
- ✓ Unit tests for recommendation generation
- ✓ Unit tests for JSON export/import
- ✓ Integration tests running on CI
- ✓ Manual validation on development hardware (1+ hour stability test)

## Out of Scope

### Not Implemented in Initial Version
- ✗ Multi-track testing (simultaneous audio sources)
- ✗ Crossfade scenario simulation
- ✗ Hardware fingerprinting and automatic profile matching
- ✗ Continuous monitoring of parameter effectiveness
- ✗ Automatic re-tuning triggers
- ✗ Web UI for tuning control
- ✗ Remote tuning via SSE
- ✗ Platform-specific optimizations (Windows WASAPI, macOS CoreAudio)
- ✗ Multiple audio device testing (test default device only)
- ✗ Automatic tuning on first run (on-demand only)
- ✗ Integration with startup sequence

### Deferred to Future Enhancements
- Future: Multi-track and crossfade testing (TUNE-FUT-010)
- Future: Hardware fingerprinting (TUNE-FUT-020)
- Future: Continuous monitoring (TUNE-FUT-030)
- Future: Remote tuning (TUNE-FUT-040)
- Future: Platform-specific tuning (TUNE-PLAT-010, TUNE-PLAT-020)

### Not Part of This Feature
- ✗ Changes to existing playback engine
- ✗ Changes to ring buffer implementation (reuse as-is)
- ✗ Changes to mixer thread logic
- ✗ New audio output backends
- ✗ Database schema changes (use existing settings table)

## Assumptions

### System and Environment
1. **Database available:** SQLite database exists and is accessible
2. **Audio device available:** At least one audio output device present
3. **Test audio available:** Either music passage in DB or can generate test tone
4. **Full pipeline functional:** Existing decode→resample→mix→output works correctly
5. **Ring buffer monitoring works:** Underrun detection is accurate
6. **Settings table exists:** Database has settings table with mixer_check_interval_ms and audio_buffer_size

### User Environment
7. **User can run standalone utility:** Permission to execute binary
8. **User can hear audio:** Or has monitoring tools to detect playback
9. **System has moderate load:** Not running intensive background tasks during tuning
10. **User available to supervise:** For manual validation tests

### Technical
11. **Parameter ranges valid:** 1-100ms interval, 64-8192 frames buffer are sufficient
12. **30-60s test duration adequate:** Long enough to detect underruns reliably
13. **<0.1% underrun threshold appropriate:** Defines "stable" correctly
14. **Binary search converges:** Algorithm will find stability boundary
15. **Results reproducible:** Same hardware produces consistent results (±10% variation)

### Dependencies
16. **wkmp-ap compiles:** Existing codebase builds successfully
17. **All dependencies available:** symphonia, rubato, cpal, tokio, sqlx, serde_json
18. **No breaking changes:** Dependencies don't introduce API changes during development

## Constraints

### Time Constraints
- **Tuning duration:** Complete in <15 minutes (thorough mode), <5 minutes (quick mode)
- **Test duration:** 30-60 seconds per configuration (adjustable)
- **Cooldown period:** 2 seconds between tests (system recovery)

### Technical Constraints
- **Parameter ranges:** Must stay within 1-100ms interval, 64-8192 frames buffer
- **Memory:** Limited by system RAM (ring buffer allocations)
- **CPU:** Must not consume 100% CPU (leave headroom for scheduler)
- **Audio device:** Single device testing only (default from database)

### Platform Constraints
- **Primary target:** Linux (ALSA)
- **Secondary target:** Raspberry Pi Zero 2W (future validation)
- **Audio API:** cpal-supported platforms only
- **Database:** SQLite only (no PostgreSQL/MySQL)

### Quality Constraints
- **Reproducibility:** Results must be consistent (±10% variation)
- **Accuracy:** No false positives/negatives in stability classification
- **Safety:** Must restore settings on any failure/abort
- **Usability:** Clear progress indication and error messages

### Resource Constraints
- **Development time:** Target 40-80 hours implementation
- **Test coverage:** 100% requirement traceability, comprehensive unit/integration tests
- **Documentation:** Inline code documentation, user guide in CLI help

### Dependency Constraints
- **No new external dependencies:** Use existing Cargo dependencies only
- **Rust stable:** No nightly features required
- **wkmp-ap only:** No dependencies on wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le

## Success Metrics

### Functional Success
- ✓ Completes tuning run in target time (<5 min quick, <15 min thorough)
- ✓ Identifies stable parameter combinations correctly
- ✓ Generates actionable recommendations
- ✓ Exports valid JSON for hardware comparison
- ✓ Applies values correctly to database when requested

### Quality Success
- ✓ Recommended values produce <0.1% underruns
- ✓ No false positives (stable marked as unstable)
- ✓ No false negatives (unstable marked as stable)
- ✓ Results reproducible (±10% variation)
- ✓ At least 1 hour stable playback with recommended values

### Usability Success
- ✓ Clear progress indication during operation
- ✓ Understandable recommendations with rationale
- ✓ Easy to apply results (single confirmation)
- ✓ Useful error messages for common problems
- ✓ Help text documents all options

## Boundary Conditions

### Minimum System Requirements
- CPU: Sufficient for real-time audio decode/resample/mix/output
- RAM: Enough for 8192-frame buffers + OS overhead
- Audio: At least one working output device
- Disk: Space for database and test audio

### Maximum Supported
- Test duration: Up to 300 seconds (5 minutes) per configuration
- Buffer size: Up to 8192 frames (~185ms @ 44.1kHz)
- Mixer interval: Up to 100ms
- Number of test points: Up to 50 configurations (thorough mode)

### Edge Cases
- No audio device: Exit gracefully with error message
- Database locked: Wait briefly, then fail with clear message
- Test hangs: Abort after 60 seconds, report timeout
- All configurations fail: Report system may be inadequate
- Results anomalous: Warn user, export data for analysis

## Dependencies

### Internal (WKMP)
- **wkmp-ap source:** Existing audio playback infrastructure
  - Status: ✓ Exists, stable
- **Ring buffer:** wkmp-ap/src/playback/ring_buffer.rs
  - Status: ✓ Exists, with underrun monitoring
- **Audio output:** wkmp-ap/src/audio/output.rs
  - Status: ✓ Exists, supports cpal
- **Database settings:** wkmp-ap/src/db/settings.rs
  - Status: ✓ Exists, load_clamped_setting() available
- **Decoder:** wkmp-ap/src/audio/decoder.rs
  - Status: ✓ Exists, symphonia-based
- **Resampler:** wkmp-ap/src/audio/resampler.rs
  - Status: ✓ Exists, rubato-based

### External (Cargo Dependencies)
- **tokio:** Async runtime
  - Status: ✓ Already in Cargo.toml
- **sqlx:** Database access
  - Status: ✓ Already in Cargo.toml
- **serde_json:** JSON export
  - Status: ✓ Already in Cargo.toml
- **cpal:** Audio output
  - Status: ✓ Already in Cargo.toml
- **symphonia:** Audio decoding
  - Status: ✓ Already in Cargo.toml
- **rubato:** Resampling
  - Status: ✓ Already in Cargo.toml

### Hardware
- **Audio device:** System must have working audio output
  - Status: ⚠ User responsibility
- **CPU:** Adequate for real-time audio processing
  - Status: ⚠ Validated during tuning

### Documentation
- **SPEC016:** Decoder buffer design (parameter definitions)
  - Status: ✓ Exists, defines DBD-PARAM-110, DBD-PARAM-111
- **IMPL001:** Database schema (settings table)
  - Status: ✓ Exists, documented

## Risk Summary

### High Risk (Must Mitigate)
- **False negatives:** Marking unstable as stable → Users experience underruns
  - Mitigation: Conservative thresholds (<0.1%), 6-sigma recommendation logic
- **System hang:** Test causes system unresponsiveness
  - Mitigation: 60-second timeout per test, abort mechanism
- **Settings not restored:** Abort leaves bad values in database
  - Mitigation: Robust backup/restore, signal handlers

### Medium Risk (Monitor)
- **Test duration too short:** 30s insufficient to detect intermittent issues
  - Mitigation: Allow user to extend test duration, default to 60s in thorough mode
- **Results not reproducible:** Hardware variability causes inconsistent results
  - Mitigation: Accept ±10% variation, run validation tests multiple times
- **Anomalous results:** Recommendations make no sense
  - Mitigation: Sanity checks, warn user, export data for analysis

### Low Risk (Accept)
- **Edge case parameter combinations:** Some combinations untested
  - Accept: Binary search covers most of parameter space efficiently
- **Platform differences:** Behavior varies across Linux distributions
  - Accept: Primary target is development hardware, document known issues
