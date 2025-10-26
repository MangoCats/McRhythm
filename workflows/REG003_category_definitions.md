# Document Category Definitions

- **Purpose:** Define all document categories for WKMP project
- **Maintained by:** Manual (governance document)
- **Last Updated:** 2025-10-25

---

## Overview

WKMP uses a unified 13-category system combining original WKMP categories (7) with workflow-adoption categories (6). All documents use the format `CAT###_descriptive_name.md`.

**Benefits:**
- Terse cross-references: "See RPT074" instead of long filenames
- Stable ordering and organization
- Clear separation between product docs (WHAT) and workflow docs (HOW)
- Easy to find and reference documents

---

## Category System (13 Categories)

### Product Documentation (WHAT we're building)

**Location:** `docs/`

#### GOV### - Governance
**Purpose:** Documentation framework and project governance
**Examples:**
- GOV001_document_hierarchy.md
- GOV002_requirements_enumeration.md
- GOV003_filename_convention.md

**Content Type:**
- Documentation standards
- Requirements enumeration systems
- File naming conventions
- Meta-documentation (documentation about documentation)

#### REQ### - Requirements
**Purpose:** Authoritative requirements (WHAT system must do)
**Examples:**
- REQ001_requirements.md
- REQ002_entity_definitions.md

**Content Type:**
- Functional requirements
- Non-functional requirements
- Entity definitions
- System capabilities

**Tier:** Tier 1 (Authoritative) in document hierarchy

#### SPEC### - Specifications
**Purpose:** Design specifications (HOW requirements are satisfied)
**Examples:**
- SPEC002_crossfade.md
- SPEC007_api_design.md
- SPEC011_event_system.md

**Content Type:**
- Technical specifications
- Algorithm designs
- API designs
- System architecture
- Interface specifications

**Tier:** Tier 2 (Design) in document hierarchy

#### IMPL### - Implementation
**Purpose:** Concrete implementation specifications
**Examples:**
- IMPL001_database_schema.md
- IMPL002_coding_conventions.md
- IMPL003_project_structure.md

**Content Type:**
- Database schemas
- Coding standards
- Project structure
- Deployment procedures
- Technology stack details

**Tier:** Tier 3 (Implementation) in document hierarchy

#### EXEC### - Execution
**Purpose:** Implementation order and schedules
**Examples:**
- EXEC001_implementation_order.md

**Content Type:**
- Implementation schedules
- Feature sequencing
- Milestone planning
- Aggregates all upstream specs

**Tier:** Tier 4 (Execution) in document hierarchy

#### REV### - Reviews
**Purpose:** Design reviews and decision updates
**Examples:**
- REV001_wkmp_ap_design_review.md
- REV004_incremental_buffer_implementation.md

**Content Type:**
- Design review findings
- Architecture decision updates
- Technical critiques
- Change rationale

#### GUIDE### - Guides
**Purpose:** Implementation guides and tutorials
**Examples:**
- GUIDE001_wkmp_ap_implementation_plan.md

**Content Type:**
- Implementation walkthroughs
- Developer guides
- Tutorial content
- How-to documentation

---

### Analysis & Planning (Research and preparation)

**Location:** `docs/` or `wip/` (then archived)

#### RPT### - Reports & Analysis
**Purpose:** Analysis outputs, research, investigations
**Examples:**
- RPT001_architecture_analysis.md (hypothetical)
- RPT002_performance_investigation.md (hypothetical)

**Content Type:**
- /think workflow outputs
- Research reports
- Investigation findings
- Technical analysis
- Option comparisons
- Feasibility studies

**Lifecycle:** Created during analysis, archived after implementation

#### PLAN### - Implementation Plans
**Purpose:** Implementation plans with test specifications
**Examples:**
- PLAN001_audio_player_implementation/ (hypothetical)
- PLAN002_ui_development/ (hypothetical)

**Content Type:**
- /plan workflow outputs
- Test specifications
- Implementation breakdowns
- Requirement traceability matrices
- Acceptance criteria

**Lifecycle:** Created during planning, archived after completion

---

### Workflow Documentation (HOW we build)

**Location:** `workflows/`

#### DWI### - Developer Work Instructions
**Purpose:** Workflow procedures and process documentation
**Examples:**
- DWI001_workflow_quickstart.md (to be created)
- DWI002_commit_workflow_design.md (hypothetical)

**Content Type:**
- Workflow procedures
- Process architecture
- Development instructions
- Workflow design decisions
- Tool usage guides

**Note:** This is workflow/process documentation, NOT product documentation

#### TMPL### - Templates
**Purpose:** Reusable document templates
**Examples:**
- TMPL001_think_document_template.md (hypothetical)
- TMPL002_analysis_template.md (hypothetical)

**Content Type:**
- Document templates
- Workflow patterns
- Reusable structures
- Template guidelines

---

### Cross-Cutting (Tracking and reference)

#### LOG### - Operational Logs
**Purpose:** Ongoing logs, metrics, feedback
**Location:** `project_management/`

**Examples:**
- LOG001_workflow_execution_log.md (hypothetical)
- LOG002_ai_feedback_log.md (hypothetical)

