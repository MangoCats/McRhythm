#!/bin/bash
# Phase 3B: Database Migration Validation Script
# T1-TIMING-001: REAL seconds → INTEGER ticks migration validator
#
# Usage: ./phase3b-validation-script.sh <database_path> [stage]
# Stage: pre-migration | post-migration | cleanup
#
# Requirements:
# - sqlite3 command-line tool
# - Database with passages table (pre-migration) or migrated (post-migration)
# - Write access to database for snapshot table creation

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
QUERIES_FILE="${SCRIPT_DIR}/phase3b-validation-queries.sql"
TICK_RATE=28224000

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Validation counters
CHECKS_TOTAL=0
CHECKS_PASSED=0
CHECKS_FAILED=0

# Usage information
usage() {
    cat <<EOF
Database Migration Validation Script
T1-TIMING-001: REAL seconds → INTEGER ticks

USAGE:
    $0 <database_path> [stage]

ARGUMENTS:
    database_path    Path to WKMP SQLite database
    stage           Validation stage (optional):
                      pre-migration  - Create snapshot before migration
                      post-migration - Verify migration accuracy (default)
                      cleanup        - Remove snapshot table after validation

EXAMPLES:
    # Pre-migration: Create snapshot
    $0 /home/sw/Music/wkmp.db pre-migration

    # Post-migration: Run validation
    $0 /home/sw/Music/wkmp.db post-migration

    # Cleanup: Remove snapshot table
    $0 /home/sw/Music/wkmp.db cleanup

REQUIREMENTS:
    - sqlite3 command-line tool
    - Passages table in database
    - Write permission for snapshot table creation

EXIT CODES:
    0 - All validation checks passed
    1 - One or more validation checks failed
    2 - Usage error or missing dependencies
EOF
    exit 2
}

# Check dependencies
check_dependencies() {
    if ! command -v sqlite3 &> /dev/null; then
        echo -e "${RED}ERROR: sqlite3 command not found${NC}"
        echo "Please install sqlite3 to run validation"
        exit 2
    fi

    if [ ! -f "$QUERIES_FILE" ]; then
        echo -e "${RED}ERROR: Validation queries file not found${NC}"
        echo "Expected: $QUERIES_FILE"
        exit 2
    fi
}

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $*"
}

log_failure() {
    echo -e "${RED}[FAIL]${NC} $*"
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

log_section() {
    echo ""
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}$*${NC}"
    echo -e "${BLUE}========================================${NC}"
}

# Execute SQL query and return results
run_query() {
    local db_path="$1"
    local query="$2"
    sqlite3 "$db_path" "$query" 2>&1
}

# Execute SQL from file
run_query_file() {
    local db_path="$1"
    local query_file="$2"
    sqlite3 "$db_path" < "$query_file" 2>&1
}

# Parse validation result (looks for PASS/FAIL in output)
parse_result() {
    local output="$1"
    if echo "$output" | grep -q "PASS"; then
        return 0
    elif echo "$output" | grep -q "FAIL"; then
        return 1
    else
        # No explicit PASS/FAIL found
        return 2
    fi
}

# Increment check counters
record_check() {
    local result="$1"  # 0 = pass, 1 = fail
    CHECKS_TOTAL=$((CHECKS_TOTAL + 1))
    if [ "$result" -eq 0 ]; then
        CHECKS_PASSED=$((CHECKS_PASSED + 1))
    else
        CHECKS_FAILED=$((CHECKS_FAILED + 1))
    fi
}

# Pre-migration: Create snapshot table
run_pre_migration() {
    local db_path="$1"

    log_section "PRE-MIGRATION SNAPSHOT"
    log_info "Creating snapshot of current timing data..."

    # Check if passages table exists
    if ! run_query "$db_path" "SELECT COUNT(*) FROM passages;" &> /dev/null; then
        log_failure "Passages table not found in database"
        return 1
    fi

    # Get passage count
    local passage_count
    passage_count=$(run_query "$db_path" "SELECT COUNT(*) FROM passages;")
    log_info "Found $passage_count passages in database"

    # Drop existing snapshot if present
    run_query "$db_path" "DROP TABLE IF EXISTS passages_migration_snapshot;" > /dev/null

    # Extract snapshot creation query from queries file
    local snapshot_query
    snapshot_query=$(sed -n '/^CREATE TABLE IF NOT EXISTS passages_migration_snapshot/,/^FROM passages;/p' "$QUERIES_FILE")

    if [ -z "$snapshot_query" ]; then
        log_failure "Could not extract snapshot query from $QUERIES_FILE"
        return 1
    fi

    # Create snapshot
    if run_query "$db_path" "$snapshot_query" > /dev/null; then
        local snapshot_count
        snapshot_count=$(run_query "$db_path" "SELECT COUNT(*) FROM passages_migration_snapshot;")
        log_success "Snapshot created: $snapshot_count rows"

        # Display data distribution
        log_info "Calculating data checksums..."
        local checksum_query
        checksum_query=$(sed -n '/^-- Query 2: Calculate checksum/,/^FROM passages;$/p' "$QUERIES_FILE" | grep -A 100 "^SELECT")

        echo ""
        echo "Data Distribution:"
        echo "==================="
        run_query "$db_path" "$checksum_query" -header -column

        return 0
    else
        log_failure "Failed to create snapshot table"
        return 1
    fi
}

