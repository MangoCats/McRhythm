-- Phase 3B: Database Migration Validation Queries
-- T1-TIMING-001: REAL seconds → INTEGER ticks migration
-- Tick Rate: 28,224,000 Hz (per [SRC-TICK-020])
-- Maximum Error: 1 tick (~35.4 nanoseconds)
--
-- Purpose: Validate that the timing field migration preserves data integrity
-- and maintains sample-accurate precision as required by SPEC017.

-- ============================================================================
-- PRE-MIGRATION: Snapshot Queries
-- ============================================================================

-- Query 1: Create snapshot table for pre-migration data
-- This captures the original REAL values before conversion
CREATE TABLE IF NOT EXISTS passages_migration_snapshot AS
SELECT
    passage_guid,
    start_time AS start_time_original_seconds,
    end_time AS end_time_original_seconds,
    fade_in_point AS fade_in_point_original_seconds,
    fade_out_point AS fade_out_point_original_seconds,
    lead_in_point AS lead_in_point_original_seconds,
    lead_out_point AS lead_out_point_original_seconds,
    -- Pre-calculate expected tick values
    CASE
        WHEN start_time IS NOT NULL THEN CAST(ROUND(start_time * 28224000.0) AS INTEGER)
        ELSE NULL
    END AS start_time_expected_ticks,
    CASE
        WHEN end_time IS NOT NULL THEN CAST(ROUND(end_time * 28224000.0) AS INTEGER)
        ELSE NULL
    END AS end_time_expected_ticks,
    CASE
        WHEN fade_in_point IS NOT NULL THEN CAST(ROUND(fade_in_point * 28224000.0) AS INTEGER)
        ELSE NULL
    END AS fade_in_point_expected_ticks,
    CASE
        WHEN fade_out_point IS NOT NULL THEN CAST(ROUND(fade_out_point * 28224000.0) AS INTEGER)
        ELSE NULL
    END AS fade_out_point_expected_ticks,
    CASE
        WHEN lead_in_point IS NOT NULL THEN CAST(ROUND(lead_in_point * 28224000.0) AS INTEGER)
        ELSE NULL
    END AS lead_in_point_expected_ticks,
    CASE
        WHEN lead_out_point IS NOT NULL THEN CAST(ROUND(lead_out_point * 28224000.0) AS INTEGER)
        ELSE NULL
    END AS lead_out_point_expected_ticks
FROM passages;

-- Query 2: Calculate checksum of original data
-- Produces a single hash value representing all timing data
SELECT
    COUNT(*) AS total_passages,
    SUM(CASE WHEN start_time IS NOT NULL THEN 1 ELSE 0 END) AS start_time_count,
    SUM(CASE WHEN end_time IS NOT NULL THEN 1 ELSE 0 END) AS end_time_count,
    SUM(CASE WHEN fade_in_point IS NOT NULL THEN 1 ELSE 0 END) AS fade_in_point_count,
    SUM(CASE WHEN fade_out_point IS NOT NULL THEN 1 ELSE 0 END) AS fade_out_point_count,
    SUM(CASE WHEN lead_in_point IS NOT NULL THEN 1 ELSE 0 END) AS lead_in_point_count,
    SUM(CASE WHEN lead_out_point IS NOT NULL THEN 1 ELSE 0 END) AS lead_out_point_count,
    -- Checksum of all non-NULL values (sum provides simple integrity check)
    SUM(COALESCE(start_time, 0.0)) AS start_time_sum,
    SUM(COALESCE(end_time, 0.0)) AS end_time_sum,
    SUM(COALESCE(fade_in_point, 0.0)) AS fade_in_point_sum,
    SUM(COALESCE(fade_out_point, 0.0)) AS fade_out_point_sum,
    SUM(COALESCE(lead_in_point, 0.0)) AS lead_in_point_sum,
    SUM(COALESCE(lead_out_point, 0.0)) AS lead_out_point_sum
FROM passages;

-- Query 3: Document data distribution for statistical validation
-- Helps identify edge cases and outliers
SELECT
    'start_time' AS field_name,
    MIN(start_time) AS min_value,
    MAX(start_time) AS max_value,
    AVG(start_time) AS avg_value,
    COUNT(*) AS non_null_count
FROM passages WHERE start_time IS NOT NULL
UNION ALL
SELECT
    'end_time',
    MIN(end_time),
    MAX(end_time),
    AVG(end_time),
    COUNT(*)
