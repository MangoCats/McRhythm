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

# Agent Guidance

- **docs-specialist:** Use for reviewing and improving project documentation in the `docs/` directory.
- **project-architect:** Use for architectural planning and resolving discrepancies identified in documentation.
- **ui-ux-designer:** Use for frontend design tasks, including UI components and SSE data visualization.
- **code-implementer:** Use for writing, refactoring, and debugging core application code.

---

# Key Directories

- **`docs/`**: All technical documentation (requirements, architecture, design specs)
  - Start with [Document Hierarchy](docs/document_hierarchy.md) for documentation governance
  - See [Requirements](docs/requirements.md) for complete feature specifications
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

**Entity Definitions** (see [docs/entity_definitions.md](docs/entity_definitions.md)):
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

WKMP uses a strict 5-tier documentation framework (see [docs/document_hierarchy.md](docs/document_hierarchy.md)):

- **Tier 0 (Governance):** document_hierarchy.md - Documentation framework itself
- **Tier 1 (Authoritative):** requirements.md, entity_definitions.md - WHAT the system must do
- **Tier 2 (Design):** architecture.md, api_design.md, crossfade.md, etc. - HOW requirements are satisfied
- **Tier 3 (Implementation):** database_schema.md, coding_conventions.md, project_structure.md - Concrete implementation specs
- **Tier 4 (Execution):** implementation_order.md - WHEN features are built (aggregates all upstream specs)

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

All requirements are enumerated with IDs following the scheme in [docs/requirements_enumeration.md](docs/requirements_enumeration.md):

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

❌ **Don't:** Update requirements.md because implementation is easier a different way<br/>
✅ **Do:** Update design/implementation to satisfy requirements as written, or formally propose requirement change

❌ **Don't:** Let implementation_order.md define new requirements<br/>
✅ **Do:** Use implementation_order.md to discover requirement gaps, then update requirements.md via change control

❌ **Don't:** Create circular references between documents<br/>
✅ **Do:** Follow strict hierarchy: higher tiers inform lower tiers, never reverse

❌ **Don't:** Put module-specific code in `common/` library<br/>
✅ **Do:** Only shared models, utilities, and cross-module types go in `common/`

---

# Getting Help

- **Documentation Structure:** See [docs/document_hierarchy.md](docs/document_hierarchy.md)
- **Requirements:** See [docs/requirements.md](docs/requirements.md)
- **Architecture:** See [docs/architecture.md](docs/architecture.md)
- **API Design:** See [docs/api_design.md](docs/api_design.md)
- **Database Schema:** See [docs/database_schema.md](docs/database_schema.md)
- **Implementation Plan:** See [docs/implementation_order.md](docs/implementation_order.md)

For questions about the project, consult the documentation hierarchy or ask the technical lead.
