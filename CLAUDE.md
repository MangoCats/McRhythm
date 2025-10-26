# WKMP (Auto DJ Music Player)

**Purpose:** Develop and maintain a music player that automatically selects and plays music passages based on user-configured musical flavor preferences, using AcousticBrainz data and algorithmic selection with sample-accurate crossfading.

**Technology Stack:**
- **Language:** Rust (stable channel)
- **Async Runtime:** Tokio
- **Web Framework:** Axum (HTTP server, SSE support)
- **Audio Stack:** symphonia (decode), rubato (resample), cpal (output)
- **Database:** SQLite with JSON1 extension
- **Architecture:** Microservices (5 independent HTTP servers)

---

# Development Workflows

WKMP uses automated workflows via Claude Code custom commands. See [.claude/commands/README.md](.claude/commands/README.md) for complete documentation.

**Core Workflows:**
- **/commit** - Multi-step commit with automatic change history tracking and archive synchronization
- **/doc-name** - Assign CAT### prefixes to documents following governance system
- **/think** - Multi-agent analysis for complex questions and architectural decisions
- **/plan** - Create specification-driven implementation plans with test-first approach
- **/archive** - Move completed documents to archive branch (context window optimization)
- **/archive-plan** - Batch archive completed implementation plans

**Quick Start:** See [workflows/DWI001_workflow_quickstart.md](workflows/DWI001_workflow_quickstart.md)

**Key Principles:**
- Always use `/commit` for commits (maintains change_history.md automatically)
- Use `/think` before major architectural decisions (evidence-based choices)
- Use `/plan` for non-trivial features (test-first, spec verification)
- Archive completed work regularly (clean context, preserved history)

---

# Implementation Workflow - MANDATORY

**For all features requiring >5 requirements OR novel/complex features:**
- MUST run `/plan [specification_document]` before implementing
- MUST resolve all CRITICAL specification issues before coding
- MUST achieve 100% test coverage per traceability matrix
- MUST pass all acceptance tests before considering increment complete

**Rationale:** Proactive specification verification prevents costly rework. Research shows "most agent failures are context failures" - `/plan` workflow catches specification gaps, ambiguities, and conflicts BEFORE implementation begins.

**When to use `/plan`:**
- Feature has >5 requirements (non-trivial)
- Feature involves novel/complex technical elements
- Feature affects multiple microservices
- Specification complexity detected (ambiguous requirements, missing details)

**What `/plan` provides:**
- Specification completeness verification (Phase 2)
- Acceptance test definitions (Phase 3)
- Traceability matrix (100% requirement coverage)
- Automatic `/think` integration if complexity warrants deeper analysis

---

# Decision-Making Framework - MANDATORY

**All design and implementation decisions MUST follow this framework:**

## 1. Risk Assessment (Primary Criterion)

Evaluate failure risk FIRST for every approach:
- Identify specific failure modes (what could go wrong)
- Quantify probability and impact for each failure mode
- Define mitigation strategies
- Evaluate residual risk after mitigation
- Rank approaches by failure risk (lowest risk = highest priority)

**Risk is the primary decision factor.** Choose the approach with lowest residual risk.

## 2. Quality Characteristics (Secondary Criterion)

Among approaches with **equivalent risk**, evaluate quality:
- Maintainability: How easy to modify, extend, debug
- Test coverage: Can we achieve adequate testing
- Architectural alignment: Fits with existing patterns and standards

**Quality is the tiebreaker when risks are equivalent.**

## 3. Implementation Effort (Tertiary Consideration)

Among approaches with **equivalent risk AND equivalent quality**, consider effort:
- Implementation time (design, coding, testing)
- Dependencies and complexity
- Resource requirements

**Effort is acknowledged but NOT a decision factor unless risk and quality are equivalent.**

**Critical Rule:** If the lowest-risk approach requires 2x effort versus a higher-risk approach, **choose the lowest-risk approach.** Effort differential is secondary to risk reduction.

