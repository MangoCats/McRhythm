# SPEC031: Closed-Loop Algorithm Improvement System

**Status:** Draft Specification
**Created:** 2025-12-13
**Purpose:** Define a feedback-driven evaluation system for iteratively improving MBID assignment accuracy

---

## 1. Executive Summary

This specification defines a closed-loop system where an AI agent can:
1. Execute audio file import attempts with MBID assignment
2. Evaluate success/failure against ground truth
3. Analyze failure patterns to identify algorithm weaknesses
4. Propose and implement improvements
5. Re-test to measure improvement impact

**Target:** Achieve >90% MBID assignment accuracy before optimizing for speed.

---

## 2. System Architecture

### 2.1 Feedback Loop Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    CLOSED-LOOP CYCLE                            │
│                                                                 │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐  │
│  │  IMPORT  │───▶│ EVALUATE │───▶│ ANALYZE  │───▶│ IMPROVE  │  │
│  │  BATCH   │    │ RESULTS  │    │ FAILURES │    │ ALGORITHM│  │
│  └──────────┘    └──────────┘    └──────────┘    └──────────┘  │
│       ▲                                               │         │
│       └───────────────────────────────────────────────┘         │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 Components

| Component | Purpose | Location |
|-----------|---------|----------|
| **TestOrchestrator** | Manages test batches and progression | `wkmp-ai/src/testing/` |
| **EvaluationEngine** | Compares results against ground truth | `wkmp-ai/src/testing/` |
| **FailureAnalyzer** | Identifies patterns in failures | `wkmp-ai/src/testing/` |
| **MetricsCollector** | Tracks accuracy, timing, improvement | `wkmp-ai/src/testing/` |
| **GroundTruthStore** | Manages known-correct MBID mappings | `wkmp-ai/src/testing/` |

---

## 3. Ground Truth Management

### 3.1 Ground Truth Sources (Priority Order)

1. **Embedded MBID Tags** (Highest reliability)
   - Files with `MUSICBRAINZ_TRACKID` ID3 tag
   - Considered authoritative if tag exists
   - Validation: Tag format matches UUID pattern

2. **Curated Test Sets**
   - Manually verified file-to-MBID mappings
   - Stored in `ground_truth.json` alongside test files
   - Format: `{ "file_hash": "sha256...", "mbid": "uuid", "verified_by": "method" }`

3. **Cross-Validation Agreement**
   - When 3+ identification methods agree on same MBID
   - Methods: AcoustID fingerprint, MusicBrainz metadata search, audio analysis
   - Confidence: High if all methods agree with >90% individual confidence

4. **Human Review Queue**
   - Ambiguous cases flagged for manual review
   - Human decisions become ground truth for future runs

### 3.2 Ground Truth Schema

```sql
CREATE TABLE IF NOT EXISTS ground_truth (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_hash TEXT NOT NULL UNIQUE,
    file_path TEXT NOT NULL,
    expected_mbid TEXT,  -- NULL if file should NOT match any recording
    expected_album_mbid TEXT,  -- For album-level tests
    verification_method TEXT NOT NULL,  -- 'embedded_tag', 'curated', 'cross_validation', 'human_review'
    verified_at TEXT NOT NULL,
    confidence REAL NOT NULL DEFAULT 1.0,
    notes TEXT
);

CREATE INDEX idx_ground_truth_hash ON ground_truth(file_hash);
```

---

## 4. Test Batch Management

### 4.1 Batch Progression Strategy

**Phase 1: Small Batches (Accuracy Focus)**
- Start with 5-10 files per batch
- Stop after 3 consecutive failures indicate pattern
- Analyze and improve before continuing
- Target: Identify algorithm weaknesses quickly

**Phase 2: Medium Batches (Validation)**
- 50-100 files per batch
- Continue until 90% accuracy achieved
- Track improvement trends across batches
- Target: Validate improvements at scale

**Phase 3: Large Batches (Speed Optimization)**
- 500+ files per batch
- Enable parallelization (configurable worker count)
- Measure throughput while maintaining accuracy
- Target: Optimize import speed without accuracy regression