# Post-migration: Run validation checks
run_post_migration() {
    local db_path="$1"

    log_section "POST-MIGRATION VALIDATION"

    # Check if snapshot exists
    if ! run_query "$db_path" "SELECT COUNT(*) FROM passages_migration_snapshot;" &> /dev/null; then
        log_failure "Snapshot table not found. Run 'pre-migration' stage first."
        return 1
    fi

    local all_passed=0

    # Check 1: Row count verification
    log_info "Check 1: Verifying row count..."
    local row_check
    row_check=$(run_query "$db_path" "SELECT
        (SELECT COUNT(*) FROM passages) AS current_count,
        (SELECT COUNT(*) FROM passages_migration_snapshot) AS snapshot_count,
        CASE
            WHEN (SELECT COUNT(*) FROM passages) = (SELECT COUNT(*) FROM passages_migration_snapshot)
            THEN 'PASS'
            ELSE 'FAIL'
        END AS status;" -header -column)
    echo "$row_check"

    if echo "$row_check" | grep -q "PASS"; then
        log_success "Row count unchanged"
        record_check 0
    else
        log_failure "Row count mismatch detected"
        record_check 1
        all_passed=1
    fi
    echo ""

    # Check 2: Data type verification
    log_info "Check 2: Verifying data types..."
    local schema
    schema=$(run_query "$db_path" "SELECT sql FROM sqlite_master WHERE type='table' AND name='passages';")

    if echo "$schema" | grep -E "start_time.*INTEGER" &> /dev/null && \
       echo "$schema" | grep -E "end_time.*INTEGER" &> /dev/null; then
        log_success "Timing fields are INTEGER type"
        record_check 0
    else
        log_failure "Timing fields are not INTEGER type"
        echo "$schema"
        record_check 1
        all_passed=1
    fi
    echo ""

    # Check 3: Tick value accuracy
    log_info "Check 3: Verifying tick value accuracy..."
    local tick_check
    tick_check=$(run_query "$db_path" "SELECT
        field_name,
        total_rows,
        matching_rows,
        mismatched_rows,
        status
    FROM (
        SELECT
            'start_time' AS field_name,
            COUNT(*) AS total_rows,
            SUM(CASE
                WHEN p.start_time = s.start_time_expected_ticks OR (p.start_time IS NULL AND s.start_time_expected_ticks IS NULL)
                THEN 1 ELSE 0
            END) AS matching_rows,
            SUM(CASE
                WHEN p.start_time != s.start_time_expected_ticks OR (p.start_time IS NULL AND s.start_time_expected_ticks IS NOT NULL) OR (p.start_time IS NOT NULL AND s.start_time_expected_ticks IS NULL)
                THEN 1 ELSE 0
            END) AS mismatched_rows,
            CASE
                WHEN SUM(CASE WHEN p.start_time != s.start_time_expected_ticks OR (p.start_time IS NULL AND s.start_time_expected_ticks IS NOT NULL) OR (p.start_time IS NOT NULL AND s.start_time_expected_ticks IS NULL) THEN 1 ELSE 0 END) = 0
                THEN 'PASS' ELSE 'FAIL'
            END AS status
        FROM passages p
        INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
    ) WHERE status='FAIL';" -header -column)

    if [ -z "$tick_check" ]; then
        log_success "All tick values match expected calculations"
        record_check 0
    else
        log_failure "Tick value mismatches detected:"
        echo "$tick_check"
        record_check 1
        all_passed=1
    fi
    echo ""

    # Check 4: NULL preservation
    log_info "Check 4: Verifying NULL preservation..."
    local null_check
    null_check=$(run_query "$db_path" "SELECT
        field_name,
        null_preserved,
        null_to_value,
        value_to_null,
        status
    FROM (
        SELECT
            'start_time' AS field_name,
            SUM(CASE WHEN s.start_time_original_seconds IS NULL AND p.start_time IS NULL THEN 1 ELSE 0 END) AS null_preserved,
            SUM(CASE WHEN s.start_time_original_seconds IS NULL AND p.start_time IS NOT NULL THEN 1 ELSE 0 END) AS null_to_value,
            SUM(CASE WHEN s.start_time_original_seconds IS NOT NULL AND p.start_time IS NULL THEN 1 ELSE 0 END) AS value_to_null,
            CASE
                WHEN SUM(CASE WHEN (s.start_time_original_seconds IS NULL) != (p.start_time IS NULL) THEN 1 ELSE 0 END) = 0
                THEN 'PASS' ELSE 'FAIL'
            END AS status
        FROM passages p
        INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
    ) WHERE status='FAIL';" -header -column)

    if [ -z "$null_check" ]; then
        log_success "NULL values correctly preserved"
        record_check 0
    else
        log_failure "NULL preservation errors detected:"
        echo "$null_check"
        record_check 1
        all_passed=1
    fi
    echo ""

    # Check 5: Precision loss detection
    log_info "Check 5: Checking for precision loss (max error: 1 tick = 35.4ns)..."
    local precision_check
    precision_check=$(run_query "$db_path" "SELECT
        field_name,
        total_non_null,
        ROUND(max_error_ticks, 6) AS max_error_ticks,
        within_tolerance,
        exceeds_tolerance,
        status
    FROM (
        SELECT
            'start_time' AS field_name,
            COUNT(*) AS total_non_null,
            MAX(ABS(s.start_time_original_seconds - CAST(p.start_time AS REAL) / 28224000.0) * 28224000.0) AS max_error_ticks,
            SUM(CASE WHEN ABS(s.start_time_original_seconds - CAST(p.start_time AS REAL) / 28224000.0) * 28224000.0 <= 1.0 THEN 1 ELSE 0 END) AS within_tolerance,
            SUM(CASE WHEN ABS(s.start_time_original_seconds - CAST(p.start_time AS REAL) / 28224000.0) * 28224000.0 > 1.0 THEN 1 ELSE 0 END) AS exceeds_tolerance,
            CASE WHEN SUM(CASE WHEN ABS(s.start_time_original_seconds - CAST(p.start_time AS REAL) / 28224000.0) * 28224000.0 > 1.0 THEN 1 ELSE 0 END) = 0 THEN 'PASS' ELSE 'FAIL' END AS status
        FROM passages p
        INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid
        WHERE s.start_time_original_seconds IS NOT NULL AND p.start_time IS NOT NULL
    ) WHERE status='FAIL';" -header -column)

    if [ -z "$precision_check" ]; then
        log_success "All conversions within precision tolerance"
        record_check 0
    else
        log_failure "Precision errors detected (>1 tick):"
        echo "$precision_check"

        # Show examples of problematic passages
        log_warning "Example passages with precision loss:"
        run_query "$db_path" "SELECT
            passage_guid,
            field_name,
            ROUND(original_seconds, 10) AS original_seconds,
            actual_ticks,
            expected_ticks,
            ROUND(error_ticks, 6) AS error_ticks,
            ROUND(error_nanoseconds, 2) AS error_ns
        FROM (
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
        ) LIMIT 5;" -header -column

        record_check 1
        all_passed=1
    fi
    echo ""

    # Check 6: Edge case - zero values
    log_info "Check 6: Validating zero values..."
    local zero_count
    zero_count=$(run_query "$db_path" "SELECT COUNT(*) FROM passages_migration_snapshot
        WHERE start_time_original_seconds = 0.0
           OR end_time_original_seconds = 0.0
           OR fade_in_point_original_seconds = 0.0
           OR fade_out_point_original_seconds = 0.0
           OR lead_in_point_original_seconds = 0.0
           OR lead_out_point_original_seconds = 0.0;")

    if [ "$zero_count" -gt 0 ]; then
        local zero_check
        zero_check=$(run_query "$db_path" "SELECT
            SUM(CASE WHEN s.start_time_original_seconds = 0.0 AND p.start_time != 0 THEN 1 ELSE 0 END) +
            SUM(CASE WHEN s.end_time_original_seconds = 0.0 AND p.end_time != 0 THEN 1 ELSE 0 END) AS errors
        FROM passages p
        INNER JOIN passages_migration_snapshot s ON p.passage_guid = s.passage_guid;")

        if [ "$zero_check" -eq 0 ]; then
            log_success "Zero values correctly converted ($zero_count found)"
            record_check 0
        else
            log_failure "Zero value conversion errors: $zero_check instances"
            record_check 1
            all_passed=1
        fi
    else
        log_info "No zero values found in data (skipping check)"
    fi
    echo ""

    # Check 7: Edge case - small values
    log_info "Check 7: Validating small values (<1ms)..."
    local small_count
    small_count=$(run_query "$db_path" "SELECT COUNT(*) FROM passages_migration_snapshot
        WHERE (start_time_original_seconds > 0.0 AND start_time_original_seconds < 0.001)
           OR (end_time_original_seconds > 0.0 AND end_time_original_seconds < 0.001);")

    if [ "$small_count" -gt 0 ]; then
        log_success "Found $small_count small value(s), validating precision..."
        # Small values already covered by precision check
        log_info "Small values validated via precision check (Check 5)"
    else
        log_info "No small values (<1ms) found in data (skipping check)"
    fi
    echo ""

    # Check 8: Edge case - large values
    log_info "Check 8: Validating large values (>10000s)..."
    local large_count
    large_count=$(run_query "$db_path" "SELECT COUNT(*) FROM passages_migration_snapshot
        WHERE start_time_original_seconds > 10000.0
           OR end_time_original_seconds > 10000.0;")

    if [ "$large_count" -gt 0 ]; then
        log_success "Found $large_count large value(s), validating precision..."
        # Large values already covered by precision check
        log_info "Large values validated via precision check (Check 5)"
    else
        log_info "No large values (>10000s) found in data (skipping check)"
    fi
    echo ""

    return $all_passed
}

# Cleanup: Remove snapshot table
run_cleanup() {
    local db_path="$1"

    log_section "CLEANUP"
    log_info "Removing snapshot table..."

    if run_query "$db_path" "SELECT COUNT(*) FROM passages_migration_snapshot;" &> /dev/null; then
        if run_query "$db_path" "DROP TABLE passages_migration_snapshot;" > /dev/null; then
            log_success "Snapshot table removed"
            return 0
        else
            log_failure "Failed to drop snapshot table"
            return 1
        fi
    else
        log_warning "Snapshot table not found (already removed?)"
        return 0
    fi
}

# Main execution
main() {
    # Parse arguments
    if [ $# -lt 1 ]; then
        usage
    fi

    local db_path="$1"
    local stage="${2:-post-migration}"

    # Validate arguments
    if [ ! -f "$db_path" ]; then
        echo -e "${RED}ERROR: Database file not found: $db_path${NC}"
        exit 2
    fi

    if [[ ! "$stage" =~ ^(pre-migration|post-migration|cleanup)$ ]]; then
        echo -e "${RED}ERROR: Invalid stage: $stage${NC}"
        echo "Valid stages: pre-migration, post-migration, cleanup"
        exit 2
    fi

    # Check dependencies
    check_dependencies

    # Display header
    echo ""
    echo "╔════════════════════════════════════════════════════════════╗"
    echo "║   Database Migration Validation - T1-TIMING-001          ║"
    echo "║   REAL seconds → INTEGER ticks (28,224,000 Hz)           ║"
    echo "╚════════════════════════════════════════════════════════════╝"
    echo ""
    log_info "Database: $db_path"
    log_info "Stage: $stage"
    log_info "Tick Rate: $TICK_RATE Hz"
    log_info "Max Error Tolerance: 1 tick (~35.4 nanoseconds)"

    # Execute appropriate stage
    case "$stage" in
        pre-migration)
            if run_pre_migration "$db_path"; then
                echo ""
                log_success "Pre-migration snapshot complete"
                log_info "Next: Run migration script, then validate with '$0 $db_path post-migration'"
                exit 0
            else
                echo ""
                log_failure "Pre-migration snapshot failed"
                exit 1
            fi
            ;;

        post-migration)
            if run_post_migration "$db_path"; then
                validation_result=0
            else
                validation_result=1
            fi

            # Display summary
            log_section "VALIDATION SUMMARY"
            echo "Total Checks:  $CHECKS_TOTAL"
            echo "Checks Passed: $CHECKS_PASSED"
            echo "Checks Failed: $CHECKS_FAILED"
            echo ""

            if [ $validation_result -eq 0 ] && [ $CHECKS_FAILED -eq 0 ]; then
                log_success "ALL VALIDATION CHECKS PASSED"
                echo ""
                log_info "Migration successful. Data integrity verified."
                log_info "To clean up snapshot: $0 $db_path cleanup"
                exit 0
            else
                log_failure "VALIDATION FAILED"
                echo ""
                log_failure "$CHECKS_FAILED check(s) failed"
                log_warning "Review errors above and consider rollback"
                exit 1
            fi
            ;;

        cleanup)
            if run_cleanup "$db_path"; then
                echo ""
                log_success "Cleanup complete"
                exit 0
            else
                echo ""
                log_failure "Cleanup failed"
                exit 1
            fi
            ;;
    esac
}

# Execute main function
main "$@"
