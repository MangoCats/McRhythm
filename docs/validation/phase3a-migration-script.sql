-- ============================================================================
-- WKMP Database Migration: REAL Seconds → INTEGER Ticks
-- ============================================================================
-- Migration File: 2025-10-19_tick_timing_migration.sql
-- Created: 2025-10-19
-- Authority: T1-TIMING-001 (Tier 1 APPROVED)
-- Reference: SPEC017-sample_accurate_timing.md
--
-- PURPOSE:
--   Convert 6 timing fields in the `passages` table from REAL (seconds)
--   to INTEGER (ticks) for sample-accurate precision.
--
-- TICK RATE: 28,224,000 Hz (SRC-TICK-020)
--   - LCM of common sample rates: 44100, 48000, 88200, 96000, 176400, 192000
--   - Enables lossless conversion between all standard audio formats
--   - Precision: ~0.035 microseconds per tick
--
-- FIELDS AFFECTED:
--   start_time REAL           → start_time INTEGER NOT NULL
--   fade_in_start REAL        → fade_in_start INTEGER
--   lead_in_start REAL        → lead_in_start INTEGER
--   lead_out_start REAL       → lead_out_start INTEGER
--   fade_out_start REAL       → fade_out_start INTEGER
--   end_time REAL             → end_time INTEGER NOT NULL
--
-- CONVERSION FORMULA:
--   ticks = ROUND(seconds * 28224000)
--
-- MIGRATION STRATEGY:
--   1. Add new INTEGER columns with temporary _ticks suffix
--   2. Migrate data with NULL preservation
--   3. Validate conversion (zero data loss)
--   4. Drop old REAL columns
--   5. Rename _ticks columns to original names
--
-- ROLLBACK:
--   See companion file: 2025-10-19_tick_timing_rollback.sql
--
-- SAFETY:
--   - Wrapped in transaction (auto-rollback on error)
--   - Multiple validation checkpoints
--   - NULL values preserved
--   - Data integrity constraints enforced
-- ============================================================================

BEGIN TRANSACTION;

-- ============================================================================
-- STEP 1: Pre-Migration Validation
-- ============================================================================

-- Verify passages table exists
SELECT CASE
    WHEN (SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='passages') = 0
    THEN RAISE(ABORT, 'PRE-CHECK FAILED: passages table does not exist')
END;

-- Store initial row count for verification
CREATE TEMP TABLE migration_validation AS
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
-- STEP 2: Add New INTEGER Columns (Temporary _ticks Suffix)
-- ============================================================================

ALTER TABLE passages ADD COLUMN start_time_ticks INTEGER;
ALTER TABLE passages ADD COLUMN fade_in_start_ticks INTEGER;
ALTER TABLE passages ADD COLUMN lead_in_start_ticks INTEGER;
ALTER TABLE passages ADD COLUMN lead_out_start_ticks INTEGER;
ALTER TABLE passages ADD COLUMN fade_out_start_ticks INTEGER;
ALTER TABLE passages ADD COLUMN end_time_ticks INTEGER;

-- ============================================================================
-- STEP 3: Migrate Data (REAL Seconds → INTEGER Ticks)
-- ============================================================================

UPDATE passages
SET
    start_time_ticks = CASE
        WHEN start_time IS NOT NULL
        THEN CAST(ROUND(start_time * 28224000.0) AS INTEGER)
        ELSE NULL
    END,
    fade_in_start_ticks = CASE
        WHEN fade_in_start IS NOT NULL
        THEN CAST(ROUND(fade_in_start * 28224000.0) AS INTEGER)
        ELSE NULL
    END,
    lead_in_start_ticks = CASE
        WHEN lead_in_start IS NOT NULL
        THEN CAST(ROUND(lead_in_start * 28224000.0) AS INTEGER)
        ELSE NULL
    END,
    lead_out_start_ticks = CASE
        WHEN lead_out_start IS NOT NULL
        THEN CAST(ROUND(lead_out_start * 28224000.0) AS INTEGER)
        ELSE NULL
    END,
    fade_out_start_ticks = CASE
        WHEN fade_out_start IS NOT NULL
        THEN CAST(ROUND(fade_out_start * 28224000.0) AS INTEGER)
        ELSE NULL
    END,
    end_time_ticks = CASE
        WHEN end_time IS NOT NULL
        THEN CAST(ROUND(end_time * 28224000.0) AS INTEGER)
        ELSE NULL
    END;

-- ============================================================================
-- STEP 4: Validate Conversion
-- ============================================================================

-- Verify NULL counts unchanged
SELECT CASE
    WHEN (SELECT COUNT(*) FROM passages WHERE start_time IS NULL) !=
         (SELECT COUNT(*) FROM passages WHERE start_time_ticks IS NULL)
    THEN RAISE(ABORT, 'VALIDATION FAILED: start_time NULL count mismatch')
END;

SELECT CASE
    WHEN (SELECT COUNT(*) FROM passages WHERE fade_in_start IS NULL) !=
         (SELECT COUNT(*) FROM passages WHERE fade_in_start_ticks IS NULL)
    THEN RAISE(ABORT, 'VALIDATION FAILED: fade_in_start NULL count mismatch')
END;

SELECT CASE
    WHEN (SELECT COUNT(*) FROM passages WHERE lead_in_start IS NULL) !=
         (SELECT COUNT(*) FROM passages WHERE lead_in_start_ticks IS NULL)
    THEN RAISE(ABORT, 'VALIDATION FAILED: lead_in_start NULL count mismatch')
END;

SELECT CASE
    WHEN (SELECT COUNT(*) FROM passages WHERE lead_out_start IS NULL) !=
         (SELECT COUNT(*) FROM passages WHERE lead_out_start_ticks IS NULL)
    THEN RAISE(ABORT, 'VALIDATION FAILED: lead_out_start NULL count mismatch')
