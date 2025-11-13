# WKMP Specification Document Search Index

**Quick Reference for Finding Database, Zero-Config, and Initialization Specifications**

## Search Index by Topic

### Database Schema & Management

| Topic | Document | Lines | Key Sections |
|---|---|---|---|
| **Complete Schema Definition** | docs/IMPL001-database_schema.md | 1,325 | All tables, relationships, constraints |
| **Schema Versioning** | docs/IMPL001-database_schema.md | 1253-1283 | Migration strategy, development phase status |
| **Settings Table** | docs/IMPL001-database_schema.md | 799-891, 999-1046 | 27+ settings, database-first configuration |
| **Module Config Table** | docs/IMPL001-database_schema.md | 750-798 | Module discovery, default initialization |
| **Core Entity Tables** | docs/IMPL001-database_schema.md | 119-533 | Files, passages, songs, artists, works, albums |
| **Relationship Tables** | docs/IMPL001-database_schema.md | 421-532 | passage_songs, song_artists, passage_albums |
| **Playback History** | docs/IMPL001-database_schema.md | 534-672 | play_history, song_play_history, likes_dislikes |
| **Triggers & Automation** | docs/IMPL001-database_schema.md | 1096-1206 | Automatic updates, cooldown tracking |

### Zero-Configuration Startup

| Topic | Document | Lines | Key Details |
|---|---|---|---|
| **4-Tier Resolution System** | docs/ADR-003-zero_configuration_strategy.md | 23-65 | CLI → ENV → TOML → defaults priority |
| **Implementation Pattern** | docs/ADR-003-zero_configuration_strategy.md | 34-51 | RootFolderResolver & RootFolderInitializer usage |
| **Architecture Details** | docs/SPEC001-architecture.md | 382-426 | Startup sequence, module initialization |
| **Deployment Specs** | docs/IMPL004-deployment.md | 50-94, 119-247 | Configuration locations, graceful degradation |
| **Graceful Degradation** | docs/IMPL007-graceful_degradation_implementation.md | 1-70 | Implementation specification and patterns |

### Requirements by ID

#### Zero-Configuration (REQ-NF-030 through REQ-NF-038)

| Req ID | Document | Lines | Title |
|---|---|---|---|
| **REQ-NF-030** | docs/REQ001-requirements.md | 256-285 | Configuration file graceful degradation |
| **REQ-NF-031** | docs/REQ001-requirements.md | 257 | Missing TOML files SHALL NOT cause termination |
| **REQ-NF-032** | docs/REQ001-requirements.md | 258-261 | Missing config → warning + defaults + startup |
| **REQ-NF-033** | docs/REQ001-requirements.md | 262-265 | Root folder default location per platform |
| **REQ-NF-034** | docs/REQ001-requirements.md | 266-269 | Default values for logging, static assets |
| **REQ-NF-035** | docs/REQ001-requirements.md | 270-274 | Priority order for root folder resolution |
| **REQ-NF-036** | docs/REQ001-requirements.md | 275-278 | First-run experience (auto-creation) |
| **REQ-NF-037** | docs/REQ001-requirements.md | 279-283 | Implementation enforcement via wkmp_common |
| **REQ-NF-038** | docs/REQ001-requirements.md | 285-292 | TOML Configuration Directory Auto-Creation |

#### Architecture Initialization (ARCH-INIT-*)

| Req ID | Document | Key Purpose |
|---|---|---|
| **ARCH-INIT-003** | docs/IMPL007-graceful_degradation_implementation.md:55 | Tracing subscriber initialization |
| **ARCH-INIT-004** | docs/IMPL007-graceful_degradation_implementation.md:56, CLAUDE.md | Build identification logging (REQUIRED) |
| **ARCH-INIT-005** | docs/IMPL007-graceful_degradation_implementation.md:57 | Root folder resolution algorithm |
| **ARCH-INIT-010** | docs/IMPL007-graceful_degradation_implementation.md:58 | Module startup sequence |
| **ARCH-INIT-015** | docs/IMPL007-graceful_degradation_implementation.md:59 | Missing configuration handling |
| **ARCH-INIT-020** | docs/IMPL007-graceful_degradation_implementation.md:60 | Default value initialization |

#### Deployment (DEP-*)

