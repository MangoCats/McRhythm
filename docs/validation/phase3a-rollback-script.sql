-- ============================================================================
-- WKMP Database Migration ROLLBACK: INTEGER Ticks → REAL Seconds
-- ============================================================================
-- Rollback File: 2025-10-19_tick_timing_rollback.sql
-- Created: 2025-10-19
-- Authority: T1-TIMING-001 (Tier 1 APPROVED)
-- Reference: SPEC017-sample_accurate_timing.md
--
-- PURPOSE:
--   Reverse the tick-based timing migration, converting INTEGER ticks back
--   to REAL seconds. This rollback should only be used if critical issues
--   are discovered post-migration.
--
-- WARNING:
--   This rollback involves precision loss! The original REAL values stored
--   seconds with ~15 significant digits, while tick-based storage has
--   quantization to 1/28,224,000 second intervals (~0.035 microseconds).
--   Round-tripping will introduce minor rounding differences.
--
-- TICK RATE: 28,224,000 Hz (SRC-TICK-020)
--
-- FIELDS AFFECTED:
--   start_time INTEGER NOT NULL      → start_time REAL
--   fade_in_start INTEGER            → fade_in_start REAL
--   lead_in_start INTEGER            → lead_in_start REAL
--   lead_out_start INTEGER           → lead_out_start REAL
--   fade_out_start INTEGER           → fade_out_start REAL
--   end_time INTEGER NOT NULL        → end_time REAL
--
-- CONVERSION FORMULA:
--   seconds = ticks / 28224000.0
--
-- ROLLBACK STRATEGY:
--   1. Add new REAL columns with temporary _seconds suffix
--   2. Convert tick values to seconds with NULL preservation
--   3. Validate conversion
--   4. Recreate table with REAL columns
--   5. Copy data and drop temporary columns
--
-- SAFETY:
--   - Wrapped in transaction (auto-rollback on error)
--   - Multiple validation checkpoints
--   - NULL values preserved
--   - Data integrity constraints enforced
-- ============================================================================

BEGIN TRANSACTION;

-- ============================================================================
-- STEP 1: Pre-Rollback Validation
-- ============================================================================

-- Verify passages table exists
SELECT CASE
    WHEN (SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='passages') = 0
    THEN RAISE(ABORT, 'PRE-CHECK FAILED: passages table does not exist')
END;

-- Verify current schema uses INTEGER types
SELECT CASE
    WHEN (SELECT COUNT(*)
          FROM pragma_table_info('passages')
          WHERE name = 'start_time' AND type = 'INTEGER') = 0
    THEN RAISE(ABORT, 'PRE-CHECK FAILED: start_time is not INTEGER type (migration not applied?)')
END;

SELECT CASE
    WHEN (SELECT COUNT(*)
          FROM pragma_table_info('passages')
          WHERE name = 'end_time' AND type = 'INTEGER') = 0
    THEN RAISE(ABORT, 'PRE-CHECK FAILED: end_time is not INTEGER type (migration not applied?)')
END;

-- Store initial row count for verification
CREATE TEMP TABLE rollback_validation AS
SELECT
    COUNT(*) AS total_rows,
    COUNT(start_time) AS start_time_non_null,
    COUNT(fade_in_start) AS fade_in_start_non_null,
    COUNT(lead_in_start) AS lead_in_start_non_null,
    COUNT(lead_out_start) AS lead_out_start_non_null,
    COUNT(fade_out_start) AS fade_out_start_non_null,
    COUNT(end_time) AS end_time_non_null
FROM passages;

-- ============================================================================
-- STEP 2: Add New REAL Columns (Temporary _seconds Suffix)
-- ============================================================================

ALTER TABLE passages ADD COLUMN start_time_seconds REAL;
ALTER TABLE passages ADD COLUMN fade_in_start_seconds REAL;
ALTER TABLE passages ADD COLUMN lead_in_start_seconds REAL;
ALTER TABLE passages ADD COLUMN lead_out_start_seconds REAL;
ALTER TABLE passages ADD COLUMN fade_out_start_seconds REAL;
ALTER TABLE passages ADD COLUMN end_time_seconds REAL;

-- ============================================================================
-- STEP 3: Migrate Data (INTEGER Ticks → REAL Seconds)
-- ============================================================================

