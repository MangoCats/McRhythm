# WKMP Custom Commands

**Purpose:** Command definitions for WKMP Music Player development workflows
**Tool:** Claude Code (AI-assisted development environment)
**Last Updated:** 2025-10-28

---

## Overview

This directory contains custom command definitions for the WKMP (Auto DJ Music Player) project workspace. These workflows provide automated support for commits, documentation, analysis, planning, and archival operations.

**Workflow Philosophy:**
- **Separation of Concerns:** Analysis (`/think`) separate from planning (`/plan`), planning separate from implementation
- **Traceability:** Every change tracked in `change_history.md` via `/commit`
- **Context Window Optimization:** Modular output (<500 lines per section), archival reduces active workspace
- **Specification-First:** `/plan` verifies specs before implementation begins
- **Non-Destructive Archival:** `/archive` preserves full git history while reducing clutter

---

## Available Commands

### Core Workflow Commands

#### /commit - Multi-Step Commit with Change History Tracking

**Purpose:** Perform consistent git commits with automatic change history tracking and archive branch synchronization

**Usage:**
```bash
/commit
```

**When to Use:**
- After implementing a feature or bug fix
- After updating documentation
- After applying `/doc-name` or `/archive` operations
- **Always use `/commit` instead of manual `git commit`** for consistency

**What It Does:**
- Validates git repository state and remote updates
- Stages all changes in repository
- Generates comprehensive change summary
- Automatically updates `project_management/change_history.md`
- Creates commit with condensed summary message
- Adds commit hash to change history (one-commit-lag system)
- Synchronizes archive branch automatically
- Prompts to push both working and archive branches

**What It Does NOT Do:**
- Does not commit without user confirmation
- Does not skip validation steps
- Does not require manual change history editing

**One-Commit-Lag Hash System:**
- Commit N: change_history.md entry added WITHOUT hash
- Immediately after: Hash of Commit N added to entry and staged
- Commit N+1: Includes hash of Commit N
- Result: Complete traceability without git hooks

**Success Criteria:**
- All repository changes committed
- change_history.md updated with entry
- Previous entry hash added (if missing)
- Current commit hash staged for next commit
- Archive branch synchronized
- Working tree clean except staged hash

---

#### /doc-name - Document Prefix Assignment

