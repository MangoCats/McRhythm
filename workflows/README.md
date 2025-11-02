# WKMP Development Workflows

**Purpose:** Process documentation and workflow automation for WKMP development

---

## Quick Start

**New to WKMP workflows?** Read **[DWI001_workflow_quickstart.md](DWI001_workflow_quickstart.md)** (5 minutes)

This covers the four core workflows:
- `/commit` - Multi-step commit with automatic change history tracking
- `/think` - Multi-agent analysis for complex architectural decisions
- `/plan` - Specification-driven implementation planning with test-first approach
- `/archive` - Move completed documents to archive branch

---

## Workflow Documents

### Core Workflows

**[DWI001: Workflow Quickstart](DWI001_workflow_quickstart.md)**
- Getting started with automated workflows
- When to use each workflow
- Quick reference guide

### Registries

**[REG001: Number Registry](REG001_number_registry.md)**
- Tracks document numbering across all categories
- Updated automatically by `/doc-name` workflow
- Prevents duplicate document IDs

**[REG002: Archive Index](REG002_archive_index.md)** ⭐ **FREQUENTLY NEEDED**
- **Purpose:** Index of all archived documents (completed plans, analysis, etc.)
- **When to use:**
  - Looking for historical context on completed features
  - Understanding why design decisions were made
  - Finding implementation plans for reference
- **How to retrieve:** `git show archive-branch:<path>` or use `/archive` workflow

**[REG003: Category Definitions](REG003_category_definitions.md)**
- 13-category system for document classification
- Defines prefixes (GOV, REQ, SPEC, IMPL, EXEC, PLAN, etc.)
- Used by `/doc-name` for automatic categorization

### Templates

**[TMPL001: Think Input Template](TMPL001_think_input_template.md)**
- Template for `/think` workflow input
- Structured problem description format
- Ensures consistent analysis quality

---

## Archive System (REG002)

The archive system keeps the working directory clean while preserving all historical documentation.

### What Gets Archived?

- ✅ Completed implementation plans (PLAN###)
- ✅ Completed analysis documents
- ✅ Completed sprint reports
- ✅ Resolved technical debt analysis
- ❌ Active specifications (SPEC###, REQ###)
- ❌ Active implementation guides (IMPL###)
- ❌ Governance documents (GOV###)

### Finding Archived Documents

1. **Check the index:** [REG002_archive_index.md](REG002_archive_index.md)
2. **Retrieve document:** `git show archive-branch:<path>`
3. **Or use workflow:** `/archive list` to search

### Example: Finding a Completed Plan

```bash
# 1. Check REG002_archive_index.md for the path
# 2. Retrieve it
git show archive-branch:wip/PLAN008_sprint3_completion_report.md
```

---

## Workflow Usage Guidelines

### When to Use /commit

**Always** use `/commit` instead of manual git commits:
- Automatically updates `project_management/change_history.md`
- Synchronizes archive branch
- Maintains audit trail
- Generates consistent commit messages

### When to Use /think

Use for **complex architectural decisions** requiring multi-agent analysis:
- Choosing between design alternatives
- Evaluating risk tradeoffs
- Analyzing performance implications
- Assessing technical debt remediation strategies

**Example scenarios:**
- "Should we use marker-based crossfading or property-based crossfading?"
- "What's the best approach for zero-configuration database initialization?"

### When to Use /plan

Use for **non-trivial feature implementation** (>5 requirements or novel/complex):
- Creates specification-driven implementation plan
- Defines acceptance tests
- Generates traceability matrix (100% requirement coverage)
- Automatically integrates `/think` if complexity warrants

**Mandatory for:**
- Features with >5 requirements
- Features involving novel/complex technical elements
- Features affecting multiple microservices

### When to Use /archive

Use to **move completed documents** to archive branch:
- After completing implementation plans
- After resolving technical debt items
- After finishing sprint reports
- When documents are no longer actively referenced

**Batch operation:** Use `/archive-plan` to archive multiple plans at once

---

## Governance Integration

Workflows are documented in the main project documentation:
- **[CLAUDE.md](../CLAUDE.md)** - Overview of all workflows
- **[docs/DEV_QUICKSTART.md](../docs/DEV_QUICKSTART.md)** - Developer onboarding includes workflow intro
- **[docs/GOV001-document_hierarchy.md](../docs/GOV001-document_hierarchy.md)** - How workflows fit into documentation framework

---

## Frequently Asked Questions

### Q: Where did PLAN008 go? It's not in /wip anymore.

**A:** Check [REG002_archive_index.md](REG002_archive_index.md). Completed plans are moved to the archive branch to keep the working directory clean.

### Q: How do I see what changed in the last commit?

**A:** Check `project_management/change_history.md` - automatically maintained by `/commit` workflow.

### Q: I need to understand why a design decision was made 3 months ago.

**A:** Look for analysis documents in [REG002_archive_index.md](REG002_archive_index.md). Retrieve with `git show archive-branch:<path>`.

### Q: When should I use /plan vs just implementing?

**A:** Use `/plan` for features with >5 requirements or novel/complex technical elements. For simple bug fixes or trivial features, implement directly.

---

**Maintained By:** WKMP Development Team
**Last Updated:** 2025-11-02