### 4.2 Batch Configuration

```rust
pub struct BatchConfig {
    /// Number of files per batch
    pub batch_size: usize,

    /// Stop after N consecutive failures (Phase 1)
    pub failure_threshold: usize,

    /// Minimum accuracy to proceed (Phase 2+)
    pub accuracy_threshold: f64,

    /// Enable parallel processing (Phase 3)
    pub parallel_workers: usize,

    /// Maximum time per file before timeout
    pub timeout_secs: u64,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            batch_size: 10,
            failure_threshold: 3,
            accuracy_threshold: 0.90,
            parallel_workers: 1,
            timeout_secs: 60,
        }
    }
}
```

### 4.3 Progression Criteria

| Phase | Entry Criteria | Exit Criteria |
|-------|---------------|---------------|
| 1 (Small) | Start of testing | 3+ consecutive batches at >85% accuracy |
| 2 (Medium) | Phase 1 complete | 5+ batches at >90% accuracy |
| 3 (Large) | Phase 2 complete | Speed targets met OR diminishing returns |

---

## 5. Evaluation Framework

### 5.1 Success/Failure Classification

**For Single Songs:**

| Result | Condition | Action |
|--------|-----------|--------|
| **TRUE_POSITIVE** | Assigned MBID matches ground truth | Success |
| **FALSE_POSITIVE** | Assigned MBID differs from ground truth | Failure - wrong identification |
| **TRUE_NEGATIVE** | No MBID assigned, ground truth is NULL | Success - correctly rejected |
| **FALSE_NEGATIVE** | No MBID assigned, ground truth exists | Failure - missed identification |

**For Albums:**

| Result | Condition | Action |
|--------|-----------|--------|
| **ALBUM_MATCH** | Album MBID + all track MBIDs correct | Success |
| **PARTIAL_MATCH** | Album MBID correct, some tracks wrong | Partial success |
| **WRONG_ALBUM** | Album MBID incorrect | Failure |
| **NO_MATCH** | No album identification | Failure (if ground truth exists) |

### 5.2 Evaluation Metrics

```rust
pub struct EvaluationMetrics {
    // Accuracy metrics
    pub true_positives: usize,
    pub false_positives: usize,
    pub true_negatives: usize,
    pub false_negatives: usize,

    // Derived metrics
    pub accuracy: f64,      // (TP + TN) / Total
    pub precision: f64,     // TP / (TP + FP)
    pub recall: f64,        // TP / (TP + FN)
    pub f1_score: f64,      // 2 * (precision * recall) / (precision + recall)

    // Confidence calibration
    pub avg_confidence_correct: f64,
    pub avg_confidence_incorrect: f64,
    pub confidence_correlation: f64,  // Higher confidence should correlate with correctness

    // Timing metrics
    pub avg_time_per_file_ms: u64,
    pub total_batch_time_ms: u64,
    pub api_calls_per_file: f64,
}
```

### 5.3 Failure Categories

```rust
pub enum FailureCategory {
    // Identification failures
    WrongRecording,           // Got an MBID but it's wrong
    WrongAlbum,               // Correct song, wrong album release
    WrongArtist,              // Similar song name, different artist
    MissedIdentification,     // Should have found MBID but didn't

    // Confidence failures
    OverconfidentWrong,       // High confidence but wrong
    UnderconfidentRight,      // Low confidence but actually correct

    // Data source failures
    AcoustIdMismatch,         // AcoustID returned wrong result
    MetadataSearchFailed,     // MusicBrainz search returned no/wrong results
    ID3TagsUnreliable,        // ID3 metadata was misleading

    // Edge cases
    MultipleValidMatches,     // Multiple correct MBIDs exist (remasters, etc.)
    NoGroundTruth,            // Can't verify correctness
    Timeout,                  // Processing took too long
    ApiError,                 // External service failure
}
```

---

## 6. Failure Analysis