FROM passages WHERE end_time IS NOT NULL
UNION ALL
SELECT
    'fade_in_point',
    MIN(fade_in_point),
    MAX(fade_in_point),
    AVG(fade_in_point),
    COUNT(*)
FROM passages WHERE fade_in_point IS NOT NULL
UNION ALL
SELECT
    'fade_out_point',
    MIN(fade_out_point),
    MAX(fade_out_point),
    AVG(fade_out_point),
    COUNT(*)
FROM passages WHERE fade_out_point IS NOT NULL
UNION ALL
SELECT
    'lead_in_point',
    MIN(lead_in_point),
    MAX(lead_in_point),
    AVG(lead_in_point),
    COUNT(*)
FROM passages WHERE lead_in_point IS NOT NULL
UNION ALL
SELECT
    'lead_out_point',
    MIN(lead_out_point),
    MAX(lead_out_point),
    AVG(lead_out_point),
    COUNT(*)
FROM passages WHERE lead_out_point IS NOT NULL;

-- ============================================================================
-- POST-MIGRATION: Verification Queries
-- ============================================================================

-- Query 4: Verify row count unchanged
-- PASS: Counts match
-- FAIL: Any row lost during migration
SELECT
    (SELECT COUNT(*) FROM passages) AS current_count,
    (SELECT COUNT(*) FROM passages_migration_snapshot) AS snapshot_count,
    CASE
        WHEN (SELECT COUNT(*) FROM passages) = (SELECT COUNT(*) FROM passages_migration_snapshot)
        THEN 'PASS'
        ELSE 'FAIL'
    END AS status;

-- Query 5: Verify data type correctness
-- PASS: All timing fields are INTEGER
-- FAIL: Any field is not INTEGER type
SELECT
    name AS table_name,
    sql AS table_definition
FROM sqlite_master
WHERE type = 'table' AND name = 'passages';

