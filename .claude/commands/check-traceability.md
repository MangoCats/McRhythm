# Requirement Traceability Validator

**Purpose:** Validate requirement ID traceability between code, tests, and specifications

**Task:** Perform comprehensive requirement traceability validation for the WKMP project.

---

## Instructions

You are validating requirement traceability across the WKMP codebase. Follow these steps systematically:

### Step 1: Catalog All Requirements

Extract requirement IDs from specification documents:

1. Read docs/REQ001-requirements.md (summary only, 50 lines)
2. Identify which specification documents to scan based on summary
3. Search for all requirement IDs matching pattern: `[A-Z]+-[A-Z]+-[0-9]+`
   - Examples: REQ-CF-010, ARCH-VOL-010, XFD-CURVE-020
4. Build master list of all documented requirements

**Output:** List of all requirement IDs found in specifications

### Step 2: Scan Code for Requirement References

Search all Rust source files for requirement ID citations:

1. Use Grep to search for requirement ID patterns in:
   - `wkmp-ap/**/*.rs`
   - `wkmp-ui/**/*.rs`
   - `wkmp-pd/**/*.rs`
   - `wkmp-ai/**/*.rs`
   - `wkmp-le/**/*.rs`
   - `common/**/*.rs`

2. For each requirement ID found:
   - Record file path and line number
   - Extract surrounding context (3 lines before/after)
   - Identify if it's in code, comment, or test

**Output:** Map of requirement IDs → code locations

### Step 3: Scan Tests for Requirement Coverage

Search all test files for requirement ID citations:

1. Use Grep to search test files (pattern: `**/tests/**/*.rs`, `**/*_test.rs`, files with `#[test]`)
2. For each requirement ID in tests:
   - Record test function name
   - Record test file location
   - Note if test is unit, integration, or acceptance test

**Output:** Map of requirement IDs → test coverage

### Step 4: Cross-Reference and Identify Gaps

Compare the three datasets:

**For each requirement in specifications:**
- ✅ Has code implementation? → List files
- ✅ Has test coverage? → List tests
- ❌ Missing implementation? → Flag as gap
- ❌ Missing tests? → Flag as gap
- ⚠️ Orphaned (in code but not in specs)? → Flag as potential error

### Step 5: Generate Traceability Report

Create comprehensive report with:

#### Summary Statistics
- Total requirements documented: X
- Requirements with code implementation: X (Y%)
- Requirements with test coverage: X (Y%)
- Fully traced requirements (spec + code + test): X (Y%)
- Orphaned code references: X

#### Coverage by Category
Group by requirement prefix (REQ-CF, ARCH-VOL, etc.):
- Show coverage % per category
- Highlight gaps by category

#### Gap Analysis

**Critical Gaps (P0):**
- Requirements missing both code AND tests

**Missing Tests (P1):**
- Requirements with code but no test coverage

**Missing Implementation (P1):**
- Requirements with tests but no implementation

**Orphaned References (P2):**
- Requirement IDs in code that don't exist in specs
- Possible typos or outdated references

#### Detailed Traceability Matrix

Table format:
```
| Req ID | Spec Doc | Code Files | Test Files | Status |
|--------|----------|------------|------------|--------|
| REQ-CF-010 | REQ001:340 | wkmp-ap/crossfade.rs:45 | tests/crossfade_test.rs:120 | ✅ Complete |
| REQ-CF-015 | REQ001:365 | - | - | ❌ Not Implemented |
```

### Step 6: Actionable Recommendations

Based on gaps found, provide:

1. **Immediate Actions** (P0 gaps):
   - List specific requirements needing urgent attention
   - Recommend implementation order

2. **Short-term Actions** (P1 gaps):
   - Missing test coverage items
   - Missing implementation items

3. **Cleanup Actions** (P2):
   - Orphaned references to investigate/remove
   - Documentation updates needed

---

## Output Format

Generate a report file: `wip/traceability_report_YYYY-MM-DD.md`

Include:
- Executive summary (10 lines max)
- Summary statistics table
- Coverage by category table
- Gap analysis (categorized by priority)
- Full traceability matrix (appendix)
- Actionable recommendations

**Display to user:**
- Executive summary
- Summary statistics
- Top 5 critical gaps
- Link to full report file

---

## Performance Optimization

- Use parallel Grep calls when searching multiple directories
- Use `files_with_matches` mode first, then targeted `content` mode
- Limit initial spec document reads to summaries only
- Cache requirement ID list to avoid re-parsing

---

## Error Handling

If errors occur:
- **Spec not found:** Report which specs are missing, continue with available
- **No matches found:** Report zero coverage, continue validation
- **Invalid requirement ID format:** Log warning, skip invalid ID

---

## Success Criteria

✅ All specification documents scanned
✅ All Rust source files scanned
✅ All test files scanned
✅ Traceability report generated
✅ Gaps identified with priority levels
✅ Actionable recommendations provided

---

**Expected runtime:** 2-4 minutes for full codebase scan
**Output file:** `wip/traceability_report_YYYY-MM-DD.md`