### 6.1 Pattern Detection

The FailureAnalyzer identifies recurring patterns:

```rust
pub struct FailurePattern {
    pub category: FailureCategory,
    pub occurrence_count: usize,
    pub affected_files: Vec<String>,
    pub common_characteristics: Vec<String>,
    pub suggested_improvements: Vec<String>,
}
```

**Example Patterns:**
- "60% of failures are classical music with variant spellings"
- "AcoustID consistently wrong for live recordings"
- "Short tracks (<60s) often misidentified"

### 6.2 Root Cause Analysis

For each failure pattern, analyze:

1. **Data Source Analysis**
   - Which source(s) contributed to wrong answer?
   - What confidence did each source report?
   - Was the correct answer available from any source?

2. **Algorithm Analysis**
   - Did fusion combine sources correctly?
   - Were thresholds appropriate?
   - Did normalization help or hurt?

3. **Metadata Analysis**
   - What metadata characteristics led to failure?
   - Genre, duration, artist name complexity?
   - Multiple releases/versions involved?

### 6.3 Improvement Suggestions

Based on failure patterns, generate actionable suggestions:

```rust
pub enum ImprovementSuggestion {
    AdjustThreshold {
        parameter: String,
        current: f64,
        suggested: f64,
        rationale: String,
    },
    AddNormalization {
        field: String,
        pattern: String,
        replacement: String,
    },
    WeightAdjustment {
        source: String,
        context: String,  // e.g., "for classical music"
        current_weight: f64,
        suggested_weight: f64,
    },
    NewHeuristic {
        description: String,
        pseudocode: String,
    },
    SkipStrategy {
        condition: String,
        reason: String,
    },
}
```

---

## 7. Database Reset Strategy

### 7.1 Reset Modes

```rust
pub enum ResetMode {
    /// Clear entire database, fresh start
    Full,

    /// Clear only test-related data, keep ground truth
    TestDataOnly,

    /// Clear specific batch results
    BatchOnly { batch_id: String },

    /// No reset, build on existing data
    Incremental,
}
```

### 7.2 Reset Implementation

```rust
pub async fn reset_for_testing(pool: &SqlitePool, mode: ResetMode) -> Result<()> {
    match mode {
        ResetMode::Full => {
            // Backup ground_truth table
            // Drop and recreate all tables via migrations
            // Restore ground_truth
        }
        ResetMode::TestDataOnly => {
            // DELETE FROM passages WHERE source = 'test_import'
            // DELETE FROM files WHERE import_session LIKE 'test_%'
            // Keep ground_truth, settings, etc.
        }
        ResetMode::BatchOnly { batch_id } => {
            // DELETE FROM passages WHERE batch_id = ?
            // DELETE FROM files WHERE batch_id = ?
        }
        ResetMode::Incremental => {
            // No reset, continue with existing data
        }
    }
    Ok(())
}
```

---

## 8. Reporting and Metrics

### 8.1 Batch Report

After each batch:

```
═══════════════════════════════════════════════════════════════
BATCH REPORT: batch_2025_12_13_001
═══════════════════════════════════════════════════════════════
Files Processed:    50
Duration:           3m 24s (4.08s/file avg)

ACCURACY METRICS:
  True Positives:   42 (84%)
  False Positives:   3 (6%)
  True Negatives:    2 (4%)
  False Negatives:   3 (6%)

  Overall Accuracy: 88.0%
  Precision:        93.3%
  Recall:           93.3%
  F1 Score:         93.3%

CONFIDENCE CALIBRATION:
  Avg confidence (correct):   0.91
  Avg confidence (incorrect): 0.72
  Calibration:                GOOD (high conf = high accuracy)

FAILURE ANALYSIS:
  Wrong Recording:            2 (classical music variants)
  Missed Identification:      3 (live recordings)
  AcoustID Mismatch:          1

SUGGESTED IMPROVEMENTS:
  1. Increase duration tolerance for live recordings (±15s → ±30s)
  2. Add "live" keyword detection to reduce AcoustID weight
  3. Consider opus number matching for classical

PROGRESSION STATUS:
  Current Phase:    2 (Medium Batches)
  Accuracy Trend:   ↑ (+2.1% from last batch)
  Phase 3 Ready:    NO (need 90%+ for 3 more batches)
═══════════════════════════════════════════════════════════════
```