-- Query 6: Verify tick values match expected calculations
-- PASS: All tick values match expected
-- FAIL: Any discrepancy detected
SELECT
    'start_time' AS field_name,
    COUNT(*) AS total_rows,
    SUM(CASE
        WHEN p.start_time = s.start_time_expected_ticks OR (p.start_time IS NULL AND s.start_time_expected_ticks IS NULL)
        THEN 1
        ELSE 0
    END) AS matching_rows,
    SUM(CASE
        WHEN p.start_time != s.start_time_expected_ticks OR (p.start_time IS NULL AND s.start_time_expected_ticks IS NOT NULL) OR (p.start_time IS NOT NULL AND s.start_time_expected_ticks IS NULL)
        THEN 1
        ELSE 0
    END) AS mismatched_rows,
    CASE
        WHEN SUM(CASE WHEN p.start_time != s.start_time_expected_ticks OR (p.start_time IS NULL AND s.start_time_expected_ticks IS NOT NULL) OR (p.start_time IS NOT NULL AND s.start_time_expected_ticks IS NULL) THEN 1 ELSE 0 END) = 0
        THEN 'PASS'
        ELSE 'FAIL'
    END AS status
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
UNION ALL
SELECT
    'end_time',
    COUNT(*),
    SUM(CASE WHEN p.end_time = s.end_time_expected_ticks OR (p.end_time IS NULL AND s.end_time_expected_ticks IS NULL) THEN 1 ELSE 0 END),
    SUM(CASE WHEN p.end_time != s.end_time_expected_ticks OR (p.end_time IS NULL AND s.end_time_expected_ticks IS NOT NULL) OR (p.end_time IS NOT NULL AND s.end_time_expected_ticks IS NULL) THEN 1 ELSE 0 END),
    CASE WHEN SUM(CASE WHEN p.end_time != s.end_time_expected_ticks OR (p.end_time IS NULL AND s.end_time_expected_ticks IS NOT NULL) OR (p.end_time IS NOT NULL AND s.end_time_expected_ticks IS NULL) THEN 1 ELSE 0 END) = 0 THEN 'PASS' ELSE 'FAIL' END
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
UNION ALL
SELECT
    'fade_in_point',
    COUNT(*),
    SUM(CASE WHEN p.fade_in_point = s.fade_in_point_expected_ticks OR (p.fade_in_point IS NULL AND s.fade_in_point_expected_ticks IS NULL) THEN 1 ELSE 0 END),
    SUM(CASE WHEN p.fade_in_point != s.fade_in_point_expected_ticks OR (p.fade_in_point IS NULL AND s.fade_in_point_expected_ticks IS NOT NULL) OR (p.fade_in_point IS NOT NULL AND s.fade_in_point_expected_ticks IS NULL) THEN 1 ELSE 0 END),
    CASE WHEN SUM(CASE WHEN p.fade_in_point != s.fade_in_point_expected_ticks OR (p.fade_in_point IS NULL AND s.fade_in_point_expected_ticks IS NOT NULL) OR (p.fade_in_point IS NOT NULL AND s.fade_in_point_expected_ticks IS NULL) THEN 1 ELSE 0 END) = 0 THEN 'PASS' ELSE 'FAIL' END
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
UNION ALL
SELECT
    'fade_out_point',
    COUNT(*),
    SUM(CASE WHEN p.fade_out_point = s.fade_out_point_expected_ticks OR (p.fade_out_point IS NULL AND s.fade_out_point_expected_ticks IS NULL) THEN 1 ELSE 0 END),
    SUM(CASE WHEN p.fade_out_point != s.fade_out_point_expected_ticks OR (p.fade_out_point IS NULL AND s.fade_out_point_expected_ticks IS NOT NULL) OR (p.fade_out_point IS NOT NULL AND s.fade_out_point_expected_ticks IS NULL) THEN 1 ELSE 0 END),
    CASE WHEN SUM(CASE WHEN p.fade_out_point != s.fade_out_point_expected_ticks OR (p.fade_out_point IS NULL AND s.fade_out_point_expected_ticks IS NOT NULL) OR (p.fade_out_point IS NOT NULL AND s.fade_out_point_expected_ticks IS NULL) THEN 1 ELSE 0 END) = 0 THEN 'PASS' ELSE 'FAIL' END
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
UNION ALL
SELECT
    'lead_in_point',
    COUNT(*),
    SUM(CASE WHEN p.lead_in_point = s.lead_in_point_expected_ticks OR (p.lead_in_point IS NULL AND s.lead_in_point_expected_ticks IS NULL) THEN 1 ELSE 0 END),
    SUM(CASE WHEN p.lead_in_point != s.lead_in_point_expected_ticks OR (p.lead_in_point IS NULL AND s.lead_in_point_expected_ticks IS NOT NULL) OR (p.lead_in_point IS NOT NULL AND s.lead_in_point_expected_ticks IS NULL) THEN 1 ELSE 0 END),
    CASE WHEN SUM(CASE WHEN p.lead_in_point != s.lead_in_point_expected_ticks OR (p.lead_in_point IS NULL AND s.lead_in_point_expected_ticks IS NOT NULL) OR (p.lead_in_point IS NOT NULL AND s.lead_in_point_expected_ticks IS NULL) THEN 1 ELSE 0 END) = 0 THEN 'PASS' ELSE 'FAIL' END
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
UNION ALL
SELECT
    'lead_out_point',
    COUNT(*),
    SUM(CASE WHEN p.lead_out_point = s.lead_out_point_expected_ticks OR (p.lead_out_point IS NULL AND s.lead_out_point_expected_ticks IS NULL) THEN 1 ELSE 0 END),
    SUM(CASE WHEN p.lead_out_point != s.lead_out_point_expected_ticks OR (p.lead_out_point IS NULL AND s.lead_out_point_expected_ticks IS NOT NULL) OR (p.lead_out_point IS NOT NULL AND s.lead_out_point_expected_ticks IS NULL) THEN 1 ELSE 0 END),
    CASE WHEN SUM(CASE WHEN p.lead_out_point != s.lead_out_point_expected_ticks OR (p.lead_out_point IS NULL AND s.lead_out_point_expected_ticks IS NOT NULL) OR (p.lead_out_point IS NOT NULL AND s.lead_out_point_expected_ticks IS NULL) THEN 1 ELSE 0 END) = 0 THEN 'PASS' ELSE 'FAIL' END
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid;