---

## Equivalent Risk Definition

Approaches have **equivalent risk** when their residual risk (after mitigation) falls in the same category:

- **Low = Low** (equivalent)
- **Low-Medium = Low-Medium** (equivalent)
- **Low ≠ Low-Medium** (NOT equivalent - choose Low)

For borderline cases (e.g., "high-end Low" vs. "low-end Low-Medium"):
- Use engineering judgment
- Document rationale in decision record
- When in doubt, choose more conservative (lower) risk

---

## Rationale

**Project charter goals ([PCH001](PCH001_project_charter.md)) are quality-absolute:**
- "Flawless audio playback" - Zero-defect goal, NOT effort-bounded
- "Listener experience reminiscent of 1970s FM radio" - Reference quality standard

**Risk of failure to achieve these goals outweighs implementation time.**

This framework ensures decisions align with charter by prioritizing approaches that minimize risk of failing to deliver quality-absolute goals.

---

# Document Generation Verbosity Standards

**All document generation MUST follow these standards:**

## Quantified Targets

- Aim for 20-40% shorter than comprehensive first draft
- Executive summaries: <300 lines
- Detailed sections: <300 lines each
- Commit messages: <10 lines
- Analysis documents: Use modular structure if >1200 lines

## Stylistic Guidelines

- **Use bullet points** instead of paragraphs for lists
- **One concept per sentence** - avoid run-on explanations
- **Link to existing documentation** instead of repeating content
- **Prefer tables** over prose for structured data
- **Remove hedging language** ('possibly,' 'might,' 'could potentially')
- **Be direct:** "To implement X" not "In order to accomplish the task of implementing X"

## Examples

❌ **Verbose:** "In order to accomplish the task of implementing this feature, it is necessary to first consider the architectural implications and then proceed with the design phase before moving on to implementation." (34 words)

✅ **Concise:** "To implement this feature: consider architecture, design, then implement." (10 words, 70% shorter)

❌ **Verbose:** "It might be possible that we could potentially consider using approach A, though approach B might also possibly work."

✅ **Concise:** "Consider approach A or B."

## Priority

**Clarity first, then conciseness.** If in doubt, err on the side of clarity. The goal is efficient communication, not cryptic brevity.

## Success Metrics

- Document size reduced 20-40% from unoptimized draft
- Meaning preserved (no information loss)
- Improved readability (easier to scan and understand)

---

# Documentation Reading Protocol - MANDATORY

**For ALL documentation access, follow this protocol:**

## 1. Always Start with Summaries

- If document has executive summary, read ONLY summary first
- If no summary exists, read first 50-100 lines for overview
- Do NOT load full document initially

## 2. Drill Down Strategically

- Based on summary, identify which sections are relevant to current task
- Read ONLY those sections (reference by line number ranges when possible)
- Ignore irrelevant sections entirely

## 3. Never Load Full Specifications Into Context

- For SPEC###, REQ###, IMPL### documents: use targeted section reading
- Reference sections by line number (e.g., "per SPEC002:45-78")
- Keep detailed specification content out of context unless actively implementing that specific feature
- Exception: If document <300 lines, may read in full

## 4. Use Line Number References

- When referencing specifications, cite line numbers (e.g., "per SPEC002:45-78")
- Allows future readers to locate exact content without loading full document
- Supports efficient context window usage

## Examples

✅ **Good:** Read GOV001 lines 1-50 (overview) → Identify relevant section → Read GOV001 lines 200-250 (modular structure section only)

❌ **Bad:** Load entire GOV001 (997 lines) into context to answer question about one section

✅ **Good:** "Per REQ001-requirements.md lines 340-365, crossfade timing must be sample-accurate"

❌ **Bad:** Load all of REQ001 to check one requirement

## Rationale

**Context window capacity is finite.** Loading unnecessary content reduces focus and increases risk of missing critical details ("lost in the middle" phenomenon). Summary-first reading with targeted drill-down optimizes both AI and human comprehension.

