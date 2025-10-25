# WKMP Custom Commands

**Purpose:** Command definitions for WKMP Music Player development workflows
**Tool:** Claude Code (AI-assisted development environment)
**Last Updated:** 2025-10-25

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

### /commit - Multi-Step Commit with Change History Tracking

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

### /doc-name - Document Prefix Assignment

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

### /think - Multi-Agent Document Analysis

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

### /plan - Implementation Planning Workflow

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

### /archive - Document Archival

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

### /archive-plan - Batch Archive Plan Documents

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
7. /commit → Track changes
   ↓
8. Verify tests pass for increment 1
   ↓
9. Repeat steps 6-8 for remaining increments
   ↓
10. All increments complete, all tests pass
    ↓
11. /doc-name wip/new_document.md → Assign prefixes to new docs
    ↓
12. /commit → Track documentation updates
    ↓
13. /archive-plan PLAN### → Clean up completed plan
    ↓
14. /commit → Track archival
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
/commit
/doc-name wip/analysis.md
/think wip/question.md
/plan docs/SPEC002-crossfade.md
/archive docs/RPT001_old_analysis.md "reason"
/archive-plan PLAN002
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

- **Always use `/commit`** for all commits (maintains consistency and change history)
- **Use `/doc-name`** before moving documents out of `wip/` (establishes references)
- **Use `/think` before major architectural decisions** (evidence-based choices)
- **Use `/plan` before implementing new features** (test-first, spec verification)
- **Archive completed work** regularly (context window optimization)
- **Read plan summaries first** before diving into details (progressive disclosure)
- **Review specification issues** from `/plan` before coding (catch gaps early)

### Don'ts ❌

- **Don't manually edit `change_history.md`** (use `/commit` workflow exclusively)
- **Don't skip `/think` for complex decisions** (prevents costly rework)
- **Don't implement without `/plan`** for non-trivial features (risk missing requirements)
- **Don't archive actively referenced documents** (breaks current work)
- **Don't commit without staging** (use `/commit` which handles staging)
- **Don't rename documents manually** (use `/doc-name` for traceability)

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