END;

SELECT CASE
    WHEN (SELECT COUNT(*) FROM passages WHERE fade_out_start IS NULL) !=
         (SELECT COUNT(*) FROM passages WHERE fade_out_start_ticks IS NULL)
    THEN RAISE(ABORT, 'VALIDATION FAILED: fade_out_start NULL count mismatch')
END;

SELECT CASE
    WHEN (SELECT COUNT(*) FROM passages WHERE end_time IS NULL) !=
         (SELECT COUNT(*) FROM passages WHERE end_time_ticks IS NULL)
    THEN RAISE(ABORT, 'VALIDATION FAILED: end_time NULL count mismatch')
END;

-- Verify precision loss is acceptable (< 0.000001 seconds for typical passages)
-- Check that round-trip conversion error is minimal
SELECT CASE
    WHEN EXISTS (
        SELECT 1 FROM passages
        WHERE start_time IS NOT NULL
        AND ABS(start_time - (start_time_ticks / 28224000.0)) > 0.000001
    )
    THEN RAISE(ABORT, 'VALIDATION FAILED: start_time precision loss detected')
END;

SELECT CASE
    WHEN EXISTS (
        SELECT 1 FROM passages
        WHERE end_time IS NOT NULL
        AND ABS(end_time - (end_time_ticks / 28224000.0)) > 0.000001
    )
    THEN RAISE(ABORT, 'VALIDATION FAILED: end_time precision loss detected')
END;

-- Verify no negative values created (timing values should be >= 0)
SELECT CASE
    WHEN EXISTS (SELECT 1 FROM passages WHERE start_time_ticks < 0)
    THEN RAISE(ABORT, 'VALIDATION FAILED: negative start_time_ticks detected')
END;

SELECT CASE
    WHEN EXISTS (SELECT 1 FROM passages WHERE end_time_ticks < 0)
    THEN RAISE(ABORT, 'VALIDATION FAILED: negative end_time_ticks detected')
END;

-- Verify temporal ordering is preserved (start_time <= end_time)
SELECT CASE
    WHEN EXISTS (
        SELECT 1 FROM passages
        WHERE start_time_ticks IS NOT NULL
        AND end_time_ticks IS NOT NULL
        AND start_time_ticks > end_time_ticks
    )
    THEN RAISE(ABORT, 'VALIDATION FAILED: temporal ordering violated (start > end)')
END;

-- ============================================================================
-- STEP 5: Create New Table with Correct Schema
-- ============================================================================
-- SQLite does not support dropping columns directly, so we recreate the table

-- Create new table with INTEGER tick columns
CREATE TABLE passages_new (
    guid TEXT PRIMARY KEY,
    passage_guid TEXT UNIQUE NOT NULL,
    file_path TEXT NOT NULL,
    start_time INTEGER NOT NULL,
    fade_in_start INTEGER,
    lead_in_start INTEGER,
    lead_out_start INTEGER,
    fade_out_start INTEGER,
    end_time INTEGER NOT NULL,
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

-- Copy data from old table to new table (with tick values)
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
    start_time_ticks, fade_in_start_ticks, lead_in_start_ticks,
    lead_out_start_ticks, fade_out_start_ticks, end_time_ticks,
    recording_guid, work_guid, recording_name, artist_name, work_name,
    musical_flavor, base_probability, fade_in_curve, fade_out_curve,
    last_played_at, created_at, updated_at
FROM passages;

-- ============================================================================
-- STEP 6: Validate Row Count Preserved
-- ============================================================================

SELECT CASE
    WHEN (SELECT COUNT(*) FROM passages_new) !=
         (SELECT total_rows FROM migration_validation)
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
-- (Add actual index creation statements here based on current schema)
-- Example:
-- CREATE INDEX idx_passages_start_time ON passages(start_time);
-- CREATE INDEX idx_passages_end_time ON passages(end_time);

-- ============================================================================
-- STEP 9: Final Validation
-- ============================================================================

-- Verify final table structure
SELECT CASE
    WHEN (SELECT COUNT(*)
          FROM pragma_table_info('passages')
          WHERE name = 'start_time' AND type = 'INTEGER') = 0
    THEN RAISE(ABORT, 'FINAL VALIDATION FAILED: start_time is not INTEGER type')
END;

SELECT CASE
    WHEN (SELECT COUNT(*)
          FROM pragma_table_info('passages')
          WHERE name = 'end_time' AND type = 'INTEGER') = 0
    THEN RAISE(ABORT, 'FINAL VALIDATION FAILED: end_time is not INTEGER type')
END;

-- Verify no old REAL columns remain
SELECT CASE
    WHEN EXISTS (
        SELECT 1 FROM pragma_table_info('passages')
        WHERE name LIKE '%_ticks'
    )
    THEN RAISE(ABORT, 'FINAL VALIDATION FAILED: temporary _ticks columns still exist')
END;

-- ============================================================================
-- STEP 10: Cleanup and Commit
-- ============================================================================

DROP TABLE migration_validation;

-- Update schema version (if you have a version table)
-- UPDATE schema_version SET version = version + 1,
--        description = 'T1-TIMING-001: Migrated timing fields from REAL to INTEGER ticks',
--        applied_at = strftime('%s', 'now');

COMMIT;

-- ============================================================================
-- MIGRATION COMPLETE
-- ============================================================================
-- Post-migration verification steps (run these queries manually):
--
-- 1. Check sample data conversion:
--    SELECT passage_guid, start_time, end_time,
--           start_time / 28224000.0 AS start_seconds,
--           end_time / 28224000.0 AS end_seconds
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
-- ============================================================================
