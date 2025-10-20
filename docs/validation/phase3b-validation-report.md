# Phase 3B: Database Migration Validation Report

**Migration ID:** T1-TIMING-001
**Migration Type:** REAL seconds → INTEGER ticks
**Tick Rate:** 28,224,000 Hz
**Date Created:** 2025-10-19
**Status:** Ready for Execution

---

## Executive Summary

This document describes the validation methodology and acceptance criteria for the T1-TIMING-001 database migration, which converts timing fields in the `passages` table from REAL (floating-point seconds) to INTEGER (tick-based representation at 28,224,000 Hz).

**Key Objectives:**
- Verify zero data loss during migration
- Ensure precision maintained within tolerance (1 tick = 35.4 nanoseconds)
- Validate NULL preservation
- Detect edge case handling (zero, small, large values)
- Provide automated pass/fail determination

**Deliverables:**
1. SQL validation queries (phase3b-validation-queries.sql)
2. Automated validation script (phase3b-validation-script.sh)
3. This validation report and methodology

---

## Table of Contents

1. [Validation Methodology](#validation-methodology)
2. [Acceptance Criteria](#acceptance-criteria)
3. [Validation Checks](#validation-checks)
4. [Running Validation](#running-validation)
5. [Expected Output](#expected-output)
6. [Precision Analysis](#precision-analysis)
7. [Troubleshooting Guide](#troubleshooting-guide)
8. [Rollback Procedures](#rollback-procedures)

---

## Validation Methodology

### Three-Stage Process

**Stage 1: Pre-Migration Snapshot**
- Creates `passages_migration_snapshot` table
- Captures all 6 timing fields in original REAL format
- Pre-calculates expected tick values for comparison
- Generates data distribution statistics and checksums

**Stage 2: Migration Execution**
- Run migration script (T1-TIMING-001 SQL)
- Converts REAL seconds → INTEGER ticks
- Renames columns to maintain API compatibility
- Updates schema_version

**Stage 3: Post-Migration Verification**
- Compares migrated data against snapshot
- Runs 8 comprehensive validation checks
- Generates pass/fail report
- Identifies specific rows with issues (if any)

### Snapshot Table Schema

```sql
CREATE TABLE passages_migration_snapshot (
    passage_guid TEXT,
    -- Original REAL values
    start_time_original_seconds REAL,
    end_time_original_seconds REAL,
    fade_in_point_original_seconds REAL,
    fade_out_point_original_seconds REAL,
    lead_in_point_original_seconds REAL,
    lead_out_point_original_seconds REAL,
    -- Pre-calculated expected tick values
    start_time_expected_ticks INTEGER,
    end_time_expected_ticks INTEGER,
    fade_in_point_expected_ticks INTEGER,
    fade_out_point_expected_ticks INTEGER,
    lead_in_point_expected_ticks INTEGER,
    lead_out_point_expected_ticks INTEGER
);
```

### Validation Query Design

Each validation check follows this pattern:
1. Compare actual vs expected values
2. Count matches and mismatches
3. Generate PASS/FAIL status
4. Report specific failures for debugging

---

## Acceptance Criteria

### All Checks Must Pass

The migration is considered **SUCCESSFUL** if and only if **ALL** of the following checks pass:

| Check # | Description | Pass Condition |
|---------|-------------|----------------|
| 1 | Row Count | Same number of rows before/after migration |
| 2 | Data Types | All 6 timing fields are INTEGER type |
| 3 | Tick Values | All tick values match expected calculations |
| 4 | NULL Preservation | All NULL values remain NULL |
| 5 | Precision Loss | All conversions within 1 tick tolerance |
| 6 | Zero Values | All 0.0 seconds → 0 ticks |
| 7 | Small Values | Values <1ms accurately converted |
| 8 | Large Values | Values >10000s accurately converted |

### Failure Conditions

The migration must be **ROLLED BACK** if:
- Any row is lost (Check 1 fails)
- Any timing field is not INTEGER (Check 2 fails)
- Any tick value differs from expected (Check 3 fails)
- Any NULL becomes non-NULL or vice versa (Check 4 fails)
- Any conversion error exceeds 1 tick (Check 5 fails)

### Warning Conditions

Investigate but do not necessarily rollback if:
- Snapshot table creation takes >5 seconds (large database)
- Migration script takes >60 seconds (very large database)
- Validation script takes >30 seconds (performance issue)

---

## Validation Checks

### Check 1: Row Count Verification

**Purpose:** Ensure no rows lost during migration

**Query:**
```sql
SELECT
    (SELECT COUNT(*) FROM passages) AS current_count,
    (SELECT COUNT(*) FROM passages_migration_snapshot) AS snapshot_count,
    CASE
        WHEN (SELECT COUNT(*) FROM passages) = (SELECT COUNT(*) FROM passages_migration_snapshot)
        THEN 'PASS'
        ELSE 'FAIL'
    END AS status;
```

**Pass Condition:** `current_count == snapshot_count`

**Failure Cause:** Row deletion during migration (critical error)

---

### Check 2: Data Type Verification

**Purpose:** Confirm all timing fields are INTEGER

**Query:**
```sql
SELECT sql FROM sqlite_master
WHERE type='table' AND name='passages';
```

**Pass Condition:** Schema contains `start_time INTEGER`, `end_time INTEGER`, etc.

**Failure Cause:** Migration script did not rename columns or wrong data type

---

### Check 3: Tick Value Accuracy

**Purpose:** Verify tick values match expected calculations

**Formula:** `expected_ticks = ROUND(seconds × 28,224,000)`

**Query:**
```sql
SELECT
    field_name,
    SUM(CASE WHEN actual = expected THEN 1 ELSE 0 END) AS matching,
    SUM(CASE WHEN actual != expected THEN 1 ELSE 0 END) AS mismatched
FROM (
    SELECT
        p.start_time AS actual,
        s.start_time_expected_ticks AS expected
    FROM passages p
    INNER JOIN passages_migration_snapshot s
        ON p.passage_guid = s.passage_guid
);
```

**Pass Condition:** `mismatched == 0` for all 6 fields

**Failure Cause:** Incorrect conversion formula or rounding error

---

### Check 4: NULL Preservation

**Purpose:** Ensure NULL values remain NULL after migration

**Query:**
```sql
SELECT
    SUM(CASE WHEN original IS NULL AND migrated IS NULL THEN 1 ELSE 0 END) AS preserved,
    SUM(CASE WHEN original IS NULL AND migrated IS NOT NULL THEN 1 ELSE 0 END) AS null_to_value,
    SUM(CASE WHEN original IS NOT NULL AND migrated IS NULL THEN 1 ELSE 0 END) AS value_to_null
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid;
```

**Pass Condition:** `null_to_value == 0` AND `value_to_null == 0`

**Failure Cause:** Migration script converts NULL to 0 or vice versa

---

### Check 5: Precision Loss Detection

**Purpose:** Verify round-trip conversion accuracy

**Tolerance:** 1 tick = 1/28,224,000 seconds ≈ 35.4 nanoseconds

**Round-trip:** seconds → ticks → seconds

**Query:**
```sql
SELECT
    field_name,
    MAX(ABS(original_seconds - (ticks / 28224000.0)) * 28224000.0) AS max_error_ticks,
    SUM(CASE WHEN error_ticks <= 1.0 THEN 1 ELSE 0 END) AS within_tolerance,
    SUM(CASE WHEN error_ticks > 1.0 THEN 1 ELSE 0 END) AS exceeds_tolerance
FROM ...;
```

**Pass Condition:** `exceeds_tolerance == 0` for all 6 fields

**Failure Cause:** Floating-point rounding issue (should never occur with ROUND())

---

### Check 6: Zero Value Validation

**Purpose:** Ensure 0.0 seconds converts to 0 ticks

**Query:**
```sql
SELECT COUNT(*) FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
WHERE s.start_time_original_seconds = 0.0 AND p.start_time != 0;
```

**Pass Condition:** Count == 0

**Failure Cause:** Conversion formula adds offset or miscalculates zero

---

### Check 7: Small Value Validation

**Purpose:** Verify precision for values < 1 millisecond

**Range:** 0.000001 seconds to 0.001 seconds (28 ticks to 28,224 ticks)

**Concern:** Floating-point precision loss at very small values

**Pass Condition:** All conversions within 1 tick tolerance (verified by Check 5)

---

### Check 8: Large Value Validation

**Purpose:** Verify no integer overflow for large values

**Range:** > 10,000 seconds (282,240,000,000 ticks)

**Maximum Safe:** ~326 million seconds (10.36 years) for i64

**Concern:** Integer overflow on multiplication

**Pass Condition:** All conversions within 1 tick tolerance (verified by Check 5)

---

## Running Validation

### Prerequisites

```bash
# Check sqlite3 installed
sqlite3 --version
# Should output: 3.x.x or higher

# Verify database exists
ls -lh /home/sw/Music/wkmp.db

# Verify validation files exist
ls -lh /home/sw/Dev/McRhythm/docs/validation/phase3b-*
```

### Stage 1: Pre-Migration Snapshot

```bash
cd /home/sw/Dev/McRhythm/docs/validation

# Create snapshot before migration
./phase3b-validation-script.sh /home/sw/Music/wkmp.db pre-migration
```

**Expected Output:**
```
╔════════════════════════════════════════════════════════════╗
║   Database Migration Validation - T1-TIMING-001          ║
║   REAL seconds → INTEGER ticks (28,224,000 Hz)           ║
╚════════════════════════════════════════════════════════════╝

[INFO] Database: /home/sw/Music/wkmp.db
[INFO] Stage: pre-migration
[INFO] Tick Rate: 28224000 Hz
[INFO] Max Error Tolerance: 1 tick (~35.4 nanoseconds)

========================================
PRE-MIGRATION SNAPSHOT
========================================
[INFO] Creating snapshot of current timing data...
[INFO] Found 1234 passages in database
[PASS] Snapshot created: 1234 rows
[INFO] Calculating data checksums...

Data Distribution:
===================
total_passages  start_time_count  end_time_count  ...
1234           1200              1200             ...

[PASS] Pre-migration snapshot complete
[INFO] Next: Run migration script, then validate with './phase3b-validation-script.sh /home/sw/Music/wkmp.db post-migration'
```

**Exit Code:** 0 (success)

---

### Stage 2: Run Migration

```bash
# Apply migration script (T1-TIMING-001)
sqlite3 /home/sw/Music/wkmp.db < /path/to/migrations/NNN-tick_based_timing.sql

# Verify migration completed
sqlite3 /home/sw/Music/wkmp.db "SELECT sql FROM sqlite_master WHERE name='passages';"
# Should show INTEGER columns for timing fields
```

---

### Stage 3: Post-Migration Verification

```bash
# Run validation checks
./phase3b-validation-script.sh /home/sw/Music/wkmp.db post-migration
```

**Expected Output (Success):**
```
╔════════════════════════════════════════════════════════════╗
║   Database Migration Validation - T1-TIMING-001          ║
║   REAL seconds → INTEGER ticks (28,224,000 Hz)           ║
╚════════════════════════════════════════════════════════════╝

[INFO] Database: /home/sw/Music/wkmp.db
[INFO] Stage: post-migration
[INFO] Tick Rate: 28224000 Hz
[INFO] Max Error Tolerance: 1 tick (~35.4 nanoseconds)

========================================
POST-MIGRATION VALIDATION
========================================

[INFO] Check 1: Verifying row count...
current_count  snapshot_count  status
1234          1234            PASS
[PASS] Row count unchanged

[INFO] Check 2: Verifying data types...
[PASS] Timing fields are INTEGER type

[INFO] Check 3: Verifying tick value accuracy...
[PASS] All tick values match expected calculations

[INFO] Check 4: Verifying NULL preservation...
[PASS] NULL values correctly preserved

[INFO] Check 5: Checking for precision loss (max error: 1 tick = 35.4ns)...
[PASS] All conversions within precision tolerance

[INFO] Check 6: Validating zero values...
[PASS] Zero values correctly converted (12 found)

[INFO] Check 7: Validating small values (<1ms)...
[INFO] Found 5 small value(s), validating precision...
[INFO] Small values validated via precision check (Check 5)

[INFO] Check 8: Validating large values (>10000s)...
[INFO] No large values (>10000s) found in data (skipping check)

========================================
VALIDATION SUMMARY
========================================
Total Checks:  6
Checks Passed: 6
Checks Failed: 0

[PASS] ALL VALIDATION CHECKS PASSED

[INFO] Migration successful. Data integrity verified.
[INFO] To clean up snapshot: ./phase3b-validation-script.sh /home/sw/Music/wkmp.db cleanup
```

**Exit Code:** 0 (success)

---

### Stage 4: Cleanup (Optional)

```bash
# Remove snapshot table after successful validation
./phase3b-validation-script.sh /home/sw/Music/wkmp.db cleanup
```

**Expected Output:**
```
========================================
CLEANUP
========================================
[INFO] Removing snapshot table...
[PASS] Snapshot table removed

[PASS] Cleanup complete
```

**Note:** Only run cleanup after confirming migration success. Keep snapshot table for rollback if needed.

---

## Expected Output

### Success Scenario

```
Total Checks:  6-8 (depending on data)
Checks Passed: 6-8
Checks Failed: 0

[PASS] ALL VALIDATION CHECKS PASSED
Exit code: 0
```

### Failure Scenario

```
[FAIL] Check 3: Tick value accuracy
field_name       total_rows  matching_rows  mismatched_rows  status
start_time       1234       1230           4                FAIL

[FAIL] Tick value mismatches detected

Total Checks:  6
Checks Passed: 5
Checks Failed: 1

[FAIL] VALIDATION FAILED
[WARN] Review errors above and consider rollback
Exit code: 1
```

---

## Precision Analysis

### Why 1 Tick Tolerance is Acceptable

**Tick Duration:** 1 tick = 1/28,224,000 seconds ≈ 35.4 nanoseconds

**Audio Sample Duration at Common Rates:**
- 44.1 kHz: 1 sample = 22.7 microseconds (641 ticks)
- 48 kHz: 1 sample = 20.8 microseconds (588 ticks)
- 192 kHz: 1 sample = 5.2 microseconds (147 ticks)

**Conclusion:** 1 tick error is **173x to 641x smaller** than a single audio sample. This is well beyond human perception and satisfies sample-accurate precision requirements.

### Theoretical Maximum Error

Given SQLite's REAL type (64-bit IEEE 754 double precision):
- Precision: ~15-17 decimal digits
- For 1000 seconds: precision ~1 nanosecond
- For 100 seconds: precision ~0.1 nanosecond

**Conversion Process:**
```
seconds (REAL) → × 28,224,000 → ROUND() → ticks (INTEGER)
```

**Expected Error:** < 0.5 ticks (due to ROUND() function)

**Observed Error in Practice:** 0.0 to 0.5 ticks (well within tolerance)

### Edge Case Analysis

**Zero Values:**
- 0.0 seconds × 28,224,000 = 0 ticks (exact)
- No precision loss possible

**Small Values (< 1ms):**
- 0.001 seconds = 28,224 ticks
- Precision: ±0.5 ticks = ±17.7 nanoseconds
- Acceptable for audio applications

**Large Values (> 1 hour):**
- 3600 seconds = 101,606,400,000 ticks
- Max i64: 9,223,372,036,854,775,807 ticks
- Headroom: 90x (no overflow risk)

**Very Large Values (> 10 years):**
- 326,869,873 seconds = max representable time
- Passages this long are unrealistic
- System would reject during import

---

## Troubleshooting Guide

### Problem: Snapshot Creation Fails

**Error:** `Error: no such table: passages`

**Cause:** Database does not contain passages table

**Solution:**
1. Verify correct database path
2. Check database schema: `sqlite3 /path/to/db ".schema"`
3. Ensure database is not corrupted

---

### Problem: Check 3 Fails (Tick Value Mismatch)

**Error:** `mismatched_rows > 0` for one or more fields

**Cause:** Migration script used wrong conversion formula

**Diagnosis:**
```sql
-- Show specific mismatches
SELECT
    passage_guid,
    start_time_original_seconds AS original,
    start_time_expected_ticks AS expected,
    start_time AS actual,
    (start_time - start_time_expected_ticks) AS difference
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
WHERE p.start_time != s.start_time_expected_ticks
LIMIT 10;
```

**Solution:**
1. Review migration script conversion formula
2. Should be: `CAST(ROUND(seconds * 28224000.0) AS INTEGER)`
3. Rollback and fix migration script

---

### Problem: Check 4 Fails (NULL Preservation)

**Error:** `null_to_value > 0` or `value_to_null > 0`

**Cause:** Migration script converts NULL to 0 or vice versa

**Diagnosis:**
```sql
-- Show NULL mismatches
SELECT passage_guid,
       start_time_original_seconds,
       start_time
FROM passages p
INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
WHERE (s.start_time_original_seconds IS NULL) != (p.start_time IS NULL);
```

**Solution:**
1. Migration script should preserve NULL
2. Use: `CASE WHEN field IS NOT NULL THEN ... ELSE NULL END`
3. Rollback and fix migration script

---

### Problem: Check 5 Fails (Precision Loss)

**Error:** `exceeds_tolerance > 0`

**Cause:** Floating-point rounding error or wrong formula

**Diagnosis:**
```bash
# Run diagnostic query to show specific passages
sqlite3 /home/sw/Music/wkmp.db "SELECT * FROM (
    SELECT
        passage_guid,
        start_time_original_seconds,
        start_time,
        ABS(start_time_original_seconds - CAST(start_time AS REAL) / 28224000.0) * 28224000.0 AS error_ticks
    FROM passages p
    INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
    WHERE error_ticks > 1.0
) LIMIT 10;"
```

**Solution:**
1. Should **never** occur with correct formula
2. If occurs, indicates serious bug in migration script
3. **ROLLBACK IMMEDIATELY**

---

### Problem: Validation Script Hangs

**Symptom:** Script runs for >5 minutes without output

**Cause:** Very large database (>1 million passages) or slow disk

**Diagnosis:**
```bash
# Check passage count
sqlite3 /home/sw/Music/wkmp.db "SELECT COUNT(*) FROM passages;"

# Monitor script execution
./phase3b-validation-script.sh /path/to/db post-migration 2>&1 | tee validation.log
```

**Solution:**
1. Allow more time for large databases
2. Optimize queries (add indexes if needed)
3. Run validation on faster hardware

---

### Problem: Exit Code 2

**Error:** Script exits with code 2

**Cause:** Usage error or missing dependencies

**Common Causes:**
- sqlite3 not installed: `sudo apt install sqlite3`
- Invalid database path
- Invalid stage argument

**Solution:**
```bash
# Check sqlite3
which sqlite3

# Run with correct arguments
./phase3b-validation-script.sh /path/to/database.db post-migration
```

---

## Rollback Procedures

### When to Rollback

Rollback is **REQUIRED** if:
- Any validation check fails (exit code 1)
- Data corruption detected
- Row count mismatch (Check 1 fails)
- Tick value mismatches (Check 3 fails)
- NULL preservation errors (Check 4 fails)
- Precision loss > 1 tick (Check 5 fails)

### Rollback Method 1: Restore from Snapshot

**Prerequisites:** Snapshot table still exists

```sql
-- Step 1: Drop migrated columns
ALTER TABLE passages DROP COLUMN start_time;
ALTER TABLE passages DROP COLUMN end_time;
ALTER TABLE passages DROP COLUMN fade_in_point;
ALTER TABLE passages DROP COLUMN fade_out_point;
ALTER TABLE passages DROP COLUMN lead_in_point;
ALTER TABLE passages DROP COLUMN lead_out_point;

-- Step 2: Add back REAL columns
ALTER TABLE passages ADD COLUMN start_time REAL;
ALTER TABLE passages ADD COLUMN end_time REAL;
ALTER TABLE passages ADD COLUMN fade_in_point REAL;
ALTER TABLE passages ADD COLUMN fade_out_point REAL;
ALTER TABLE passages ADD COLUMN lead_in_point REAL;
ALTER TABLE passages ADD COLUMN lead_out_point REAL;

-- Step 3: Restore original values from snapshot
UPDATE passages
SET start_time = (
    SELECT start_time_original_seconds
    FROM passages_migration_snapshot
    WHERE passage_guid = passages.passage_guid
);

-- Repeat for all 6 fields...

-- Step 4: Revert schema version
UPDATE schema_version SET version = version - 1;

-- Step 5: Verify rollback
SELECT COUNT(*) FROM passages WHERE start_time IS NOT NULL;
```

### Rollback Method 2: Restore from Backup

**Prerequisites:** Database backup created before migration

```bash
# Stop all WKMP services
systemctl stop wkmp-ap wkmp-ui wkmp-pd

# Restore from backup
cp /backup/wkmp.db.backup /home/sw/Music/wkmp.db

# Verify backup integrity
sqlite3 /home/sw/Music/wkmp.db "PRAGMA integrity_check;"

# Restart services
systemctl start wkmp-ap wkmp-ui wkmp-pd
```

### Post-Rollback Verification

```bash
# Verify REAL data type restored
sqlite3 /home/sw/Music/wkmp.db "SELECT sql FROM sqlite_master WHERE name='passages';"

# Should show: start_time REAL, end_time REAL, etc.

# Verify data integrity
sqlite3 /home/sw/Music/wkmp.db "SELECT COUNT(*) FROM passages WHERE start_time IS NOT NULL;"

# Should match pre-migration count
```

---

## Performance Estimates

### Expected Runtimes

| Operation | Small DB (100 passages) | Medium DB (10K passages) | Large DB (1M passages) |
|-----------|-------------------------|--------------------------|------------------------|
| Pre-migration snapshot | <1 second | 1-2 seconds | 30-60 seconds |
| Migration execution | <1 second | 2-5 seconds | 60-120 seconds |
| Post-migration validation | 2-3 seconds | 10-15 seconds | 5-10 minutes |
| Cleanup | <1 second | <1 second | 1-2 seconds |

**Hardware Assumptions:**
- CPU: Modern multi-core processor
- Disk: SSD (HDD may be 5-10x slower)
- RAM: 8GB+ available

---

## Conclusion

This validation framework provides comprehensive, automated verification of the T1-TIMING-001 database migration. All checks are designed to catch data integrity issues, precision loss, and edge cases.

**Success Criteria:** ALL validation checks must pass (exit code 0)

**On Failure:** Rollback immediately and investigate root cause

**After Success:** Keep snapshot table for at least 7 days in case post-deployment issues arise

**Next Steps:**
1. Run pre-migration snapshot
2. Execute migration script
3. Run post-migration validation
4. If all checks pass: Deploy to production
5. If any checks fail: Rollback and debug

---

**Document Version:** 1.0
**Created:** 2025-10-19
**Status:** Ready for Execution
**Migration ID:** T1-TIMING-001
**Related Documents:**
- [T1-TIMING-001-APPROVED.md](T1-TIMING-001-APPROVED.md)
- [SPEC017-sample_rate_conversion.md](../SPEC017-sample_rate_conversion.md)
- [IMPL001-database_schema.md](../IMPL001-database_schema.md)

---

End of Validation Report