UPDATE passages
SET
    start_time_seconds = CASE
        WHEN start_time IS NOT NULL
        THEN CAST(start_time AS REAL) / 28224000.0
        ELSE NULL
    END,
    fade_in_start_seconds = CASE
        WHEN fade_in_start IS NOT NULL
        THEN CAST(fade_in_start AS REAL) / 28224000.0
        ELSE NULL
    END,
    lead_in_start_seconds = CASE
        WHEN lead_in_start IS NOT NULL
        THEN CAST(lead_in_start AS REAL) / 28224000.0
        ELSE NULL
    END,
    lead_out_start_seconds = CASE
        WHEN lead_out_start IS NOT NULL
        THEN CAST(lead_out_start AS REAL) / 28224000.0
        ELSE NULL
    END,
    fade_out_start_seconds = CASE
        WHEN fade_out_start IS NOT NULL
        THEN CAST(fade_out_start AS REAL) / 28224000.0
        ELSE NULL
    END,
    end_time_seconds = CASE
        WHEN end_time IS NOT NULL
        THEN CAST(end_time AS REAL) / 28224000.0
        ELSE NULL
    END;

-- ============================================================================
-- STEP 4: Validate Conversion
-- ============================================================================

-- Verify NULL counts unchanged
SELECT CASE
    WHEN (SELECT COUNT(*) FROM passages WHERE start_time IS NULL) !=
         (SELECT COUNT(*) FROM passages WHERE start_time_seconds IS NULL)
    THEN RAISE(ABORT, 'VALIDATION FAILED: start_time NULL count mismatch')
END;

SELECT CASE
    WHEN (SELECT COUNT(*) FROM passages WHERE fade_in_start IS NULL) !=
         (SELECT COUNT(*) FROM passages WHERE fade_in_start_seconds IS NULL)
    THEN RAISE(ABORT, 'VALIDATION FAILED: fade_in_start NULL count mismatch')
END;

SELECT CASE
    WHEN (SELECT COUNT(*) FROM passages WHERE lead_in_start IS NULL) !=
         (SELECT COUNT(*) FROM passages WHERE lead_in_start_seconds IS NULL)
    THEN RAISE(ABORT, 'VALIDATION FAILED: lead_in_start NULL count mismatch')
END;

SELECT CASE
    WHEN (SELECT COUNT(*) FROM passages WHERE lead_out_start IS NULL) !=
         (SELECT COUNT(*) FROM passages WHERE lead_out_start_seconds IS NULL)
    THEN RAISE(ABORT, 'VALIDATION FAILED: lead_out_start NULL count mismatch')
END;

SELECT CASE
    WHEN (SELECT COUNT(*) FROM passages WHERE fade_out_start IS NULL) !=
         (SELECT COUNT(*) FROM passages WHERE fade_out_start_seconds IS NULL)
    THEN RAISE(ABORT, 'VALIDATION FAILED: fade_out_start NULL count mismatch')
END;

SELECT CASE
    WHEN (SELECT COUNT(*) FROM passages WHERE end_time IS NULL) !=
         (SELECT COUNT(*) FROM passages WHERE end_time_seconds IS NULL)
    THEN RAISE(ABORT, 'VALIDATION FAILED: end_time NULL count mismatch')
END;

-- Verify no negative values (timing values should be >= 0)
SELECT CASE
    WHEN EXISTS (SELECT 1 FROM passages WHERE start_time_seconds < 0.0)
    THEN RAISE(ABORT, 'VALIDATION FAILED: negative start_time_seconds detected')
END;

SELECT CASE
    WHEN EXISTS (SELECT 1 FROM passages WHERE end_time_seconds < 0.0)
    THEN RAISE(ABORT, 'VALIDATION FAILED: negative end_time_seconds detected')
END;

-- Verify temporal ordering preserved (start_time <= end_time)
SELECT CASE
    WHEN EXISTS (
        SELECT 1 FROM passages
        WHERE start_time_seconds IS NOT NULL
        AND end_time_seconds IS NOT NULL
        AND start_time_seconds > end_time_seconds
    )
    THEN RAISE(ABORT, 'VALIDATION FAILED: temporal ordering violated (start > end)')
END;

-- Verify round-trip conversion is reversible (within tick precision)
SELECT CASE
    WHEN EXISTS (
        SELECT 1 FROM passages
        WHERE start_time IS NOT NULL
        AND start_time != CAST(ROUND(start_time_seconds * 28224000.0) AS INTEGER)
    )
    THEN RAISE(ABORT, 'VALIDATION FAILED: start_time round-trip conversion error')
END;

SELECT CASE
    WHEN EXISTS (
        SELECT 1 FROM passages
        WHERE end_time IS NOT NULL
        AND end_time != CAST(ROUND(end_time_seconds * 28224000.0) AS INTEGER)
    )
    THEN RAISE(ABORT, 'VALIDATION FAILED: end_time round-trip conversion error')
END;

-- ============================================================================
-- STEP 5: Create New Table with REAL Schema
-- ============================================================================
-- SQLite does not support dropping columns directly, so we recreate the table