### 8.2 Cumulative Progress Report

Track improvement across batches:

```rust
pub struct ProgressReport {
    pub batches_completed: usize,
    pub total_files_processed: usize,
    pub accuracy_history: Vec<f64>,
    pub accuracy_trend: f64,  // Linear regression slope
    pub current_phase: Phase,
    pub improvements_applied: Vec<String>,
    pub estimated_batches_to_target: Option<usize>,
}
```

---

## 9. API Integration Points

### 9.1 Test Orchestrator Commands

```rust
impl TestOrchestrator {
    /// Start a new test run with specified configuration
    pub async fn start_run(&self, config: BatchConfig) -> Result<RunId>;

    /// Process next batch in current run
    pub async fn process_batch(&self, run_id: &RunId) -> Result<BatchReport>;

    /// Get current run status
    pub async fn get_status(&self, run_id: &RunId) -> Result<RunStatus>;

    /// Apply suggested improvement
    pub async fn apply_improvement(&self, suggestion: &ImprovementSuggestion) -> Result<()>;

    /// Reset and restart with new parameters
    pub async fn reset_and_restart(&self, mode: ResetMode, config: BatchConfig) -> Result<RunId>;

    /// Generate comprehensive report
    pub async fn generate_report(&self, run_id: &RunId) -> Result<ProgressReport>;
}
```

### 9.2 Integration with Existing Pipeline

Feedback points in existing code:

1. **Post-Fusion Hook** (`identity_resolver.rs:resolve()`)
   ```rust
   // After fusion, before return:
   if let Some(evaluator) = &self.test_evaluator {
       evaluator.record_fusion_result(&file_hash, &fused_identity).await;
   }
   ```

2. **Post-Validation Hook** (`content_type_classifier.rs`)
   ```rust
   // After classification:
   if let Some(evaluator) = &self.test_evaluator {
       evaluator.record_classification(&file_hash, &result).await;
   }
   ```

3. **Session Completion Hook** (`workflow_orchestrator.rs`)
   ```rust
   // After session completes:
   if let Some(evaluator) = &self.test_evaluator {
       evaluator.finalize_batch(&session_id).await?;
   }
   ```

---

## 10. Implementation Increments

| # | Description | Effort | Dependencies |
|---|-------------|--------|--------------|
| 1 | Ground Truth Schema & Management | 2-3h | Database migrations |
| 2 | Evaluation Engine (TP/FP/TN/FN) | 3-4h | Ground truth store |
| 3 | Batch Configuration & Orchestration | 3-4h | Evaluation engine |
| 4 | Failure Analyzer & Categorization | 4-5h | Evaluation engine |
| 5 | Reporting & Metrics Collection | 2-3h | All above |
| 6 | Pipeline Integration Hooks | 2-3h | Existing services |
| 7 | CLI Interface for Test Runs | 2-3h | All above |

**Total Estimated Effort:** 18-25 hours

---

## 11. Success Criteria

### 11.1 System Success

- [ ] Can execute test batches against ground truth
- [ ] Accurately classifies results as TP/FP/TN/FN
- [ ] Identifies failure patterns automatically
- [ ] Generates actionable improvement suggestions
- [ ] Tracks accuracy trends across batches
- [ ] Supports all three progression phases

### 11.2 Accuracy Target

- [ ] Phase 1: Identify weaknesses in <5 batches
- [ ] Phase 2: Achieve 90%+ accuracy within 20 batches
- [ ] Phase 3: Maintain 90%+ accuracy with 4x parallelization

### 11.3 Agent Workflow Success