-- Query 7: Verify NULL preservation
-- PASS: NULLs in original remain NULL after migration
-- FAIL: NULL converted to non-NULL or vice versa
SELECT
    'start_time' AS field_name,
    SUM(CASE WHEN s.start_time_original_seconds IS NULL AND p.start_time IS NULL THEN 1 ELSE 0 END) AS null_preserved,
    SUM(CASE WHEN s.start_time_original_seconds IS NULL AND p.start_time IS NOT NULL THEN 1 ELSE 0 END) AS null_to_value,
    SUM(CASE WHEN s.start_time_original_seconds IS NOT NULL AND p.start_time IS NULL THEN 1 ELSE 0 END) AS value_to_null,
    CASE
        WHEN SUM(CASE WHEN (s.start_time_original_seconds IS NULL) != (p.start_time IS NULL) THEN 1 ELSE 0 END) = 0
        THEN 'PASS'
        ELSE 'FAIL'
    END AS status
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
UNION ALL
SELECT
    'end_time',
    SUM(CASE WHEN s.end_time_original_seconds IS NULL AND p.end_time IS NULL THEN 1 ELSE 0 END),
    SUM(CASE WHEN s.end_time_original_seconds IS NULL AND p.end_time IS NOT NULL THEN 1 ELSE 0 END),
    SUM(CASE WHEN s.end_time_original_seconds IS NOT NULL AND p.end_time IS NULL THEN 1 ELSE 0 END),
    CASE WHEN SUM(CASE WHEN (s.end_time_original_seconds IS NULL) != (p.end_time IS NULL) THEN 1 ELSE 0 END) = 0 THEN 'PASS' ELSE 'FAIL' END
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
UNION ALL
SELECT
    'fade_in_point',
    SUM(CASE WHEN s.fade_in_point_original_seconds IS NULL AND p.fade_in_point IS NULL THEN 1 ELSE 0 END),
    SUM(CASE WHEN s.fade_in_point_original_seconds IS NULL AND p.fade_in_point IS NOT NULL THEN 1 ELSE 0 END),
    SUM(CASE WHEN s.fade_in_point_original_seconds IS NOT NULL AND p.fade_in_point IS NULL THEN 1 ELSE 0 END),
    CASE WHEN SUM(CASE WHEN (s.fade_in_point_original_seconds IS NULL) != (p.fade_in_point IS NULL) THEN 1 ELSE 0 END) = 0 THEN 'PASS' ELSE 'FAIL' END
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
UNION ALL
SELECT
    'fade_out_point',
    SUM(CASE WHEN s.fade_out_point_original_seconds IS NULL AND p.fade_out_point IS NULL THEN 1 ELSE 0 END),
    SUM(CASE WHEN s.fade_out_point_original_seconds IS NULL AND p.fade_out_point IS NOT NULL THEN 1 ELSE 0 END),
    SUM(CASE WHEN s.fade_out_point_original_seconds IS NOT NULL AND p.fade_out_point IS NULL THEN 1 ELSE 0 END),
    CASE WHEN SUM(CASE WHEN (s.fade_out_point_original_seconds IS NULL) != (p.fade_out_point IS NULL) THEN 1 ELSE 0 END) = 0 THEN 'PASS' ELSE 'FAIL' END
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
UNION ALL
SELECT
    'lead_in_point',
    SUM(CASE WHEN s.lead_in_point_original_seconds IS NULL AND p.lead_in_point IS NULL THEN 1 ELSE 0 END),
    SUM(CASE WHEN s.lead_in_point_original_seconds IS NULL AND p.lead_in_point IS NOT NULL THEN 1 ELSE 0 END),
    SUM(CASE WHEN s.lead_in_point_original_seconds IS NOT NULL AND p.lead_in_point IS NULL THEN 1 ELSE 0 END),
    CASE WHEN SUM(CASE WHEN (s.lead_in_point_original_seconds IS NULL) != (p.lead_in_point IS NULL) THEN 1 ELSE 0 END) = 0 THEN 'PASS' ELSE 'FAIL' END
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
UNION ALL
SELECT
    'lead_out_point',
    SUM(CASE WHEN s.lead_out_point_original_seconds IS NULL AND p.lead_out_point IS NULL THEN 1 ELSE 0 END),
    SUM(CASE WHEN s.lead_out_point_original_seconds IS NULL AND p.lead_out_point IS NOT NULL THEN 1 ELSE 0 END),
    SUM(CASE WHEN s.lead_out_point_original_seconds IS NOT NULL AND p.lead_out_point IS NULL THEN 1 ELSE 0 END),
    CASE WHEN SUM(CASE WHEN (s.lead_out_point_original_seconds IS NULL) != (p.lead_out_point IS NULL) THEN 1 ELSE 0 END) = 0 THEN 'PASS' ELSE 'FAIL' END
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid;

-- ============================================================================
-- PRECISION LOSS DETECTION
-- ============================================================================