---

# Key Directories

- **`docs/`**: All technical documentation (requirements, architecture, design specs)
  - Start with [Document Hierarchy](docs/GOV001-document_hierarchy.md) for documentation governance
  - See [Requirements](docs/REQ001-requirements.md) for complete feature specifications
- **`workflows/`**: Development workflow procedures and process documentation
  - [DWI001_workflow_quickstart.md](workflows/DWI001_workflow_quickstart.md) - Quick start guide
  - [REG001_number_registry.md](workflows/REG001_number_registry.md) - Document number tracking
  - [REG002_archive_index.md](workflows/REG002_archive_index.md) - Archive retrieval index
  - [REG003_category_definitions.md](workflows/REG003_category_definitions.md) - 13-category system
- **`project_management/`**: Project tracking and audit trail
  - [change_history.md](project_management/change_history.md) - Automatic change tracking via /commit
- **`wip/`**: Work-in-progress documents (analysis, plans, drafts)
- **`common/`**: Shared library crate (`wkmp-common`) for database models, events, utilities
- **`wkmp-ap/`**: Audio Player microservice (playback engine, queue, crossfading)
- **`wkmp-ui/`**: User Interface microservice (web UI, authentication, proxying)
- **`wkmp-pd/`**: Program Director microservice (automatic passage selection)
- **`wkmp-ai/`**: Audio Ingest microservice (file scanning, MusicBrainz integration - Full version only)
- **`wkmp-le/`**: Lyric Editor microservice (on-demand lyric editing - Full version only)
- **`migrations/`**: Shared SQLite database migrations
- **`scripts/`**: Build and packaging scripts for Full/Lite/Minimal versions

---

# Microservices Architecture

WKMP consists of **5 independent HTTP-based microservices**:

| Module | Port | Purpose | Versions |
|--------|------|---------|----------|
| **Audio Player (wkmp-ap)** | 5721 | Core playback, crossfading, queue management | All |
| **User Interface (wkmp-ui)** | 5720 | Web UI, authentication, orchestration | All |
| **Program Director (wkmp-pd)** | 5722 | Automatic passage selection algorithm | Full, Lite |
| **Audio Ingest (wkmp-ai)** | 5723 | File import, MusicBrainz/AcousticBrainz integration | Full (on-demand) |
| **Lyric Editor (wkmp-le)** | 5724 | Split-window lyric editing interface | Full (on-demand) |

**Communication:** HTTP REST APIs + Server-Sent Events (SSE) for real-time updates

---

# Core Concepts

**Entity Definitions** (see [docs/REQ002-entity_definitions.md](docs/REQ002-entity_definitions.md)):
- **Passage:** Continuous playable region within an audio file (start/end points, crossfade timing)
- **Song:** MusicBrainz Recording + Artist(s) (used for selection and cooldowns)
- **Musical Flavor:** AcousticBrainz vector characterizing passage's musical properties
- **Timeslot:** Time-of-day schedule defining target musical flavor

**Key Features:**
- Sample-accurate crossfading (~0.02ms precision, 5 fade curve types)
- Automatic passage selection based on:
  - Musical flavor distance from time-of-day target
  - Cooldown periods (song/artist/work level)
  - User-configurable base probabilities
- Multi-user coordination via SSE
- Three versions: Full (all features), Lite (no file ingest), Minimal (playback only)

---

# Documentation Hierarchy

WKMP uses a strict 5-tier documentation framework (see [docs/GOV001-document_hierarchy.md](docs/GOV001-document_hierarchy.md)):

- **Tier 0 (Governance):** GOV001-document_hierarchy.md - Documentation framework itself
- **Tier 1 (Authoritative):** REQ001-requirements.md, REQ002-entity_definitions.md - WHAT the system must do
- **Tier 2 (Design):** SPEC001-architecture.md, SPEC007-api_design.md, SPEC002-crossfade.md, etc. - HOW requirements are satisfied
- **Tier 3 (Implementation):** IMPL001-database_schema.md, IMPL002-coding_conventions.md, IMPL003-project_structure.md - Concrete implementation specs
- **Tier 4 (Execution):** EXEC001-implementation_order.md - WHEN features are built (aggregates all upstream specs)