The AI agent should be able to:
1. Run `cargo run -p wkmp-ai -- test-batch --size 10`
2. Read the batch report output
3. Analyze failure patterns
4. Modify algorithm parameters based on suggestions
5. Re-run and measure improvement
6. Iterate until accuracy target met

---

## 12. Requirements Traceability

| Requirement ID | Description | Section |
|---------------|-------------|---------|
| SPEC031-GT-010 | Ground truth management | §3 |
| SPEC031-GT-020 | Ground truth schema | §3.2 |
| SPEC031-BM-010 | Batch management | §4 |
| SPEC031-BM-020 | Progression phases | §4.1 |
| SPEC031-EV-010 | Evaluation framework | §5 |
| SPEC031-EV-020 | Success/failure classification | §5.1 |
| SPEC031-EV-030 | Evaluation metrics | §5.2 |
| SPEC031-FA-010 | Failure analysis | §6 |
| SPEC031-FA-020 | Pattern detection | §6.1 |
| SPEC031-FA-030 | Improvement suggestions | §6.3 |
| SPEC031-DB-010 | Database reset strategy | §7 |
| SPEC031-RP-010 | Batch reporting | §8.1 |
| SPEC031-RP-020 | Progress tracking | §8.2 |
| SPEC031-API-010 | API integration | §9 |

---

## 13. Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Insufficient ground truth data | Medium | High | Start with embedded MBID tags; build curated set incrementally |
| Overfitting to test set | Medium | Medium | Hold out validation set; test against new files periodically |
| API rate limiting during tests | High | Medium | Use cached results; implement backoff; batch API calls |
| False improvements | Low | High | Require improvement on held-out set; statistical significance tests |
| Slow iteration cycles | Medium | Medium | Parallel evaluation; incremental testing |

---

## Appendix A: Example Test Session

```bash
# Phase 1: Initial small batch testing
$ cargo run -p wkmp-ai -- test-batch --phase 1 --size 10

Starting test batch (Phase 1: Small Batches)
Files: 10 | Failure threshold: 3 | Workers: 1

Processing: track_01.mp3... ✓ (0.94 confidence)
Processing: track_02.mp3... ✓ (0.91 confidence)
Processing: track_03.mp3... ✗ WRONG_RECORDING
Processing: track_04.mp3... ✓ (0.88 confidence)
Processing: track_05.mp3... ✗ MISSED_IDENTIFICATION
Processing: track_06.mp3... ✗ WRONG_RECORDING

⚠ Failure threshold reached (3 failures). Stopping batch.

Accuracy: 40% (4/10)
Failures:
  - WRONG_RECORDING: 2 (both classical piano)
  - MISSED_IDENTIFICATION: 1 (live recording)

Suggested improvements:
  1. Add composer name matching for classical
  2. Increase duration tolerance for live recordings

# Apply improvement and retry
$ cargo run -p wkmp-ai -- apply-improvement --id 1
$ cargo run -p wkmp-ai -- test-batch --phase 1 --size 10 --incremental

Processing batch with improvements applied...
Accuracy: 70% (7/10) [+30% improvement]
```

---

## Appendix B: Ground Truth File Format

```json
{
  "version": "1.0",
  "created": "2025-12-13",
  "entries": [
    {
      "file_path": "test_files/classical/beethoven_moonlight.mp3",
      "file_hash": "sha256:abc123...",
      "expected_mbid": "12345678-1234-1234-1234-123456789012",
      "verification": "embedded_tag",
      "confidence": 1.0,
      "metadata": {
        "artist": "Ludwig van Beethoven",
        "title": "Piano Sonata No. 14 - I. Adagio sostenuto",
        "album": "Complete Piano Sonatas"
      }
    },
    {
      "file_path": "test_files/pop/beatles_yesterday.mp3",
      "file_hash": "sha256:def456...",
      "expected_mbid": "87654321-4321-4321-4321-210987654321",
      "verification": "curated",
      "confidence": 1.0,
      "metadata": {
        "artist": "The Beatles",
        "title": "Yesterday",
        "album": "Help!"
      }
    }
  ]
}
```