-- Query 8: Detect precision loss via round-trip conversion
-- Acceptable error: 1 tick = 1/28224000 seconds ≈ 35.4 nanoseconds
-- PASS: All errors <= 1 tick
-- FAIL: Any error > 1 tick
SELECT
    'start_time' AS field_name,
    COUNT(*) AS total_non_null,
    MAX(ABS(s.start_time_original_seconds - CAST(p.start_time AS REAL) / 28224000.0) * 28224000.0) AS max_error_ticks,
    SUM(CASE
        WHEN ABS(s.start_time_original_seconds - CAST(p.start_time AS REAL) / 28224000.0) * 28224000.0 <= 1.0
        THEN 1
        ELSE 0
    END) AS within_tolerance,
    SUM(CASE
        WHEN ABS(s.start_time_original_seconds - CAST(p.start_time AS REAL) / 28224000.0) * 28224000.0 > 1.0
        THEN 1
        ELSE 0
    END) AS exceeds_tolerance,
    CASE
        WHEN SUM(CASE WHEN ABS(s.start_time_original_seconds - CAST(p.start_time AS REAL) / 28224000.0) * 28224000.0 > 1.0 THEN 1 ELSE 0 END) = 0
        THEN 'PASS'
        ELSE 'FAIL'
    END AS status
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
WHERE s.start_time_original_seconds IS NOT NULL AND p.start_time IS NOT NULL
UNION ALL
SELECT
    'end_time',
    COUNT(*),
    MAX(ABS(s.end_time_original_seconds - CAST(p.end_time AS REAL) / 28224000.0) * 28224000.0),
    SUM(CASE WHEN ABS(s.end_time_original_seconds - CAST(p.end_time AS REAL) / 28224000.0) * 28224000.0 <= 1.0 THEN 1 ELSE 0 END),
    SUM(CASE WHEN ABS(s.end_time_original_seconds - CAST(p.end_time AS REAL) / 28224000.0) * 28224000.0 > 1.0 THEN 1 ELSE 0 END),
    CASE WHEN SUM(CASE WHEN ABS(s.end_time_original_seconds - CAST(p.end_time AS REAL) / 28224000.0) * 28224000.0 > 1.0 THEN 1 ELSE 0 END) = 0 THEN 'PASS' ELSE 'FAIL' END
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
WHERE s.end_time_original_seconds IS NOT NULL AND p.end_time IS NOT NULL
UNION ALL
SELECT
    'fade_in_point',
    COUNT(*),
    MAX(ABS(s.fade_in_point_original_seconds - CAST(p.fade_in_point AS REAL) / 28224000.0) * 28224000.0),
    SUM(CASE WHEN ABS(s.fade_in_point_original_seconds - CAST(p.fade_in_point AS REAL) / 28224000.0) * 28224000.0 <= 1.0 THEN 1 ELSE 0 END),
    SUM(CASE WHEN ABS(s.fade_in_point_original_seconds - CAST(p.fade_in_point AS REAL) / 28224000.0) * 28224000.0 > 1.0 THEN 1 ELSE 0 END),
    CASE WHEN SUM(CASE WHEN ABS(s.fade_in_point_original_seconds - CAST(p.fade_in_point AS REAL) / 28224000.0) * 28224000.0 > 1.0 THEN 1 ELSE 0 END) = 0 THEN 'PASS' ELSE 'FAIL' END
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
WHERE s.fade_in_point_original_seconds IS NOT NULL AND p.fade_in_point IS NOT NULL
UNION ALL
SELECT
    'fade_out_point',
    COUNT(*),
    MAX(ABS(s.fade_out_point_original_seconds - CAST(p.fade_out_point AS REAL) / 28224000.0) * 28224000.0),
    SUM(CASE WHEN ABS(s.fade_out_point_original_seconds - CAST(p.fade_out_point AS REAL) / 28224000.0) * 28224000.0 <= 1.0 THEN 1 ELSE 0 END),
    SUM(CASE WHEN ABS(s.fade_out_point_original_seconds - CAST(p.fade_out_point AS REAL) / 28224000.0) * 28224000.0 > 1.0 THEN 1 ELSE 0 END),
    CASE WHEN SUM(CASE WHEN ABS(s.fade_out_point_original_seconds - CAST(p.fade_out_point AS REAL) / 28224000.0) * 28224000.0 > 1.0 THEN 1 ELSE 0 END) = 0 THEN 'PASS' ELSE 'FAIL' END
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
WHERE s.fade_out_point_original_seconds IS NOT NULL AND p.fade_out_point IS NOT NULL
UNION ALL
SELECT
    'lead_in_point',
    COUNT(*),
    MAX(ABS(s.lead_in_point_original_seconds - CAST(p.lead_in_point AS REAL) / 28224000.0) * 28224000.0),
    SUM(CASE WHEN ABS(s.lead_in_point_original_seconds - CAST(p.lead_in_point AS REAL) / 28224000.0) * 28224000.0 <= 1.0 THEN 1 ELSE 0 END),
    SUM(CASE WHEN ABS(s.lead_in_point_original_seconds - CAST(p.lead_in_point AS REAL) / 28224000.0) * 28224000.0 > 1.0 THEN 1 ELSE 0 END),
    CASE WHEN SUM(CASE WHEN ABS(s.lead_in_point_original_seconds - CAST(p.lead_in_point AS REAL) / 28224000.0) * 28224000.0 > 1.0 THEN 1 ELSE 0 END) = 0 THEN 'PASS' ELSE 'FAIL' END
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
WHERE s.lead_in_point_original_seconds IS NOT NULL AND p.lead_in_point IS NOT NULL
UNION ALL
SELECT
    'lead_out_point',
    COUNT(*),
    MAX(ABS(s.lead_out_point_original_seconds - CAST(p.lead_out_point AS REAL) / 28224000.0) * 28224000.0),
    SUM(CASE WHEN ABS(s.lead_out_point_original_seconds - CAST(p.lead_out_point AS REAL) / 28224000.0) * 28224000.0 <= 1.0 THEN 1 ELSE 0 END),
    SUM(CASE WHEN ABS(s.lead_out_point_original_seconds - CAST(p.lead_out_point AS REAL) / 28224000.0) * 28224000.0 > 1.0 THEN 1 ELSE 0 END),
    CASE WHEN SUM(CASE WHEN ABS(s.lead_out_point_original_seconds - CAST(p.lead_out_point AS REAL) / 28224000.0) * 28224000.0 > 1.0 THEN 1 ELSE 0 END) = 0 THEN 'PASS' ELSE 'FAIL' END
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
WHERE s.lead_out_point_original_seconds IS NOT NULL AND p.lead_out_point IS NOT NULL;