| Req ID | Document | Purpose |
|---|---|---|
| **DEP-DB-010** | docs/IMPL004-deployment.md:250-251 | SQLite database location |
| **DEP-DB-011** | docs/IMPL004-deployment.md:264-268 | Automatic database initialization |
| **DEP-DB-020** | docs/IMPL004-deployment.md:271 | All modules use same root folder |
| **DEP-DB-030** | docs/IMPL004-deployment.md:273 | Auto-create root folder/database |
| **DEP-CFG-031** | docs/IMPL004-deployment.md:50-54 | Graceful degradation |
| **DEP-CFG-035** | docs/IMPL004-deployment.md:56-94 | Module discovery via database |
| **DEP-CFG-040** | docs/IMPL004-deployment.md:119-147 | Compiled default values |

### Quick Lookup by Concept

#### Root Folder Resolution (4 Tiers)

1. **CLI Argument** (Tier 1)
   - `--root-folder /custom/path` or `--root /custom/path`
   - See: ADR-003, lines 29-30

2. **Environment Variable** (Tier 2)
   - `WKMP_ROOT_FOLDER=/custom/path` or `WKMP_ROOT=/custom/path`
   - See: ADR-003, lines 30-31

3. **TOML Config File** (Tier 3)
   - Linux: `~/.config/wkmp/<module-name>.toml`
   - See: IMPL004-deployment.md, lines 37-42

4. **Compiled Default** (Tier 4)
   - Linux/macOS: `~/Music`, Windows: `%USERPROFILE%\Music`
   - See: ADR-003, lines 32; IMPL004-deployment.md, lines 123-126

#### Default Settings Initialization

| Setting Category | Count | Document Location |
|---|---|---|
| Playback State | 4 | IMPL001-database_schema.md:828-832 |
| Audio Configuration | 2 | IMPL001-database_schema.md:833-835 |
| Event Timing | 2 | IMPL001-database_schema.md:838-841 |
| Database Backup | 5 | IMPL001-database_schema.md:843-848 |
| Crossfade | 2 | IMPL001-database_schema.md:849-851 |
| Pause/Resume | 2 | IMPL001-database_schema.md:852-854 |
| Volume Fade | 1 | IMPL001-database_schema.md:855-856 |
| Queue Management | 7 | IMPL001-database_schema.md:857-864 |
| Module Management | 2 | IMPL001-database_schema.md:865-867 |
| Session Management | 1 | IMPL001-database_schema.md:868-869 |
| File Ingest | 1 | IMPL001-database_schema.md:870-871 |
| Library | 2 | IMPL001-database_schema.md:872-874 |
| HTTP Server | 4 | IMPL001-database_schema.md:875-878 |
| Program Director | 2 | IMPL001-database_schema.md:880-882 |

#### Module Configuration Defaults

**Document:** IMPL001-database_schema.md, lines 773-797

All 6 modules pre-populated at first run:
- `user_interface`: 127.0.0.1:5720
- `audio_player`: 127.0.0.1:5721
- `program_director`: 127.0.0.1:5722
- `audio_ingest`: 0.0.0.0:5723
- `lyric_editor`: 0.0.0.0:5724
- `database_review`: 0.0.0.0:5725

### Implementation Code Patterns

#### Per-Module Main Function (MANDATORY)

**Document:** CLAUDE.md or IMPL007-graceful_degradation_implementation.md:740-810

**Pattern Requirements:**
1. Init tracing [ARCH-INIT-003]
2. Log build identification IMMEDIATELY [ARCH-INIT-004]
3. Resolve root folder [ARCH-INIT-005]
4. Create root folder if missing [ARCH-INIT-010]
5. Initialize database
6. Initialize module-specific tables
7. Initialize default settings
8. Start HTTP server

#### Database Initialization Functions

**Document:** wkmp-common/src/db/init.rs (referenced in IMPL007-graceful_degradation_implementation.md:519-596)

**Key Functions:**
- `init_database()` - Create/open database
- `create_table_if_not_exists()` - Idempotent table creation
- `init_settings()` - Initialize default settings
- `ensure_setting()` - Insert default if missing
- `reset_null_settings()` - Handle NULL values
- `init_users()` - Create Anonymous user
- `init_module_config()` - Module discovery

#### Shared Configuration Library

