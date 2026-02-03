# PLAN031 Scope Statement

## In Scope

1. **Ground Truth Management**
   - Database schema for ground truth storage
   - Loading ground truth from embedded MBID tags
   - Loading ground truth from JSON files
   - Ground truth validation and integrity checks

2. **Test Batch Orchestration**
   - BatchConfig struct and defaults
   - Batch execution with configurable size
   - Failure threshold stopping (Phase 1)
   - Accuracy threshold validation (Phase 2+)

3. **Evaluation Engine**
   - TP/FP/TN/FN classification for single songs
   - Album-level evaluation (ALBUM_MATCH, PARTIAL_MATCH, etc.)
   - Metrics calculation (accuracy, precision, recall, F1)
   - Confidence calibration tracking

4. **Failure Analysis**
   - FailureCategory enumeration
   - Pattern detection across failures
   - Root cause analysis structure
   - ImprovementSuggestion generation

5. **Database Reset**
   - Four reset modes (Full, TestDataOnly, BatchOnly, Incremental)
   - Safe ground truth preservation during resets

6. **Reporting**
   - Batch report generation (console output)
   - Progress report with accuracy trends
   - Structured JSON output for programmatic access

7. **Pipeline Integration**
   - Optional test evaluator hooks in existing services
   - Non-invasive instrumentation pattern

8. **CLI Interface**
   - `test-batch` command with options
   - `apply-improvement` command (basic)

## Out of Scope

1. **Automatic Code Generation** - Improvements are suggestions only; agent must implement
2. **Web UI for Test Management** - CLI-only for this phase
3. **Real-time Test Monitoring** - Batch results only, no streaming
4. **Multi-Machine Test Distribution** - Single-machine parallelization only
5. **Ground Truth Discovery from External Sources** - Must be provided, not discovered
6. **Statistical Significance Testing** - Basic metrics only, no A/B testing framework

## Assumptions

1. Test audio files exist in accessible location with ground truth available
2. Existing import pipeline (PLAN024, PLAN026) is functional
3. MusicBrainz/AcoustID APIs are available (rate-limited but accessible)
4. Database migrations can be extended for ground_truth table
5. Agent can read batch reports and modify code based on suggestions

## Constraints

1. **Non-Invasive Integration** - Must not break existing import pipeline when test mode disabled
2. **Rate Limiting** - Must respect MusicBrainz (1 req/s) and AcoustID (3 req/s) limits
3. **Database Compatibility** - Ground truth table must coexist with existing schema
4. **Performance** - Phase 1 batches should complete in <5 minutes for 10 files