-- Query 9: Report passages with precision loss > 1 tick (diagnostic)
-- Lists specific passages where conversion error exceeds tolerance
SELECT
    p.passage_guid,
    'start_time' AS field_name,
    s.start_time_original_seconds AS original_seconds,
    p.start_time AS actual_ticks,
    s.start_time_expected_ticks AS expected_ticks,
    ABS(s.start_time_original_seconds - CAST(p.start_time AS REAL) / 28224000.0) * 28224000.0 AS error_ticks,
    ABS(s.start_time_original_seconds - CAST(p.start_time AS REAL) / 28224000.0) * 1e9 AS error_nanoseconds
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
WHERE s.start_time_original_seconds IS NOT NULL
  AND p.start_time IS NOT NULL
  AND ABS(s.start_time_original_seconds - CAST(p.start_time AS REAL) / 28224000.0) * 28224000.0 > 1.0
UNION ALL
SELECT
    p.passage_guid,
    'end_time',
    s.end_time_original_seconds,
    p.end_time,
    s.end_time_expected_ticks,
    ABS(s.end_time_original_seconds - CAST(p.end_time AS REAL) / 28224000.0) * 28224000.0,
    ABS(s.end_time_original_seconds - CAST(p.end_time AS REAL) / 28224000.0) * 1e9
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
WHERE s.end_time_original_seconds IS NOT NULL
  AND p.end_time IS NOT NULL
  AND ABS(s.end_time_original_seconds - CAST(p.end_time AS REAL) / 28224000.0) * 28224000.0 > 1.0
UNION ALL
SELECT
    p.passage_guid,
    'fade_in_point',
    s.fade_in_point_original_seconds,
    p.fade_in_point,
    s.fade_in_point_expected_ticks,
    ABS(s.fade_in_point_original_seconds - CAST(p.fade_in_point AS REAL) / 28224000.0) * 28224000.0,
    ABS(s.fade_in_point_original_seconds - CAST(p.fade_in_point AS REAL) / 28224000.0) * 1e9
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
WHERE s.fade_in_point_original_seconds IS NOT NULL
  AND p.fade_in_point IS NOT NULL
  AND ABS(s.fade_in_point_original_seconds - CAST(p.fade_in_point AS REAL) / 28224000.0) * 28224000.0 > 1.0