-- Create new table with REAL second columns
CREATE TABLE passages_new (
    guid TEXT PRIMARY KEY,
    passage_guid TEXT UNIQUE NOT NULL,
    file_path TEXT NOT NULL,
    start_time REAL,
    fade_in_start REAL,
    lead_in_start REAL,
    lead_out_start REAL,
    fade_out_start REAL,
    end_time REAL,
    recording_guid TEXT,
    work_guid TEXT,
    recording_name TEXT,
    artist_name TEXT,
    work_name TEXT,
    musical_flavor TEXT,
    base_probability REAL DEFAULT 1.0,
    fade_in_curve TEXT DEFAULT 'linear',
    fade_out_curve TEXT DEFAULT 'linear',
    last_played_at INTEGER,
    created_at INTEGER DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (recording_guid) REFERENCES recordings(recording_guid) ON DELETE SET NULL,
    FOREIGN KEY (work_guid) REFERENCES works(work_guid) ON DELETE SET NULL
);

-- Copy data from old table to new table (with second values)
INSERT INTO passages_new (
    guid, passage_guid, file_path,
    start_time, fade_in_start, lead_in_start,
    lead_out_start, fade_out_start, end_time,
    recording_guid, work_guid, recording_name, artist_name, work_name,
    musical_flavor, base_probability, fade_in_curve, fade_out_curve,
    last_played_at, created_at, updated_at
)
SELECT
    guid, passage_guid, file_path,
    start_time_seconds, fade_in_start_seconds, lead_in_start_seconds,
    lead_out_start_seconds, fade_out_start_seconds, end_time_seconds,
    recording_guid, work_guid, recording_name, artist_name, work_name,
    musical_flavor, base_probability, fade_in_curve, fade_out_curve,
    last_played_at, created_at, updated_at
FROM passages;

-- ============================================================================
-- STEP 6: Validate Row Count Preserved
-- ============================================================================

SELECT CASE
    WHEN (SELECT COUNT(*) FROM passages_new) !=
         (SELECT total_rows FROM rollback_validation)
    THEN RAISE(ABORT, 'VALIDATION FAILED: row count mismatch after table recreation')
END;

-- ============================================================================
-- STEP 7: Replace Old Table with New Table
-- ============================================================================

DROP TABLE passages;
ALTER TABLE passages_new RENAME TO passages;

-- ============================================================================
-- STEP 8: Recreate Indexes
-- ============================================================================

-- Recreate any indexes that existed on timing fields
-- (Add actual index creation statements here based on original schema)
-- Example:
-- CREATE INDEX idx_passages_start_time ON passages(start_time);
-- CREATE INDEX idx_passages_end_time ON passages(end_time);

-- ============================================================================
-- STEP 9: Final Validation
-- ============================================================================

-- Verify final table structure uses REAL types
SELECT CASE
    WHEN (SELECT COUNT(*)
          FROM pragma_table_info('passages')
          WHERE name = 'start_time' AND type = 'REAL') = 0
    THEN RAISE(ABORT, 'FINAL VALIDATION FAILED: start_time is not REAL type')
END;

SELECT CASE
    WHEN (SELECT COUNT(*)
          FROM pragma_table_info('passages')
          WHERE name = 'end_time' AND type = 'REAL') = 0
    THEN RAISE(ABORT, 'FINAL VALIDATION FAILED: end_time is not REAL type')
END;

-- Verify no temporary _seconds columns remain
SELECT CASE
    WHEN EXISTS (
        SELECT 1 FROM pragma_table_info('passages')
        WHERE name LIKE '%_seconds'
    )
    THEN RAISE(ABORT, 'FINAL VALIDATION FAILED: temporary _seconds columns still exist')
END;

-- ============================================================================
-- STEP 10: Cleanup and Commit
-- ============================================================================

DROP TABLE rollback_validation;

-- Update schema version (if you have a version table)
-- UPDATE schema_version SET version = version - 1,
--        description = 'T1-TIMING-001 ROLLBACK: Reverted timing fields from INTEGER ticks to REAL seconds',
--        applied_at = strftime('%s', 'now');

COMMIT;

-- ============================================================================
-- ROLLBACK COMPLETE
-- ============================================================================
-- Post-rollback verification steps (run these queries manually):
--
-- 1. Check sample data conversion:
--    SELECT passage_guid, start_time, end_time
--    FROM passages LIMIT 10;
--
-- 2. Verify NULL preservation:
--    SELECT COUNT(*) FROM passages WHERE fade_in_start IS NULL;
--    SELECT COUNT(*) FROM passages WHERE fade_out_start IS NULL;
--
-- 3. Check data types:
--    SELECT name, type FROM pragma_table_info('passages')
--    WHERE name IN ('start_time', 'end_time', 'fade_in_start');
--
-- 4. Verify temporal ordering:
--    SELECT COUNT(*) FROM passages WHERE start_time > end_time;
--    -- Should return 0
--
-- 5. WARNING - Precision comparison:
--    The rolled-back values will differ slightly from original values
--    due to tick quantization. Differences should be < 0.000001 seconds.
-- ============================================================================