**Information Flow:**
- **Downward (normal):** Requirements → Design → Implementation → Execution
- **Upward (controlled):** Implementation discoveries may inform design/requirements via formal change control

**Important:** Lower-tier documents are updated when higher-tier documents change. Upward flow requires explicit review and approval.

---

# Development Workflow

**Build Commands:**
```bash
# Build all modules
cargo build

# Build specific module
cargo build -p wkmp-ap

# Run all tests
cargo test

# Run specific module in dev mode
cargo run -p wkmp-ui
```

**Version Packaging:**
- Full: All 5 binaries (wkmp-ap, wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le)
- Lite: 3 binaries (wkmp-ap, wkmp-ui, wkmp-pd)
- Minimal: 2 binaries (wkmp-ap, wkmp-ui)

**No conditional compilation** - versions differ only by which binaries are packaged.

---

# Requirement Traceability

All requirements are enumerated with IDs following the scheme in [docs/GOV002-requirements_enumeration.md](docs/GOV002-requirements_enumeration.md):

- Format: `DOC-CAT-NNN` (e.g., `REQ-CF-010`, `ARCH-VOL-010`)
- Document codes: REQ, ARCH, XFD, FLV, DB, etc.
- All code should reference requirement IDs in comments for traceability

---

# Key Technical Decisions

**Single-Stream Audio Architecture:**
- Custom implementation using symphonia + rubato + cpal
- Sample-accurate crossfading with automatic fade curve application
- Pre-decoded PCM buffers with dynamic fade multiplication
- Ring buffer for lock-free audio thread communication

**Database Design:**
- UUID primary keys for all entities (enables database merging across versions)
- Musical flavor vectors stored as JSON (SQLite JSON1 extension)
- Automatic triggers for last_played_at timestamps
- Foreign key cascades for cleanup

**Event-Driven Architecture:**
- `tokio::broadcast` for one-to-many event distribution
- Server-Sent Events (SSE) for real-time UI updates
- HTTP REST APIs for module communication

**Configuration:**
- Database-first: All runtime settings in SQLite `settings` table
- TOML files: Bootstrap only (root folder path, logging, static assets)
- Default values initialized from code when database settings missing/NULL

---

# Common Pitfalls to Avoid

❌ **Don't:** Update REQ001-requirements.md because implementation is easier a different way<br/>
✅ **Do:** Update design/implementation to satisfy requirements as written, or formally propose requirement change

❌ **Don't:** Let EXEC001-implementation_order.md define new requirements<br/>
✅ **Do:** Use EXEC001-implementation_order.md to discover requirement gaps, then update REQ001-requirements.md via change control

❌ **Don't:** Create circular references between documents<br/>
✅ **Do:** Follow strict hierarchy: higher tiers inform lower tiers, never reverse

❌ **Don't:** Put module-specific code in `common/` library<br/>
✅ **Do:** Only shared models, utilities, and cross-module types go in `common/`

---

# Getting Help

- **Documentation Structure:** See [docs/GOV001-document_hierarchy.md](docs/GOV001-document_hierarchy.md)
- **Requirements:** See [docs/REQ001-requirements.md](docs/REQ001-requirements.md)
- **Architecture:** See [docs/SPEC001-architecture.md](docs/SPEC001-architecture.md)
- **API Design:** See [docs/SPEC007-api_design.md](docs/SPEC007-api_design.md)
- **Database Schema:** See [docs/IMPL001-database_schema.md](docs/IMPL001-database_schema.md)
- **Implementation Plan:** See [docs/EXEC001-implementation_order.md](docs/EXEC001-implementation_order.md)

For questions about the project, consult the documentation hierarchy or ask the technical lead.