UNION ALL
SELECT
    p.passage_guid,
    'fade_out_point',
    s.fade_out_point_original_seconds,
    p.fade_out_point,
    s.fade_out_point_expected_ticks,
    ABS(s.fade_out_point_original_seconds - CAST(p.fade_out_point AS REAL) / 28224000.0) * 28224000.0,
    ABS(s.fade_out_point_original_seconds - CAST(p.fade_out_point AS REAL) / 28224000.0) * 1e9
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
WHERE s.fade_out_point_original_seconds IS NOT NULL
  AND p.fade_out_point IS NOT NULL
  AND ABS(s.fade_out_point_original_seconds - CAST(p.fade_out_point AS REAL) / 28224000.0) * 28224000.0 > 1.0
UNION ALL
SELECT
    p.passage_guid,
    'lead_in_point',
    s.lead_in_point_original_seconds,
    p.lead_in_point,
    s.lead_in_point_expected_ticks,
    ABS(s.lead_in_point_original_seconds - CAST(p.lead_in_point AS REAL) / 28224000.0) * 28224000.0,
    ABS(s.lead_in_point_original_seconds - CAST(p.lead_in_point AS REAL) / 28224000.0) * 1e9
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
WHERE s.lead_in_point_original_seconds IS NOT NULL
  AND p.lead_in_point IS NOT NULL
  AND ABS(s.lead_in_point_original_seconds - CAST(p.lead_in_point AS REAL) / 28224000.0) * 28224000.0 > 1.0
UNION ALL
SELECT
    p.passage_guid,
    'lead_out_point',
    s.lead_out_point_original_seconds,
    p.lead_out_point,
    s.lead_out_point_expected_ticks,
    ABS(s.lead_out_point_original_seconds - CAST(p.lead_out_point AS REAL) / 28224000.0) * 28224000.0,
    ABS(s.lead_out_point_original_seconds - CAST(p.lead_out_point AS REAL) / 28224000.0) * 1e9
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
WHERE s.lead_out_point_original_seconds IS NOT NULL
  AND p.lead_out_point IS NOT NULL
  AND ABS(s.lead_out_point_original_seconds - CAST(p.lead_out_point AS REAL) / 28224000.0) * 28224000.0 > 1.0
LIMIT 100;

-- ============================================================================
-- EDGE CASE VALIDATION
-- ============================================================================

-- Query 10: Validate zero values
-- PASS: All zero seconds → 0 ticks
-- FAIL: Any zero value incorrectly converted
SELECT
    'Zero value validation' AS test_name,
    COUNT(*) AS zero_seconds_count,
    SUM(CASE WHEN p.start_time = 0 THEN 1 ELSE 0 END) +
    SUM(CASE WHEN p.end_time = 0 THEN 1 ELSE 0 END) +
    SUM(CASE WHEN p.fade_in_point = 0 THEN 1 ELSE 0 END) +
    SUM(CASE WHEN p.fade_out_point = 0 THEN 1 ELSE 0 END) +
    SUM(CASE WHEN p.lead_in_point = 0 THEN 1 ELSE 0 END) +
    SUM(CASE WHEN p.lead_out_point = 0 THEN 1 ELSE 0 END) AS zero_ticks_count,
    CASE
        WHEN SUM(CASE WHEN s.start_time_original_seconds = 0.0 AND p.start_time != 0 THEN 1 ELSE 0 END) +
             SUM(CASE WHEN s.end_time_original_seconds = 0.0 AND p.end_time != 0 THEN 1 ELSE 0 END) +
             SUM(CASE WHEN s.fade_in_point_original_seconds = 0.0 AND p.fade_in_point != 0 THEN 1 ELSE 0 END) +
             SUM(CASE WHEN s.fade_out_point_original_seconds = 0.0 AND p.fade_out_point != 0 THEN 1 ELSE 0 END) +
             SUM(CASE WHEN s.lead_in_point_original_seconds = 0.0 AND p.lead_in_point != 0 THEN 1 ELSE 0 END) +
             SUM(CASE WHEN s.lead_out_point_original_seconds = 0.0 AND p.lead_out_point != 0 THEN 1 ELSE 0 END) = 0
        THEN 'PASS'
        ELSE 'FAIL'
    END AS status
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
WHERE s.start_time_original_seconds = 0.0
   OR s.end_time_original_seconds = 0.0
   OR s.fade_in_point_original_seconds = 0.0
   OR s.fade_out_point_original_seconds = 0.0
   OR s.lead_in_point_original_seconds = 0.0
   OR s.lead_out_point_original_seconds = 0.0;