**Location:** wkmp-common/src/config/

**Must Use (REQ-NF-037):**
- `RootFolderResolver::new(module_name).resolve()`
- `RootFolderInitializer::new(root_folder).ensure_directory_exists()`
- `RootFolderInitializer::database_path()`

**No Custom Implementations Allowed:** All modules MUST use these shared utilities

### Database Design Constraints

| Constraint | Rationale | Document |
|---|---|---|
| **UUID Primary Keys** | Database merging (Minimal→Lite→Full) | IMPL001:1222-1228 |
| **SQLite (not PostgreSQL)** | Zero-config goal, no server | SPEC001, PCH001 |
| **Global Playback State** | Shared hi-fi system (family room) | IMPL001:21-46 |
| **Relative Paths** | Portability (move entire root folder) | IMPL001:47-69 |
| **Tick-Based Timing** | Sample-accurate precision | SPEC017, IMPL001:169-173 |
| **JSON Storage** | Schema-less extensibility | IMPL001:1230-1251 |
| **Database-First Config** | Single source of truth | IMPL001:810-825 |

### Testing & Verification

| Category | Test Count | Document |
|---|---|---|
| Zero-Config Startup | 27+ | IMPL007-TEST_SUMMARY.md:172-406 |
| Root Folder Resolution | 5 | ADR-003:309-338 |
| Database Initialization | 5 | IMPL007-TEST_SUMMARY.md:42-87 |
| Configuration Loading | 4 | IMPL007-TEST_SUMMARY.md:109-114 |
| Module Compliance | 6 | IMPL007-IMPLEMENTATION_SUMMARY.md:27-45 |
| Integration Tests | 3+ | IMPL007-graceful_degradation_implementation.md:309-355 |

### Known Gaps

| Gap | Severity | Impact | Effort |
|---|---|---|---|
| Health Check Diagnostics | Medium | Missing detailed status in `/health` endpoint | 2-3 hours per module |
| Database Backup Implementation | Medium | Settings ready, no execution | 1-2 days |
| Migration from Old Installs | Medium | Users need manual migration help | 1 day |
| Configuration Validation | Low | Silent fallback on typos | 2-3 hours |

### Recommended Reading Order

**For Understanding Zero-Config Startup:**
1. `ADR-003-zero_configuration_strategy.md` - Decision rationale (30 min)
2. `CLAUDE.md` - Zero-config requirements summary (10 min)
3. `IMPL007-graceful_degradation_implementation.md` - Implementation details (60 min)
4. `IMPL004-deployment.md` - Deployment specifications (45 min)

**For Database Schema Details:**
1. `IMPL001-database_schema.md` - Complete reference (90 min)
2. `REQ001-requirements.md` - Requirements overview (30 min)
3. `SPEC001-architecture.md` - Architecture context (30 min)

**For Implementation:**
1. `IMPL007-graceful_degradation_implementation.md` - Pattern definitions
2. `IMPL003-project_structure.md` - File organization
3. `IMPL002-coding_conventions.md` - Code standards

---

## Document Statistics

| Document | Lines | Category | Purpose |
|---|---|---|---|
| IMPL001-database_schema.md | 1,325 | Reference | Complete schema specification |
| REQ001-requirements.md | 679 | Requirements | All functional/non-functional requirements |
| SPEC001-architecture.md | 556 | Design | System architecture and module responsibilities |
| IMPL004-deployment.md | 600+ | Deployment | Configuration, startup, process management |
| ADR-003-zero_configuration_strategy.md | 406 | Decision | Zero-config rationale and alternatives |
| IMPL007-graceful_degradation_implementation.md | 1,200+ | Implementation | Detailed implementation patterns |
| IMPL007-TEST_SUMMARY.md | 600+ | Testing | Test coverage and verification |
| IMPL007-IMPLEMENTATION_SUMMARY.md | 400+ | Summary | Feature completion status |

**Total Documentation:** 5,000+ lines of specifications covering database, zero-config, and initialization

---

**Last Updated:** 2025-11-09
**Scope:** Complete database schema management, zero-configuration startup, and database initialization specifications
**Coverage:** All requirement IDs, all modules, all design decisions

See `DATABASE_REQUIREMENTS_SUMMARY.md` for comprehensive narrative documentation.
