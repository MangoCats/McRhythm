-- Migration 007: Ground Truth Table
-- PLAN031: Closed-loop algorithm improvement testing system
--
-- Stores known-correct MBID mappings for evaluating import accuracy.
-- Ground truth data can come from:
--   1. Embedded MBID tags in ID3 metadata
--   2. Curated JSON test files
--   3. Cross-validation agreement between multiple sources
--   4. Human review

CREATE TABLE IF NOT EXISTS ground_truth (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- File identification
    file_hash TEXT NOT NULL UNIQUE,
    file_path TEXT NOT NULL,

    -- Expected results
    expected_mbid TEXT,           -- Recording MBID (NULL if file should NOT match)
    expected_album_mbid TEXT,     -- Album MBID for album-level tests

    -- Verification metadata
    verification_method TEXT NOT NULL CHECK (
        verification_method IN ('embedded_tag', 'curated', 'cross_validation', 'human_review')
    ),
    verified_at TEXT NOT NULL DEFAULT (datetime('now')),
    confidence REAL NOT NULL DEFAULT 1.0 CHECK (confidence >= 0.0 AND confidence <= 1.0),

    -- Additional metadata
    notes TEXT,

    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Index for fast lookups by file hash
CREATE INDEX IF NOT EXISTS idx_ground_truth_hash ON ground_truth(file_hash);

-- Index for finding entries by verification method
CREATE INDEX IF NOT EXISTS idx_ground_truth_method ON ground_truth(verification_method);

-- Trigger to update updated_at timestamp
CREATE TRIGGER IF NOT EXISTS ground_truth_updated_at
    AFTER UPDATE ON ground_truth
    FOR EACH ROW
BEGIN
    UPDATE ground_truth SET updated_at = datetime('now') WHERE id = NEW.id;
END;

-- Test runs table - tracks batch test executions
CREATE TABLE IF NOT EXISTS test_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL UNIQUE,
    phase TEXT NOT NULL CHECK (phase IN ('phase1', 'phase2', 'phase3')),
    config_json TEXT NOT NULL,        -- Serialized BatchConfig
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    status TEXT NOT NULL DEFAULT 'running' CHECK (
        status IN ('running', 'completed', 'failed', 'cancelled')
    ),
    stop_reason TEXT,
    notes TEXT
);

CREATE INDEX IF NOT EXISTS idx_test_runs_status ON test_runs(status);

-- Test results table - stores individual file results within a test run
CREATE TABLE IF NOT EXISTS test_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL REFERENCES test_runs(run_id) ON DELETE CASCADE,
    ground_truth_id INTEGER REFERENCES ground_truth(id) ON DELETE SET NULL,
    file_hash TEXT NOT NULL,
    file_path TEXT NOT NULL,

    -- Classification result
    classification TEXT NOT NULL CHECK (
        classification IN ('true_positive', 'false_positive', 'true_negative', 'false_negative')
    ),

    -- Actual results from import
    assigned_mbid TEXT,
    assigned_confidence REAL,

    -- Failure analysis
    failure_category TEXT,
    failure_details TEXT,

    -- Performance metrics
    processing_time_ms INTEGER,
    api_calls INTEGER DEFAULT 0,

    -- Timestamps
    tested_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_test_results_run ON test_results(run_id);
CREATE INDEX IF NOT EXISTS idx_test_results_classification ON test_results(classification);