**Content Type:**
- Change history (change_history.md uses LOG implicitly)
- Workflow execution tracking
- AI feedback and corrections
- Metrics and measurements
- Operational notes

**Special:** change_history.md doesn't need LOG prefix (standard file)

#### REG### - Registries
**Purpose:** System registries, lookup tables, tracking files
**Location:** `workflows/`

**Examples:**
- REG001_number_registry.md (this file's sibling)
- REG002_archive_index.md (archive retrieval)
- REG003_category_definitions.md (this file)

**Content Type:**
- Document number tracking
- Archive indices
- Category definitions
- Lookup tables
- System registries

---

## Exempted Files (No Prefix Required)

The following files do NOT require CAT### prefixes:

**Standard Project Files:**
- `README.md` (project overview)
- `CLAUDE.md` (AI instructions)
- `STRUCTURE.md` (folder organization)

**Workflow Definitions:**
- `.claude/commands/*.md` (workflow command definitions)

**Project Management:**
- `change_history.md` (maintained by /commit)
- Standard metadata files

**Module Readmes:**
- `src/README.md`, `wkmp-ap/README.md`, etc.

**Temporary State:**
- Files in `wip/` starting with underscore (e.g., `_draft.md`)
- Files within `CAT###_*` folders (inherit parent's context)

---

## Category Selection Guidelines

### By Location

| Location | Likely Categories |
|----------|-------------------|
| `docs/` | GOV, REQ, SPEC, IMPL, EXEC, REV, GUIDE, RPT |
| `workflows/` | DWI, TMPL, REG |
| `project_management/` | LOG |
| `wip/` | RPT, PLAN (before permanent location) |

### By Name

| Keyword in Name | Likely Category |
|-----------------|-----------------|
| governance, hierarchy, convention | GOV |
| requirements, entities | REQ |
| specification, design, api, algorithm | SPEC |
| implementation, schema, coding, structure | IMPL |
| execution, schedule, order | EXEC |
| review, critique, decision | REV |
| guide, tutorial, walkthrough | GUIDE |
| analysis, report, investigation, research | RPT |
| plan, breakdown, test | PLAN |
| workflow, procedure, instruction | DWI |
| template, pattern | TMPL |
| log, feedback, history | LOG |
| registry, index, lookup | REG |

### By Content Type

**Governance/Standards:** GOV
**What system must do:** REQ
**How to satisfy requirements:** SPEC
**Concrete implementation details:** IMPL
**When to build features:** EXEC
**Reviewing/critiquing design:** REV
**Teaching how to implement:** GUIDE
**Analyzing options/problems:** RPT
**Planning implementation:** PLAN
**Workflow procedures:** DWI
**Reusable templates:** TMPL
**Ongoing tracking:** LOG
**Reference tables:** REG

---

## Product vs. Workflow Distinction

**Key Distinction:**

**Product Documentation** = WHAT we're building (the music player system)
- Categories: GOV, REQ, SPEC, IMPL, EXEC, REV, GUIDE, RPT, PLAN
- Location: `docs/`

**Workflow Documentation** = HOW we build it (development processes)
- Categories: DWI, TMPL
- Location: `workflows/`

**Cross-Cutting** = Tracking and reference for both
- Categories: LOG, REG
- Location: `project_management/`, `workflows/`

**Examples:**

| Document | Category | Reasoning |
|----------|----------|-----------|
| HTTP interface design for music player | SPEC | Product specification |
| Workflow for creating implementation plans | DWI | Process documentation |
| Analysis of crossfade algorithms | RPT | Product analysis |
| Analysis of which workflow tool to use | RPT | Process analysis (but still RPT) |
| Implementation plan for audio player | PLAN | Product planning |
| Template for creating test specifications | TMPL | Workflow template |

---

## Document Lifecycle

### Typical Flow

```
wip/
  ↓ (create and develop)
wip/CAT###_name.md (via /doc-name)
  ↓ (complete and finalize)
docs/ or workflows/ (permanent location)
  ↓ (supersede or complete)
archive branch (via /archive)
```

### Status by Category

**Never Archived** (always active):
- GOV, REG (governance and registries stay active)

**Rarely Archived:**
- REQ (requirements are stable)
- SPEC, IMPL (core specs remain relevant)
- DWI, TMPL (workflows and templates stay active)

**Commonly Archived:**
- RPT (after analysis implemented)
- PLAN (after plan completed)
- REV (after changes implemented)
- GUIDE (if superseded)

**Always Archived Eventually:**
- LOG (logs rotate to archive)

---

## Assignment Workflow

The `/doc-name` workflow handles category assignment:

1. **Analyze** document location, name, content
2. **Recommend** category based on guidelines above
3. **Get** next available number from REG001
4. **Rename** file to `CAT###_original_name.md`
5. **Update** REG001 registry
6. **Stage** changes for /commit

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-10-25 | Initial unified category system (13 categories) |

---

**Maintained by:** Manual updates (governance)
**Status:** Active reference
**Related:** REG001 (number registry), GOV001 (document hierarchy), GOV003 (filename convention)