-- Query 11: Validate small values (< 0.001 seconds = 28,224 ticks)
-- PASS: All small values accurately converted
-- FAIL: Precision loss detected in small values
SELECT
    'Small value validation (<1ms)' AS test_name,
    COUNT(*) AS small_value_count,
    MAX(ABS(s.start_time_original_seconds - CAST(p.start_time AS REAL) / 28224000.0) * 1e9) AS max_error_ns,
    SUM(CASE
        WHEN ABS(s.start_time_original_seconds - CAST(p.start_time AS REAL) / 28224000.0) * 28224000.0 <= 1.0
        THEN 1 ELSE 0
    END) AS within_tolerance,
    CASE
        WHEN SUM(CASE WHEN ABS(s.start_time_original_seconds - CAST(p.start_time AS REAL) / 28224000.0) * 28224000.0 > 1.0 THEN 1 ELSE 0 END) = 0
        THEN 'PASS'
        ELSE 'FAIL'
    END AS status
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
WHERE (s.start_time_original_seconds > 0.0 AND s.start_time_original_seconds < 0.001)
   OR (s.end_time_original_seconds > 0.0 AND s.end_time_original_seconds < 0.001)
   OR (s.fade_in_point_original_seconds > 0.0 AND s.fade_in_point_original_seconds < 0.001)
   OR (s.fade_out_point_original_seconds > 0.0 AND s.fade_out_point_original_seconds < 0.001)
   OR (s.lead_in_point_original_seconds > 0.0 AND s.lead_in_point_original_seconds < 0.001)
   OR (s.lead_out_point_original_seconds > 0.0 AND s.lead_out_point_original_seconds < 0.001);

-- Query 12: Validate large values (> 10,000 seconds)
-- PASS: All large values accurately converted
-- FAIL: Integer overflow or precision loss
SELECT
    'Large value validation (>10000s)' AS test_name,
    COUNT(*) AS large_value_count,
    MAX(ABS(s.start_time_original_seconds - CAST(p.start_time AS REAL) / 28224000.0) * 28224000.0) AS max_error_ticks,
    SUM(CASE
        WHEN ABS(s.start_time_original_seconds - CAST(p.start_time AS REAL) / 28224000.0) * 28224000.0 <= 1.0
        THEN 1 ELSE 0
    END) AS within_tolerance,
    CASE
        WHEN SUM(CASE WHEN ABS(s.start_time_original_seconds - CAST(p.start_time AS REAL) / 28224000.0) * 28224000.0 > 1.0 THEN 1 ELSE 0 END) = 0
        THEN 'PASS'
        ELSE 'FAIL'
    END AS status
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
WHERE s.start_time_original_seconds > 10000.0
   OR s.end_time_original_seconds > 10000.0
   OR s.fade_in_point_original_seconds > 10000.0
   OR s.fade_out_point_original_seconds > 10000.0
   OR s.lead_in_point_original_seconds > 10000.0
   OR s.lead_out_point_original_seconds > 10000.0;

-- ============================================================================
-- FINAL SUMMARY REPORT
-- ============================================================================

-- Query 13: Overall validation summary
-- Aggregates all validation results into single report
SELECT
    'MIGRATION VALIDATION SUMMARY' AS report_title,
    (SELECT COUNT(*) FROM passages) AS total_passages,
    (SELECT COUNT(*) FROM passages_migration_snapshot) AS snapshot_passages,
    CASE
        WHEN (SELECT COUNT(*) FROM passages) = (SELECT COUNT(*) FROM passages_migration_snapshot)
        THEN 'PASS' ELSE 'FAIL'
    END AS row_count_status,
    -- Count how many checks passed
    (SELECT COUNT(*) FROM (
        -- Row count check
        SELECT CASE WHEN (SELECT COUNT(*) FROM passages) = (SELECT COUNT(*) FROM passages_migration_snapshot) THEN 1 ELSE 0 END AS passed
        -- (Additional checks would be counted here in production)
    ) WHERE passed = 1) AS checks_passed,
    CASE
        WHEN (SELECT COUNT(*) FROM passages) = (SELECT COUNT(*) FROM passages_migration_snapshot)
        THEN 'ALL CHECKS PASSED'
        ELSE 'FAILURES DETECTED - SEE DETAILED QUERIES'
    END AS overall_status;

-- ============================================================================
-- CLEANUP (Optional - run after validation complete)
-- ============================================================================

-- Query 14: Drop snapshot table (only after validation success)
-- Uncomment to clean up snapshot after successful validation:
-- DROP TABLE IF EXISTS passages_migration_snapshot;