**Purpose:** Assign alpha-numeric prefix (CAT###_) to documents following WKMP documentation governance

**Usage:**
```bash
/doc-name path/to/document.md
```

**When to Use:**
- New document created and ready for permanent naming
- Document moved from `wip/` to permanent location
- Need to assign standard prefix for cross-referencing

**What It Does:**
- Analyzes document location, name, and content
- Recommends appropriate category from WKMP's 13-category system
- Gets next available number from `workflows/REG001_number_registry.md`
- Renames file using `CAT###_original_name.md` format
- Updates number registry
- Stages changes for `/commit`

**What It Does NOT Do:**
- Does not rename exempted files (README.md, CLAUDE.md, change_history.md, .claude/commands/*)
- Does not automatically update references (user must update manually)
- Does not commit changes (stages only - use `/commit` after)

**Category System (13 Categories):**

**Product Documentation (docs/):**
- **GOV###** - Governance (document hierarchy, conventions)
- **REQ###** - Requirements (authoritative requirements)
- **SPEC###** - Specifications (design specs, API design)
- **IMPL###** - Implementation (database schema, coding conventions)
- **EXEC###** - Execution (implementation order)
- **REV###** - Reviews (design reviews)
- **GUIDE###** - Guides (implementation guides)

**Analysis & Planning (docs/ or wip/):**
- **RPT###** - Reports & Analysis (/think outputs, investigations)
- **PLAN###** - Implementation Plans (/plan outputs, test specs)

**Workflow Documentation (workflows/):**
- **DWI###** - Developer Work Instructions (workflow procedures)
- **TMPL###** - Templates (reusable templates)

**Cross-Cutting:**
- **LOG###** - Operational Logs (project_management/)
- **REG###** - Registries (workflows/)

**Example:**
```bash
/doc-name wip/crossfade_analysis.md
# Becomes: wip/RPT001_crossfade_analysis.md
```

---

#### /think - Multi-Agent Document Analysis

**Purpose:** Analyze documents containing questions, problems, or change requests using dynamic multi-agent strategy

**Usage:**
```bash
/think path/to/analysis_request.md
```

**When to Use:**
- You have questions about project direction or technical approaches
- Need comprehensive analysis of problems with solution options
- Evaluating multiple approaches for a feature or change
- Require research-backed analysis without implementation
- Need detailed comparison of alternatives

**What It Does:**
- Dynamically deploys specialized analysis agents
- Systematically reviews project state and documentation
- Conducts internet research when appropriate
- Formulates comprehensive answers with evidence
- Presents detailed option comparisons
- Provides executive summary
- Permanently records analysis results in project documentation
- Updates input document with summary while preserving original content
- Implements context window management for large documents and analyses

**What It Does NOT Do:**
- Does not create implementation plans (use `/plan` for that)
- Does not generate code or file structures
- Does not make decisions on your behalf
- Does not modify any project files (except adding analysis results to input document)

**Context Window Management:**
- Executive summary: <300 lines (6-minute read)
- Full analysis summary: <500 lines (10-minute read)
- Modular sections: <300 lines each
- Progressive disclosure: Summary first, details on demand

**Example Scenarios:**

1. **Technical Questions:**
   ```bash
   /think wip/audio_architecture_questions.md
   ```
   Use when you have questions about system behavior, architecture decisions, or technical approaches.

2. **Requirements Analysis:**
   ```bash
   /think docs/SPEC007-api_design.md
   ```
   Use when requirements need clarification, conflict resolution, or implementation approach analysis.

3. **Problem Investigation:**
   ```bash
   /think wip/crossfade_timing_issues.md
   ```
   Use when you've documented problems that need root cause analysis and solution options.

**Output:** Creates `docs/RPT###_<original_name>_analysis_results.md`

---

#### /plan - Implementation Planning Workflow

**Purpose:** Create systematic, specification-driven implementation plans that maximize probability of meeting requirements on first attempt

**Usage:**
```bash
/plan path/to/specification.md
```

**When to Use:**
- You have requirements or design specifications ready for implementation
- Need to verify specifications are complete before coding
- Want to define acceptance tests before implementation begins
- Need structured implementation plan with small, verifiable increments

**What It Does:**
- Extracts and catalogs all requirements from specifications
- Verifies specification completeness (finds ambiguities, gaps, conflicts)
- Defines explicit acceptance tests for every requirement (test-first approach)
- Creates modular, context-window-optimized plan structure
- Automatically integrates `/think` for complex specifications
- Presents specification issues with severity levels
- Generates traceability matrix (requirements ↔ tests)
- Produces implementation-ready plans with clear increments

**What It Does NOT Do:**
- Does not write code or begin implementation
- Does not define requirements (works with specifications provided)
- Does not modify specification documents
- Does not make architectural decisions without evaluation

**Context Window Optimization:**
- Plan summary: <500 lines (start here)
- Per-increment files: <300 lines (read one at a time)
- Test specifications: Modular (read only relevant tests)
- Read only what's needed for current work

**Output:** Creates `wip/PLAN###_<name>/` folder with:
- `00_PLAN_SUMMARY.md` - Read this first
- `01_specification_issues.md` - Gaps/conflicts found
- `requirements_index.md` - All requirements cataloged
- `02_test_specifications/` - Acceptance tests
- `traceability_matrix.md` - Requirements ↔ Tests mapping

**Current Status:**
- ✅ **Week 1 Delivered:** Phases 1-3 (spec verification, test definition, context optimization)
- 🚧 **Week 2 In Progress:** Phases 4-5 (approach selection, implementation breakdown)
- 🚧 **Week 3 Planned:** Phases 6-7 (risks, estimates, schedule)

**Example:**
```bash
/plan docs/SPEC002-crossfade.md
```

---

#### /archive - Document Archival

**Purpose:** Move historical/obsolete documents to git archive branch while preserving full history

**Usage:**
```bash
/archive path/to/document.md "reason for archival"
```

**When to Use:**
- Analysis complete AND decision implemented
- Planning complete AND project finished
- Document superseded by newer version
- Content no longer relevant to active work

**What It Does:**
- Syncs document to archive branch first (safety)
- Removes document from working branch via `git rm`
- Updates `workflows/REG002_archive_index.md` with retrieval commands
- Preserves full git history
- Maintains clean working tree (only active docs visible)

**What It Does NOT Do:**
- Does not delete documents (they remain in archive branch)
- Does not automatically update references (user must update manually)
- Does not commit changes (stages only - use `/commit` after)

**Retrieval:**
```bash
# View archived document
git show archive:docs/RPT001_old_analysis.md

# Restore archived document
git show archive:docs/RPT001_old_analysis.md > RPT001_old_analysis.md

# Browse archive branch
git checkout archive
# Explore files
git checkout dev  # Return to working branch
```

**Archive Benefits:**
- Clean working tree (40-60% context reduction)
- Full preservation (nothing truly deleted)
- Easy retrieval via git commands
- History intact

**Example:**
```bash
/archive docs/RPT001_crossfade_analysis.md "Analysis complete, SPEC002 implemented"
```

---

#### /archive-plan - Batch Archive Plan Documents

**Purpose:** Batch archive all work-in-progress documents for a completed implementation plan

**Usage:**
```bash
/archive-plan PLAN###
```

**When to Use:**
- Plan implementation complete (all tests pass, all requirements met)
- All PLAN### WIP documents ready for archival
- Want to clean up completed work efficiently

**What It Does:**
- Identifies all documents matching plan number:
  - Individual files: `wip/plan###_*.md`
  - Plan folder: `wip/PLAN###_*/`
- Syncs all to archive branch (safety first)
- Removes all from working branch
- Updates archive index with retrieval commands for all
- Stages changes for `/commit`

**What It Does NOT Do:**
- Does not archive partial plans (all-or-nothing per plan number)
- Does not automatically update references
- Does not commit changes (stages only - use `/commit` after)

**Efficiency Benefit:**
- Archive 6+ documents with one command
- vs. six separate `/archive` commands

**When to Archive Plan:**
- ✅ All plan requirements implemented
- ✅ All acceptance tests pass
- ✅ Plan folder no longer needed for active work
- ✅ Implementation complete and verified

**Example:**
```bash
/archive-plan PLAN002
# Archives all wip/plan002_*.md files and wip/PLAN002_*/ folder
```

---

### Validation & Quality Commands

#### /check-traceability - Requirement Traceability Validator

**Purpose:** Validate requirement ID traceability between code, tests, and specifications

**Usage:**
```bash
/check-traceability
```

**When to Use:**
- Before major releases
- Monthly during active development
- After implementing new features
- When validating requirement coverage

**What It Does:**
- Extracts all requirement IDs from specification documents (REQ-CF-010, ARCH-VOL-010, etc.)
- Scans all Rust source files for requirement ID citations
- Scans all test files for requirement coverage
- Cross-references specs, code, and tests
- Identifies gaps: missing implementations, missing tests, orphaned references
- Generates comprehensive traceability matrix
- Provides actionable recommendations by priority

**What It Does NOT Do:**
- Does not modify any files
- Does not generate tests or code
- Does not fix gaps automatically

**Output:** Creates `wip/traceability_report_YYYY-MM-DD.md`

**Success Metrics:**
- Total requirements documented
- Code implementation coverage %
- Test coverage %
- Fully traced requirements (spec + code + test) %
- Critical gaps identified

**Example Output:**
```
Traceability Report Summary:
✅ Requirements with code: 45/50 (90%)
✅ Requirements with tests: 42/50 (84%)
✅ Fully traced: 40/50 (80%)
❌ Critical gaps: 5 requirements missing both code and tests

See wip/traceability_report_2025-10-28.md
```

---

#### /check-all - Rust Multi-Crate Workflow

**Purpose:** Run comprehensive quality checks across all WKMP workspace crates

**Usage:**
```bash
/check-all
```

**When to Use:**
- Before every commit
- After significant changes
- Before pull requests
- During continuous integration

**What It Does:**
- Runs `cargo fmt --check` (formatting validation)
- Runs `cargo clippy` (linting across all crates)
- Runs `cargo build --workspace` (compilation check)
- Runs `cargo test --workspace` (all tests)
- Runs `cargo doc` (documentation validation)
- Reports results concisely with actionable next steps
- Auto-fixes formatting issues when found
- Provides build/test time metrics

**What It Does NOT Do:**
- Does not fix clippy warnings automatically
- Does not modify code (except auto-format)
- Does not run benchmarks (unless requested)

**Execution Strategy:**
- Parallel execution where possible (fmt, clippy, doc in parallel)
- Sequential for dependencies (build before test)
- Early exit on critical failures
- Leverages Cargo incremental compilation

**Output Format:**
```
🔍 WKMP Multi-Crate Quality Check

✅ Format:  All files formatted correctly
⚠️  Clippy:  3 warnings in wkmp-ap, 1 in wkmp-ui
✅ Build:   All 6 crates built (12.3s)
✅ Tests:   95 tests passed (3.7s)
✅ Docs:    Documentation built (2 warnings)

Overall: 4/5 passed, 1 with warnings
```

**Variants:**
- Quick check: Skip doc build for rapid iteration
- Full check: Include release build and benchmarks

**Expected Runtime:**
- Quick: 10-20s
- Standard: 25-60s
- Full: 60-120s

---

#### /check-docs - Document Hierarchy Checker

**Purpose:** Validate WKMP's 5-tier documentation hierarchy for consistency and governance compliance

**Usage:**
```bash
/check-docs
```

**When to Use:**
- Before major releases
- After bulk documentation updates
- Monthly during active development
- When governance rules change

**What It Does:**
- Catalogs all documentation (docs/, workflows/, wip/, project_management/)
- Extracts document references (links, citations)
- Validates information flow direction (higher tier → lower tier allowed, reverse flagged)
- Detects circular references (A → B → A)
- Identifies orphaned documents (no incoming references)
- Validates document number sequences against REG001 registry
- Checks category consistency (correct prefixes in correct locations)
- Generates detailed hierarchy validation report

**What It Does NOT Do:**
- Does not modify documents
- Does not fix circular references automatically
- Does not archive orphaned documents (suggests archival)

**Governance Rules Validated:**
- GOV001: 5-tier hierarchy (Governance → Requirements → Design → Implementation → Execution)
- REG001: Number registry consistency
- REG002: Archive tracking
- REG003: Category definitions (13 categories)

**Output:** Creates `wip/doc_hierarchy_validation_YYYY-MM-DD.md`

**Critical Issues Detected:**
- Circular references (must fix)
- Invalid tier jumps (EXEC → REQ skipping intermediate tiers)
- Broken references (document not found)

**Warnings:**
- Upward references (lower tier → higher tier, needs review)
- Orphaned documents (candidates for archival)
- Number gaps (missing in sequence)

**Example Output:**
```
🗂️  Document Hierarchy Validation

Documents scanned: 47
✅ Valid references: 234 (92%)
⚠️  Upward references: 15 (6%) - review
❌ Circular references: 2 (CRITICAL)
⚠️  Orphaned documents: 8

See wip/doc_hierarchy_validation_2025-10-28.md
```

**Expected Runtime:** 30-90 seconds

---

#### /check-api - API Contract Validator

**Purpose:** Validate API implementations against SPEC007 API design specifications

**Usage:**
```bash
/check-api
```

**When to Use:**
- Before commits that modify APIs
- Weekly for continuous monitoring
- Before pull requests
- After updating SPEC007

**What It Does:**
- Extracts all API endpoint specifications from SPEC007-api_design.md
- Scans all 5 microservices for Axum route definitions
- Analyzes handler function signatures and types
- Compares specification vs implementation:
  - Endpoint existence
  - HTTP method matching
  - Path pattern matching
  - Request schema matching
  - Response schema matching
  - Status code documentation
- Validates error handling consistency
- Checks authentication/authorization requirements
- Validates SSE endpoints
- Generates compliance report with severity levels

**What It Does NOT Do:**
- Does not modify code or specifications
- Does not generate API implementations
- Does not auto-fix contract violations

**Services Validated:**
- wkmp-ap (Audio Player, port 5721)
- wkmp-ui (User Interface, port 5720)
- wkmp-pd (Program Director, port 5722)
- wkmp-ai (Audio Ingest, port 5723)
- wkmp-le (Lyric Editor, port 5724)

**Output:** Creates `wip/api_contract_validation_YYYY-MM-DD.md`

**Violation Severities:**
- **HIGH:** Breaking changes (response schema mismatch, missing required fields)
- **MEDIUM:** Missing functionality (endpoint documented but not implemented)
- **LOW:** Backward compatible issues (extra fields, undocumented status codes)

**Example Output:**
```
🔌 API Contract Validation

Services scanned: 5
Endpoints in spec: 42
Overall compliance: 90% (38/42)

❌ Contract violations: 3
  - wkmp-ap POST /queue/enqueue: Response missing field
  - wkmp-pd GET /select/next: Schema mismatch
  - wkmp-ui DELETE /passages/:id: Not implemented

See wip/api_contract_validation_2025-10-28.md
```

**Expected Runtime:** 45-120 seconds

---

## Workflow Integration Pattern

**Typical Development Flow:**

```
1. Question arises
   ↓
2. /think wip/question.md → Analyze and decide approach
   ↓
3. Write or update specification with chosen approach
   ↓
4. /plan docs/SPEC###.md → Verify spec, define tests, create implementation plan
   ↓
5. Review specification issues → Fix critical gaps
   ↓
6. Implement increment 1 (follow plan)
   ↓
7. /check-all → Validate formatting, linting, tests before commit
   ↓
8. /commit → Track changes
   ↓
9. Verify tests pass for increment 1
   ↓
10. Repeat steps 6-9 for remaining increments
    ↓
11. All increments complete, all tests pass
    ↓
12. /check-traceability → Verify requirement coverage
    ↓
13. /doc-name wip/new_document.md → Assign prefixes to new docs
    ↓
14. /check-docs → Validate documentation hierarchy
    ↓
15. /commit → Track documentation updates
    ↓
16. /archive-plan PLAN### → Clean up completed plan
    ↓
17. /commit → Track archival
```

**Quality Validation Points:**

```
Before every commit:
  /check-all → Ensures code quality (fmt, clippy, tests)

After API changes:
  /check-api → Validates API contract compliance

Monthly or before releases:
  /check-traceability → Verifies requirement coverage
  /check-docs → Validates documentation governance
```

---

## Command Activation

### For Claude Code

These command definitions are automatically available when:
1. Files are present in `.claude/commands/` directory
2. You invoke the command by name in conversation

**Invocation Methods:**

**Method 1: Direct Slash Command**
```
# Core workflow commands
/commit
/doc-name wip/analysis.md
/think wip/question.md
/plan docs/SPEC002-crossfade.md
/archive docs/RPT001_old_analysis.md "reason"
/archive-plan PLAN002

# Validation & quality commands
/check-all
/check-traceability
/check-docs
/check-api
```

**Method 2: Reference in Conversation**
```
Please use the /commit workflow to commit these changes
```

**Method 3: Via CLAUDE.md Reference**
The CLAUDE.md file already references these workflows in agent guidance.

---

## Best Practices

### Do's ✅

**Core Workflows:**
- **Always use `/commit`** for all commits (maintains consistency and change history)
- **Use `/doc-name`** before moving documents out of `wip/` (establishes references)
- **Use `/think` before major architectural decisions** (evidence-based choices)
- **Use `/plan` before implementing new features** (test-first, spec verification)
- **Archive completed work** regularly (context window optimization)
- **Read plan summaries first** before diving into details (progressive disclosure)
- **Review specification issues** from `/plan` before coding (catch gaps early)

**Quality & Validation:**
- **Run `/check-all` before every commit** (catches formatting, linting, test failures early)
- **Run `/check-api` after modifying endpoints** (prevents contract violations)
- **Run `/check-traceability` monthly or before releases** (ensures requirement coverage)
- **Run `/check-docs` after bulk documentation updates** (maintains hierarchy integrity)
- **Address CRITICAL issues immediately** from validation reports (prevents compounding problems)
- **Review warnings regularly** even if not blocking (technical debt prevention)

### Don'ts ❌

**Core Workflows:**
- **Don't manually edit `change_history.md`** (use `/commit` workflow exclusively)
- **Don't skip `/think` for complex decisions** (prevents costly rework)
- **Don't implement without `/plan`** for non-trivial features (risk missing requirements)
- **Don't archive actively referenced documents** (breaks current work)
- **Don't commit without staging** (use `/commit` which handles staging)
- **Don't rename documents manually** (use `/doc-name` for traceability)

**Quality & Validation:**
- **Don't skip `/check-all` to save time** (technical debt accumulates quickly)
- **Don't ignore validation warnings** (low-severity issues become high-severity over time)
- **Don't commit code with failing tests** (breaks CI/CD and other developers)
- **Don't change APIs without running `/check-api`** (breaks client contracts)
- **Don't assume requirements are covered** (run `/check-traceability` to verify)

---

## File Locations

### Created/Modified by Workflows

**By /commit:**
- `project_management/change_history.md` - Automatic change tracking

**By /doc-name:**
- `workflows/REG001_number_registry.md` - Document number tracking
- Renames target document with CAT### prefix

**By /think:**
- `docs/RPT###_<name>_analysis_results.md` - Analysis outputs
- Updates input document with executive summary

**By /plan:**
- `wip/PLAN###_<name>/` - Plan folder with modular structure
- Multiple files within plan folder

**By /archive:**
- `workflows/REG002_archive_index.md` - Archive retrieval index
- Removes document from working branch
- Syncs to archive branch

**By /archive-plan:**
- Same as /archive but for multiple plan documents

**By /check-all:**
- No files created (console output only)
- May auto-fix formatting via `cargo fmt`

**By /check-traceability:**
- `wip/traceability_report_YYYY-MM-DD.md` - Traceability validation report

**By /check-docs:**
- `wip/doc_hierarchy_validation_YYYY-MM-DD.md` - Hierarchy validation report

**By /check-api:**
- `wip/api_contract_validation_YYYY-MM-DD.md` - API contract validation report

---

## Related Project Files

- **Project Root:** [CLAUDE.md](file:///c%3A/Users/Mango%20Cat/Dev/McRhythm/CLAUDE.md) - AI instructions and agent guidance
- **Documentation:** [docs/](file:///c%3A/Users/Mango%20Cat/Dev/McRhythm/docs/) - All technical documentation
- **Document Hierarchy:** [docs/GOV001-document_hierarchy.md](file:///c%3A/Users/Mango%20Cat/Dev/McRhythm/docs/GOV001-document_hierarchy.md) - 5-tier framework
- **Requirements:** [docs/REQ001-requirements.md](file:///c%3A/Users/Mango%20Cat/Dev/McRhythm/docs/REQ001-requirements.md) - Complete feature specifications
- **Change History:** [project_management/change_history.md](file:///c%3A/Users/Mango%20Cat/Dev/McRhythm/project_management/change_history.md) - Full audit trail
- **Workflow Quickstart:** [workflows/DWI001_workflow_quickstart.md](file:///c%3A/Users/Mango%20Cat/Dev/McRhythm/workflows/DWI001_workflow_quickstart.md) - Quick reference guide

---

## Git Branch Strategy

WKMP uses three primary branches:

- **`main`** - Stable releases (mirror of dev at milestones)
- **`dev`** - Active development (working branch)
- **`archive`** - Historical documents only (context optimization)

**Archive Branch Benefits:**
- 40-60% context window reduction
- Automatic exclusion from AI tools
- Clean file explorer (only active documents visible)
- Targeted grep/search (doesn't hit archived content)
- Full preservation (nothing truly deleted)

---

## Troubleshooting

**Problem: "Not a git repository"**
- Solution: Navigate to repository root (`c:\Users\Mango Cat\Dev\McRhythm`) before running commands

**Problem: "Merge conflict in change_history.md"**
- Solution: Keep entries in chronological order, preserve all entries from both branches

**Problem: "Archive branch doesn't exist"**
- Solution: Create archive branch first: `git checkout -b archive dev`

**Problem: "Cannot find document to archive"**
- Solution: Verify document path is correct relative to repository root

**Problem: "No changes to commit"**
- Solution: Make changes first, or check `git status` to see current state

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-10-25 | Initial workflow adoption from Cursor AI project |

---

**Maintained by:** WKMP Development Team
**Tool:** Claude Code
**Workflow Source:** Adapted from proven Cursor AI workflows with WKMP-specific customizations
