# Documentation Consolidation Implementation Plan
**Date:** 2026-01-11
**Purpose:** Eliminate redundancies and establish single sources of truth across WKMP documentation

## Overview

**Problem:** WKMP documentation contains extensive redundancy with critical information duplicated in 3-5 locations. The microservices architecture table, zero-configuration startup pattern, and entity definitions are repeated across multiple documents without clear designation of authoritative sources.

**Impact:**
- Maintenance burden: Changes require updating 4+ files
- Inconsistency risk: Duplicates may drift out of sync
- Context window waste: 30-40% of documentation is redundant
- Developer confusion: Unclear which document is authoritative

**Scope:** 30+ documents across 5 tier levels (GOV, REQ, SPEC, IMPL, EXEC)

**Estimated Effort:** 13-18 hours total (3 phases)

---

## Phase 1: Quick Wins (1-2 hours)

**Goal:** Remove exact duplications from CLAUDE.md and replace with cross-references

**Success Criteria:**
- CLAUDE.md reduced by 200+ lines
- All exact duplicates replaced with references
- No information loss (all content accessible via links)

### Task 1.1: Replace Microservices Architecture Table

**File:** `CLAUDE.md`

**Current Content (lines 292-301):**
```markdown
| Module | Port | Purpose | Versions |
|--------|------|---------|----------|
| **Audio Player (wkmp-ap)** | 5721 | Core playback, crossfading, queue management | All |
| **User Interface (wkmp-ui)** | 5720 | Web UI, authentication, orchestration | All |
...
```

**Replace With:**
```markdown
## Microservices Architecture

WKMP consists of 6 independent HTTP-based microservices. See [SPEC001-architecture.md § 2.1](docs/SPEC001-architecture.md#21-module-overview) for complete module details, ports, and version availability.

**Quick Reference:**
- wkmp-ui (5720): Web UI and orchestration
- wkmp-ap (5721): Audio playback engine
- wkmp-pd (5722): Automatic passage selection
- wkmp-ai (5723): Import wizard (Full version, on-demand)
- wkmp-le (5724): Lyric editor (Full version, on-demand)
- wkmp-dr (5725): Database review (Full version)
```

**Acceptance Test:**
- Cross-reference link works
- Quick reference provides enough context for onboarding
- Full table available in SPEC001 only

---

### Task 1.2: Replace Zero-Configuration Startup Section

**File:** `CLAUDE.md`

**Current Content (lines 305-355):** 50 lines of implementation code

**Replace With:**
```markdown
### Zero-Configuration Startup (MANDATORY - ALL MODULES)

**[REQ-NF-030] through [REQ-NF-037]** ALL six modules MUST start without configuration files.

**4-Tier Priority for Root Folder Resolution:**
1. CLI argument: `--root-folder /custom/path`
2. Environment variable: `WKMP_ROOT_FOLDER=/custom/path`
3. TOML config: `~/.config/wkmp/<module-name>.toml`
4. Compiled default: `~/Music` (Linux/macOS), `%USERPROFILE%\Music` (Windows)

See [ADR-003-zero_configuration_strategy.md](docs/ADR-003-zero_configuration_strategy.md) for architectural decision and implementation pattern.

**Enforcement:**
- NO module may hardcode database paths
- ALL modules MUST use `wkmp_common::config` utilities

**Implementation Example:** See [IMPL003-project_structure.md](docs/IMPL003-project_structure.md#zero-config-startup-pattern) for code template.
```

**Acceptance Test:**
- Developer can find implementation code via references
- CLAUDE.md provides context without duplicating ADR-003
- Code example moved to IMPL003 (Task 2.4)

---

### Task 1.3: Replace On-Demand Microservices Section

**File:** `CLAUDE.md`

**Current Content (lines 357-388):** Complete architectural pattern with user flows

**Replace With:**
```markdown
### On-Demand Microservices

**[ARCH-OD-010]** wkmp-ai and wkmp-le are "on-demand" specialized tools with dedicated UIs.

**Access Method:**
- **wkmp-ai:** http://localhost:5723 (import wizard, file segmentation)
- **wkmp-le:** http://localhost:5724 (lyric editor, split-window interface)
- **wkmp-ui:** http://localhost:5720 (main playback UI, provides launch buttons)

See [SPEC001-architecture.md § 2.3](docs/SPEC001-architecture.md#on-demand-microservices) for architectural pattern, user flows, and rationale.

**Version Availability:**
- Full version: All 6 microservices
- Lite version: "Import Music" disabled with tooltip
- Minimal version: No import or lyric editing
```

**Acceptance Test:**
- Essential information preserved (ports, access method)
- Full architectural details in SPEC001 only
- Clear reference path for details

---

### Task 1.4: Verify CLAUDE.md Structure

**After Tasks 1.1-1.3:**

**CLAUDE.md Table of Contents Should Be:**
```markdown
# WKMP (Auto DJ Music Player)

**Purpose:** [Brief description]
**Technology Stack:** [One paragraph]

---

# Development Workflows
- /commit, /doc-name, /think, /plan, /archive (brief summaries with links to workflows/)

# Implementation Workflow - MANDATORY
[Implementation rules - keep as-is]

# Decision-Making Framework - MANDATORY
[Risk-First Framework - keep as-is]

# Professional Objectivity
[Objectivity standards - keep as-is]

# Document Generation Verbosity Standards
[Verbosity standards - keep as-is]

# Documentation Reading Protocol - MANDATORY
[Reading protocol - keep as-is]

# Key Directories
[Brief directory descriptions with links]

# Microservices Architecture
[UPDATED: Quick reference + link to SPEC001]

# Zero-Configuration Startup
[UPDATED: Summary + links to ADR-003 and IMPL003]

# On-Demand Microservices
[UPDATED: Access info + link to SPEC001]

# Core Concepts
[Brief overview + links to REQ002, SPEC003, etc.]

# Documentation Hierarchy
[Brief tier summary + link to GOV001]

# Development Workflow
[Build commands, version packaging - keep as-is]

# Requirement Traceability
[Brief explanation + link to GOV002]

# Key Technical Decisions
[Brief summaries - verify no duplicates]

# Common Pitfalls to Avoid
[Examples - keep as-is]

# Getting Help
[Links to key documents]
```

**Acceptance Test:**
- CLAUDE.md reduced from ~900 lines to ~600 lines
- All removed content accessible via clear references
- Onboarding value preserved

---

## Phase 2: Structure Improvement (4-6 hours)

**Goal:** Clarify requirement vs. design vs. implementation boundaries and consolidate configuration documentation

**Success Criteria:**
- Clear separation: REQ (what) vs. ADR/SPEC (how) vs. IMPL (code)
- Configuration hierarchy diagram added
- Flavor characteristic structure consolidated
- Implementation code moved from CLAUDE.md to IMPL docs

### Task 2.1: Audit SPEC001 Communication Patterns

**File:** `docs/SPEC001-architecture.md`

**Action Required:**
1. Read Section 3 "Communication Patterns" in detail
2. Compare with SPEC007-api_design.md and SPEC011-event_system.md
3. Identify duplications or overlaps

**Decision Matrix:**

| Content | SPEC001 Should Contain | SPEC007/SPEC011 Should Contain |
|---------|----------------------|--------------------------------|
| HTTP REST patterns | Overview: "Modules use HTTP REST for synchronous operations" | Detailed endpoint specifications |
| SSE patterns | Overview: "SSE used for real-time updates" | Event schemas and connection handling |
| Event broadcasting | Overview: "tokio::broadcast for internal events" | Complete event type catalog |

**Deliverable:** Update SPEC001 to reference SPEC007/SPEC011 for details if duplication found

**Acceptance Test:**
- No endpoint specifications in SPEC001 (should be SPEC007 only)
- No event schemas in SPEC001 (should be SPEC011 only)
- Clear cross-references added

---

### Task 2.2: Consolidate Flavor Characteristic Structure

**Problem:** SPEC003 and SPEC004 both explain characteristic structure (binary vs. complex dimensions)

**Solution:** Define characteristics in one location, reference from both

**Option A: Create SPEC002a-flavor_characteristics.md**

**New File:** `docs/SPEC002a-flavor_characteristics.md`

**Content Outline:**
```markdown
# SPEC002a: Flavor Characteristics

## Characteristic Types

### Binary Characteristics
- Definition: Single true/false dimension
- Examples: acoustic, electronic, aggressive, calm
- Distance calculation: Symmetric (0 if both have, 0 if neither have, 1 if mismatch)

### Complex Characteristics
- Definition: Multiple sub-dimensions with numeric values
- Example: timbral_texture (brightness, roughness, warmth, depth)
- Distance calculation: Euclidean distance across sub-dimensions

## AcousticBrainz Schema Integration
[Details of AB schema mapping]

## User-Defined Characteristics
[Extension mechanism]
```

**Update SPEC003-musical_flavor.md:**
```markdown
## Flavor Distance Calculation

Musical flavor uses characteristics defined in [SPEC002a-flavor_characteristics.md](SPEC002a-flavor_characteristics.md).

**Distance Formula:** [existing formula with reference to characteristic types]
```

**Update SPEC004-musical_taste.md:**
```markdown
## Taste Calculation

Musical taste aggregates flavors using characteristics defined in [SPEC002a-flavor_characteristics.md](SPEC002a-flavor_characteristics.md).

**Aggregation Formula:** [existing formula with reference]
```

**Option B: Expand SPEC003 with Characteristics Section**

Add Section 1.5 to SPEC003:
```markdown
### 1.5 Characteristic Structure

[Move characteristic definitions here]
```

Update SPEC004 to reference SPEC003 § 1.5.

**Recommendation:** Option A (new file) if characteristics section >100 lines, Option B (expand SPEC003) if <100 lines

**Acceptance Test:**
- Characteristic structure defined in exactly one location
- Both SPEC003 and SPEC004 reference the authoritative definition
- No duplication of binary vs. complex explanations

---

### Task 2.3: Clarify Configuration Hierarchy

**Problem:** Three separate concerns mixed without clear boundaries:
1. Root folder path resolution (where wkmp.db is stored)
2. Module configuration (which port each module binds to)
3. Application settings (playback parameters, user preferences)

**Solution:** Create clear scope documentation and hierarchy diagram

**File:** `docs/IMPL004-deployment.md`

**Add Section 2.0a: Configuration Hierarchy Overview**

```markdown
## 2.0a Configuration Hierarchy Overview

WKMP has three distinct configuration layers:

### Layer 1: Root Folder Resolution (System Bootstrap)

**Purpose:** Determine where `wkmp.db` database file is stored

**Mechanism:** 4-tier priority (CLI > ENV > TOML > Default)

**Documented In:** [ADR-003-zero_configuration_strategy.md](ADR-003-zero_configuration_strategy.md)

**Applies To:** ALL 6 microservices (identical resolution pattern)

**Example:**
```
User runs: wkmp-ui --root-folder /custom/music
Result: Database at /custom/music/wkmp.db
```

---

### Layer 2: Module Configuration (Service Discovery)

**Purpose:** Auto-discover module ports and enable/disable modules

**Mechanism:** Database table `module_config` (auto-populated on first run)

**Documented In:** [IMPL001-database_schema.md](IMPL001-database_schema.md#module_config-table)

**Applies To:** Module startup and inter-module communication

**Example:**
```sql
SELECT port FROM module_config WHERE module_id = 'wkmp-ui';
-- Returns: 5720
```

**Note:** Users should NOT manually edit this table. Defaults are correct for 99% of deployments.

---

### Layer 3: Application Settings (User Preferences)

**Purpose:** Playback parameters, timeslots, user preferences, library paths

**Mechanism:** Database table `settings` (user-editable via UI)

**Documented In:** [IMPL016-settings_reference.md](IMPL016-settings_reference.md)

**Applies To:** Application behavior (not infrastructure)

**Example:**
```sql
UPDATE settings SET value = '{"volume": 0.8}' WHERE key = 'playback.volume';
```

---

### Configuration Flow Diagram

```
[System Start]
     |
     v
[Layer 1: Root Folder Resolution] --> /home/user/Music/wkmp.db
     |
     v
[Layer 2: Module Config Discovery] --> wkmp-ui binds to port 5720
     |
     v
[Layer 3: Application Settings Load] --> Playback volume = 0.8
     |
     v
[Application Running]
```

**Key Principle:** Each layer is independent. Changing root folder doesn't affect module ports. Changing module ports doesn't affect playback settings.
```

**Acceptance Test:**
- Three configuration layers clearly distinguished
- Each layer has single authoritative documentation
- Diagram shows relationship between layers
- No confusion between "root folder resolution" and "application settings"

---

### Task 2.4: Move Implementation Code from CLAUDE.md to IMPL003

**Problem:** CLAUDE.md contains implementation code (zero-config pattern)

**Solution:** Move to IMPL003-project_structure.md

**File:** `docs/IMPL003-project_structure.md`

**Add Section: Zero-Config Startup Pattern**

```markdown
## Zero-Config Startup Pattern

**Applies To:** ALL 6 microservices

**Architecture Decision:** [ADR-003-zero_configuration_strategy.md](ADR-003-zero_configuration_strategy.md)

**Requirements:** [REQ-NF-030] through [REQ-NF-037] in [REQ001-requirements.md](REQ001-requirements.md)

### Implementation Template

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Step 0: Initialize tracing subscriber [ARCH-INIT-003]
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "module_name=debug,wkmp_common=info".into()),
        )
        .with(tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_file(true)
            .with_line_number(true))
        .init();

    // **[ARCH-INIT-004]** Log build identification
    info!(
        "Starting WKMP [Module Name] (module-id) v{} [{}] built {} ({})",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH"),
        env!("BUILD_TIMESTAMP"),
        env!("BUILD_PROFILE")
    );

    // Step 1: Resolve root folder (4-tier priority)
    let resolver = wkmp_common::config::RootFolderResolver::new("module-name");
    let root_folder = resolver.resolve();

    // Step 2: Create directory if missing
    let initializer = wkmp_common::config::RootFolderInitializer::new(root_folder);
    initializer.ensure_directory_exists()?;

    // Step 3: Get database path
    let db_path = initializer.database_path();  // root_folder/wkmp.db

    // Step 4: Initialize database connection
    let db = Database::new(&db_path).await?;

    // Step 5: Continue with module-specific startup
    ...
}
```

### Common Errors

**Error:** Module hardcodes `PathBuf::from("wkmp.db")`

**Fix:** Always use `RootFolderResolver` → `RootFolderInitializer` → `database_path()`

**Error:** Module implements custom root folder resolution

**Fix:** Remove custom code, use `wkmp_common::config` utilities
```

**Update CLAUDE.md reference (from Task 1.2):**
```markdown
**Implementation Example:** See [IMPL003-project_structure.md § Zero-Config Startup Pattern](docs/IMPL003-project_structure.md#zero-config-startup-pattern) for code template.
```

**Acceptance Test:**
- CLAUDE.md contains zero lines of implementation code
- IMPL003 contains complete code template
- Reference link in CLAUDE.md works

---

### Task 2.5: Clarify Requirement vs. Design Scope for Zero-Config

**Problem:** REQ001, ADR-003, and CLAUDE.md mix requirement/decision/implementation

**Solution:** Enforce scope boundaries

**File:** `docs/REQ001-requirements.md`

**Current (lines 276-285):** Contains implementation details (4-tier list, RootFolderResolver)

**Replace With:**
```markdown
### [REQ-NF-030] Zero-Configuration Startup - wkmp-ap

The wkmp-ap module MUST start without requiring configuration files. The system SHALL determine the root folder (database location) automatically with user-overridable defaults.

### [REQ-NF-031] Zero-Configuration Startup - wkmp-ui

The wkmp-ui module MUST start without requiring configuration files. The system SHALL determine the root folder (database location) automatically with user-overridable defaults.

[... REQ-NF-032 through REQ-NF-037 for other modules ...]

**Design Decision:** See [ADR-003-zero_configuration_strategy.md](ADR-003-zero_configuration_strategy.md) for 4-tier priority architecture.

**Implementation:** See [IMPL003-project_structure.md § Zero-Config Startup Pattern](IMPL003-project_structure.md#zero-config-startup-pattern) for code template.
```

**File:** `docs/ADR-003-zero_configuration_strategy.md`

**Verify Contains:**
- Decision: Use 4-tier priority system
- Rationale: User convenience vs. explicit control
- Alternatives considered: Environment-only, TOML-only, hardcoded defaults
- Implementation guidance: RootFolderResolver pattern

**Should NOT Contain:**
- Requirement language ("MUST", "SHALL") → belongs in REQ001
- Implementation code → belongs in IMPL003

**Acceptance Test:**
- REQ001 contains only requirements (MUST/SHALL statements)
- ADR-003 contains only architectural decision
- IMPL003 contains only implementation code
- Clear cross-references connect all three

---

## Phase 3: Documentation Enhancement (8-10 hours)

**Goal:** Add cross-references, visual diagrams, and eliminate remaining overlaps

**Success Criteria:**
- All related documents linked with precise line number references
- Three key diagrams added (entity model, config hierarchy, event flow)
- No overlapping content without explicit "see also" references
- Context window reduction: 30-40% less redundant content

### Task 3.1: Add Entity Relationship Diagram to REQ002

**File:** `docs/REQ002-entity_definitions.md`

**Add Section 2.0: Entity Relationship Overview**

```markdown
## 2.0 Entity Relationship Overview

### Relationship Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         Audio File                              │
│  - file_path (string)                                           │
│  - duration_ms (integer)                                        │
│  - segmentation_strategy (enum)                                 │
└───────────────┬─────────────────────────────────────────────────┘
                │
                │ 1:N (contains)
                │
                v
┌─────────────────────────────────────────────────────────────────┐
│                         Passage                                 │
│  - passage_id (UUID)                                            │
│  - start_ms, end_ms (integer)                                   │
│  - lead_in_ms, lead_out_ms (integer)                            │
│  - musical_flavor (JSON)                                        │
└───────────────┬─────────────────────────────────────────────────┘
                │
                │ N:M (passage-song join)
                │
                v
┌─────────────────────────────────────────────────────────────────┐
│                          Song                                   │
│  - song_id (UUID)                                               │
│  - base_probability (0.0-1.0)                                   │
└───────────────┬─────────────────────────────────────────────────┘
                │
                │ N:1 (references)
                │
                v
┌─────────────────────────────────────────────────────────────────┐
│                      Recording                                  │
│  - recording_mbid (UUID - MusicBrainz)                          │
│  - recording_title (string)                                     │
│  - duration_ms (integer - canonical)                            │
│  - acousticbrainz_data (JSON - source of musical_flavor)        │
└─────────────────────────────────────────────────────────────────┘
                │
                │ N:M (credits)
                │
                v
┌─────────────────────────────────────────────────────────────────┐
│                         Artist                                  │
│  - artist_mbid (UUID - MusicBrainz)                             │
│  - artist_name (string)                                         │
│  - artist_sort_name (string)                                    │
└─────────────────────────────────────────────────────────────────┘

                │
                │ N:M (work_credits)
                │
                v
┌─────────────────────────────────────────────────────────────────┐
│                          Work                                   │
│  - work_mbid (UUID - MusicBrainz)                               │
│  - work_title (string)                                          │
│  - work_type (string - composition, song, etc.)                 │
└─────────────────────────────────────────────────────────────────┘
```

### Cross-System Usage

| Entity | Used By | Purpose |
|--------|---------|---------|
| Passage | wkmp-ap | Playback queue, crossfading |
| Passage | wkmp-pd | Selection algorithm (flavor matching) |
| Song | wkmp-pd | Cooldown tracking (14-day song cooldown) |
| Artist | wkmp-pd | Cooldown tracking (30-minute artist cooldown) |
| Work | wkmp-pd | Cooldown tracking (2-day work cooldown) |
| Recording | wkmp-ai | Metadata source, AcousticBrainz integration |
| Recording | SPEC003 | Musical flavor distance calculation |

### Key Relationships

**1:N (Audio File → Passages)**
- Single-file albums: 1 audio file → 10-20 passages (one per track)
- Multi-file albums: 20 audio files → 20 passages (one per file)

**N:M (Passages ↔ Songs)**
- Most passages: 1 passage → 1 song (typical case)
- Medleys: 1 passage → 2+ songs (multiple recordings in one playable segment)
- Live albums: 2+ passages → 1 song (same song performed twice)

**N:1 (Songs → Recording)**
- Multiple songs (across different passages/albums) reference same Recording MBID

**N:M (Recordings ↔ Artists)**
- Feature credits: 1 recording → 3 artists (main + 2 featured)
- Compilations: 1 artist → 200 recordings

**N:M (Recordings ↔ Works)**
- Covers: 3 recordings → 1 work (original + 2 cover versions)
- Medleys: 1 recording → 4 works (medley contains 4 compositions)
```

**Acceptance Test:**
- Diagram shows all key entities and relationships
- Cross-system usage table clarifies which modules use which entities
- Developers understand entity model without reading 3 documents

---

### Task 3.2: Add Configuration Hierarchy Diagram to IMPL004

**File:** `docs/IMPL004-deployment.md`

**Visual from Task 2.3 expanded:**

```markdown
## 2.0a Configuration Hierarchy Diagram

### Startup Sequence

```
┌─────────────────────────────────────────────────────────────────┐
│ User runs: wkmp-ui --root-folder /music/library                │
└─────────────────────┬───────────────────────────────────────────┘
                      │
                      v
┌─────────────────────────────────────────────────────────────────┐
│ LAYER 1: Root Folder Resolution (4-Tier Priority)              │
│                                                                 │
│  1. CLI arg:    --root-folder /music/library   ✓ MATCH         │
│  2. ENV var:    WKMP_ROOT_FOLDER (not set)                      │
│  3. TOML file:  ~/.config/wkmp/wkmp-ui.toml (not found)        │
│  4. Default:    ~/Music                                         │
│                                                                 │
│  Result: /music/library/                                        │
└─────────────────────┬───────────────────────────────────────────┘
                      │
                      v
┌─────────────────────────────────────────────────────────────────┐
│ LAYER 2: Database Discovery                                    │
│                                                                 │
│  Database path: /music/library/wkmp.db                          │
│  - If exists: Open connection                                   │
│  - If missing: Create with migrations                           │
└─────────────────────┬───────────────────────────────────────────┘
                      │
                      v
┌─────────────────────────────────────────────────────────────────┐
│ LAYER 3: Module Configuration Discovery                        │
│                                                                 │
│  Query: SELECT * FROM module_config WHERE module_id='wkmp-ui'  │
│  Result: {port: 5720, enabled: true, auto_start: true}         │
│                                                                 │
│  Action: wkmp-ui binds to http://localhost:5720                │
└─────────────────────┬───────────────────────────────────────────┘
                      │
                      v
┌─────────────────────────────────────────────────────────────────┐
│ LAYER 4: Application Settings Load                             │
│                                                                 │
│  Query: SELECT * FROM settings                                  │
│  Result: {playback.volume: 0.8, ui.theme: "dark", ...}         │
│                                                                 │
│  Action: Initialize application with user preferences           │
└─────────────────────┬───────────────────────────────────────────┘
                      │
                      v
┌─────────────────────────────────────────────────────────────────┐
│                Application Running                              │
│  - wkmp-ui serving at http://localhost:5720                    │
│  - Database at /music/library/wkmp.db                           │
│  - User settings applied                                        │
└─────────────────────────────────────────────────────────────────┘
```

### Configuration Modification Paths

| What to Change | How to Change | Restart Required |
|----------------|---------------|------------------|
| Root folder location | Set `--root-folder` or `WKMP_ROOT_FOLDER` | Yes (affects all modules) |
| Module port (wkmp-ui) | Update `module_config` table | Yes (module only) |
| Playback volume | Update `settings` table or use UI | No (hot-reload) |
| Theme preference | Update `settings` table or use UI | No (hot-reload) |
| Database path | Cannot change directly (tied to root folder) | N/A |

### Common Misconceptions

**Misconception:** "I need to edit wkmp-ui.toml to change the port"

**Reality:** Module ports are in database (`module_config` table). TOML files only control root folder path.

**Misconception:** "Changing WKMP_ROOT_FOLDER will reset my playback settings"

**Reality:** Application settings are stored in the database. Moving the database file (via root folder change) preserves all settings.
```

**Acceptance Test:**
- Diagram clarifies four distinct layers
- Configuration modification table shows what changes what
- Common misconceptions addressed preemptively

---

### Task 3.3: Add Event System Flow Diagram to SPEC011

**File:** `docs/SPEC011-event_system.md`

**Add Section 1.5: Event Flow Architecture**

```markdown
## 1.5 Event Flow Architecture

### Internal → External Event Bridge

```
┌─────────────────────────────────────────────────────────────────┐
│                    wkmp-ap (Audio Player)                       │
│                                                                 │
│  Audio thread detects passage end                               │
│         |                                                        │
│         v                                                        │
│  event_tx.send(PassageEnded {                                   │
│      passage_id: "abc123",                                      │
│      timestamp: "2026-01-11T..."                                │
│  })                                                             │
└─────────────────────┬───────────────────────────────────────────┘
                      │
                      │ tokio::broadcast channel
                      │ (in-process pub/sub)
                      │
                      v
┌─────────────────────────────────────────────────────────────────┐
│                    wkmp-ui (User Interface)                     │
│                                                                 │
│  SSE Broadcaster subscribes to event_rx                         │
│         |                                                        │
│         v                                                        │
│  let mut rx = event_bus.subscribe();                            │
│  while let Ok(event) = rx.recv().await {                        │
│      for client in sse_clients.iter() {                         │
│          client.send(event.to_json());                          │
│      }                                                           │
│  }                                                               │
└─────────────────────┬───────────────────────────────────────────┘
                      │
                      │ HTTP SSE (Server-Sent Events)
                      │ (cross-process, browser-compatible)
                      │
                      v
┌─────────────────────────────────────────────────────────────────┐
│              Web Browser (JavaScript Client)                    │
│                                                                 │
│  const eventSource = new EventSource('/api/events');            │
│  eventSource.addEventListener('PassageEnded', (e) => {          │
│      const data = JSON.parse(e.data);                           │
│      updateUI(data);                                            │
│  });                                                             │
└─────────────────────────────────────────────────────────────────┘
```

### Event Propagation Example

**Scenario:** User clicks "Enqueue Passage" in wkmp-ui

```
1. [Browser] User clicks button
        |
        v
2. [wkmp-ui] POST /api/queue
        |
        v
3. [wkmp-ui] HTTP request to wkmp-ap: POST /playback/queue
        |
        v
4. [wkmp-ap] Adds passage to queue
        |
        v
5. [wkmp-ap] Broadcasts PassageEnqueued event (tokio::broadcast)
        |
        v
6. [wkmp-ui] SSE Broadcaster receives event (tokio::broadcast subscriber)
        |
        v
7. [wkmp-ui] Pushes event to all SSE clients (HTTP SSE)
        |
        v
8. [Browser] Receives PassageEnqueued event
        |
        v
9. [Browser] Updates queue display (React state update)
```

### Event Types by Module

| Module | Emits Events | Consumes Events |
|--------|--------------|-----------------|
| wkmp-ap | PassageStarted, PassageEnded, QueueUpdated, PlaybackStateChanged | (none - source) |
| wkmp-ui | (relays wkmp-ap events via SSE) | All wkmp-ap events (for SSE broadcast) |
| wkmp-pd | SelectionCompleted | PassageEnded (triggers new selection) |
| wkmp-ai | ImportProgress, AlbumMatched | (none - background process) |

### Key Design Decisions

**Why tokio::broadcast for internal events?**
- In-process pub/sub (no network overhead)
- Multiple subscribers (wkmp-ui SSE broadcaster + internal handlers)
- Backpressure handling (slow subscribers don't block fast emitters)

**Why HTTP SSE for external events?**
- Browser-native (no WebSocket complexity)
- Auto-reconnection built-in
- Text-based (easy debugging)
- Firewall-friendly (standard HTTP)

**Why not WebSockets?**
- SSE is unidirectional (server → client), which matches our event model
- No need for client → server messages over event channel
- Simpler implementation and debugging
```

**Acceptance Test:**
- Diagram shows internal (tokio::broadcast) vs. external (HTTP SSE) clearly
- Event propagation example traces full path from source to browser
- Design decision rationale clarifies architecture choices

---

### Task 3.4: Add Cross-References to Entity-Using Documents

**Files to Update:**

**`docs/SPEC003-musical_flavor.md`**

Add at line 15 (before first entity reference):
```markdown
> **Entity Definitions:** See [REQ002-entity_definitions.md](REQ002-entity_definitions.md) for complete definitions of Passage, Song, and Recording entities referenced in this specification.
```

**`docs/SPEC005-program_director.md`**

Add at beginning of "Cooldown System" section:
```markdown
> **Entity Definitions:** Song, Artist, and Work entities are defined in [REQ002-entity_definitions.md](REQ002-entity_definitions.md). See § 2.0 Entity Relationship Overview for cooldown tracking relationships.
```

**`docs/IMPL001-database_schema.md`**

Add at line 12 (before schema tables):
```markdown
## Conceptual Foundation

This database schema implements the entity model defined in [REQ002-entity_definitions.md](REQ002-entity_definitions.md).

**Key Relationships:**
- `audio_files` (1) → (N) `passages` via `file_id` foreign key
- `passages` (N) ↔ (M) `songs` via `passage_songs` join table
- `songs` (N) → (1) `recordings` via `recording_mbid` foreign key
- `recordings` (N) ↔ (M) `artists` via `credits` join table
- `recordings` (N) ↔ (M) `works` via `work_credits` join table

See [REQ002 § 2.0 Entity Relationship Diagram](REQ002-entity_definitions.md#20-entity-relationship-overview) for visual representation.
```

**Acceptance Test:**
- All entity-using documents reference REQ002
- Developers encounter entity definitions early (before confusion)
- Cross-references use precise section anchors

---

### Task 3.5: Add Cross-References to Cooldown System

**Files to Update:**

**`docs/SPEC001-architecture.md`** (lines 136-137)

Replace:
```markdown
Multi-level cooldowns (song: 14d, artist: 30min, work: 2d)
```

With:
```markdown
Multi-level cooldowns (song: 14d, artist: 30min, work: 2d). See [SPEC005-program_director.md § Cooldown System](SPEC005-program_director.md#cooldown-system) for detailed algorithm.
```

**`docs/REQ001-requirements.md`** (lines 79-85)

Add at end of automatic enqueueing section:
```markdown
**Cooldown Periods:** See [SPEC005-program_director.md § Cooldown System](SPEC005-program_director.md#cooldown-system) for detailed cooldown algorithm (song/artist/work levels).
```

**`docs/IMPL002-coding_conventions.md`** (lines 56, 58)

Replace:
```markdown
Cooldown calculation logic
```

With:
```markdown
Cooldown calculation logic (see [SPEC005 § Cooldown System](../SPEC005-program_director.md#cooldown-system))
```

**Acceptance Test:**
- All cooldown references point to SPEC005 as authoritative source
- No document repeats cooldown algorithm details (only SPEC005)

---

### Task 3.6: Add Precise Line Number References

**Pattern:** Replace generic references with precise line anchors

**Example Before:**
```markdown
See SPEC003-musical_flavor.md for distance calculations
```

**Example After:**
```markdown
See [SPEC003-musical_flavor.md § 3.2 Distance Calculation](SPEC003-musical_flavor.md#32-distance-calculation) for flavor distance formula (lines 145-178)
```

**Files to Audit:** All SPEC and IMPL documents

**Tools:**
- Use grep to find "See SPEC" or "See IMPL" without line numbers/sections
- Update with precise anchors

**Acceptance Test:**
- 90%+ of cross-references include section anchors
- Line number hints included where helpful (long sections)

---

### Task 3.7: Verify No Remaining Exact Duplications

**Method:** Automated duplicate detection

**Script:** `scripts/detect_doc_duplicates.py`

```python
#!/usr/bin/env python3
"""
Detect duplicate content blocks in WKMP documentation.
"""

import hashlib
from pathlib import Path
from collections import defaultdict

def hash_paragraph(text):
    """Hash normalized paragraph text."""
    normalized = ' '.join(text.split())
    return hashlib.md5(normalized.encode()).hexdigest()

def find_duplicates(docs_dir):
    """Find duplicate paragraphs across docs."""
    paragraph_locations = defaultdict(list)

    for doc in Path(docs_dir).rglob("*.md"):
        with open(doc, 'r', encoding='utf-8') as f:
            lines = f.readlines()
            paragraph = []
            start_line = 0

            for i, line in enumerate(lines, 1):
                if line.strip() == '':
                    if paragraph:
                        text = ' '.join(paragraph)
                        if len(text) > 100:  # Only check substantial paragraphs
                            h = hash_paragraph(text)
                            paragraph_locations[h].append({
                                'file': str(doc),
                                'lines': f"{start_line}-{i-1}",
                                'text': text[:100] + "..."
                            })
                        paragraph = []
                else:
                    if not paragraph:
                        start_line = i
                    paragraph.append(line.strip())

    # Print duplicates
    duplicates_found = False
    for h, locations in paragraph_locations.items():
        if len(locations) > 1:
            duplicates_found = True
            print(f"\nDuplicate content found in {len(locations)} locations:")
            for loc in locations:
                print(f"  - {loc['file']}:{loc['lines']}")
                print(f"    {loc['text']}")

    if not duplicates_found:
        print("✓ No duplicate paragraphs found")

if __name__ == '__main__':
    find_duplicates('docs/')
```

**Run Script:**
```bash
python scripts/detect_doc_duplicates.py
```

**Acceptance Test:**
- Script reports zero exact duplications
- Any reported duplicates are intentional (e.g., license headers, repeated warnings)

---

## Testing and Validation

### Test Plan

**Test 1: Link Validation**

```bash
# Check all markdown links are valid
python scripts/validate_doc_links.py docs/

# Expected: All internal links resolve, no 404s
```

**Test 2: Cross-Reference Completeness**

```bash
# Check all entity references have cross-references to REQ002
grep -r "Passage\|Song\|Recording" docs/SPEC*.md docs/IMPL*.md | \
  grep -v "REQ002" | \
  grep -v "# Entity" | \
  wc -l

# Expected: 0 (all entity mentions have REQ002 reference nearby)
```

**Test 3: No Orphaned Sections**

```bash
# Find sections with no incoming references
python scripts/find_orphaned_sections.py docs/

# Expected: <10 orphaned sections (some legitimate)
```

**Test 4: Duplicate Detection**

```bash
# Run duplicate detection script
python scripts/detect_doc_duplicates.py

# Expected: 0 duplicates (or only intentional repeats)
```

**Test 5: Developer Onboarding Simulation**

**Scenario:** New developer needs to understand "zero-config startup"

**Path Test:**
1. Read CLAUDE.md § Zero-Configuration Startup
2. Follow link to ADR-003 (decision rationale)
3. Follow link to IMPL003 (code template)
4. Developer can implement without reading REQ001 or SPEC001

**Expected:** Developer finds complete information in 3 documents (vs 5+ currently)

**Scenario:** Developer needs to understand "musical flavor calculation"

**Path Test:**
1. Read SPEC003-musical_flavor.md
2. Follow link to REQ002 for entity definitions
3. Follow link to SPEC002a (or SPEC003 § 1.5) for characteristic structure
4. Developer understands flavor without reading SPEC004 or IMPL001

**Expected:** Developer finds complete information without duplicates

---

## Acceptance Criteria

### Phase 1 Completion

- [ ] CLAUDE.md reduced by 200+ lines
- [ ] All exact duplicates removed (microservices table, zero-config code, on-demand pattern)
- [ ] All removed content accessible via cross-references
- [ ] No information loss verified

### Phase 2 Completion

- [ ] Configuration hierarchy documented with 3 clear layers
- [ ] Zero-config implementation code moved to IMPL003
- [ ] Flavor characteristic structure consolidated (SPEC002a or SPEC003 expansion)
- [ ] Clear requirement vs. design vs. implementation boundaries
- [ ] SPEC001 Communication Patterns audited for SPEC007/SPEC011 overlap

### Phase 3 Completion

- [ ] Entity relationship diagram added to REQ002
- [ ] Configuration hierarchy diagram added to IMPL004
- [ ] Event system flow diagram added to SPEC011
- [ ] All entity-using documents reference REQ002
- [ ] All cooldown references point to SPEC005
- [ ] 90%+ of cross-references include section anchors
- [ ] Zero duplicate paragraphs (verified by script)
- [ ] Developer onboarding path tests pass

---

## Rollback Plan

**If Phase 1 breaks documentation:**
1. Git revert individual commits
2. Restore CLAUDE.md from backup
3. Keep cross-references added (harmless)

**If Phase 2 creates confusion:**
1. Revert IMPL003 additions
2. Restore original CLAUDE.md zero-config section
3. Keep configuration hierarchy documentation (improves clarity)

**If Phase 3 diagrams are incorrect:**
1. Remove diagrams (code documentation remains functional)
2. Fix diagrams based on developer feedback
3. Re-add corrected diagrams

---

## Maintenance Plan

### Ongoing Documentation Hygiene

**Rule 1: Single Source of Truth**
- Before adding content to a document, search for existing coverage
- If duplicate found, add cross-reference instead of repeating

**Rule 2: Cross-Reference Quality**
- Use section anchors: `[SPEC003 § 3.2](SPEC003.md#32-distance-calculation)`
- Include line hints for long sections: `(lines 145-178)`
- Update references when sections are renumbered

**Rule 3: Change Propagation**
- When updating authoritative source, check all cross-references
- Use `git grep` to find all references to changed content
- Update or remove outdated references

**Rule 4: Quarterly Duplicate Scans**
- Run `scripts/detect_doc_duplicates.py` monthly
- Investigate and consolidate any new duplicates found
- Track duplicate count in project metrics

---

## Success Metrics

**Quantitative:**
- Documentation size reduced by 15-20% (eliminate redundancy)
- Cross-reference count increased by 50+ references
- Duplicate paragraph count: 0 (verified by script)
- Developer onboarding time reduced (measured via survey)

**Qualitative:**
- Developers report "clear single source of truth" (post-implementation survey)
- No confusion about "which document is authoritative" (issue tracker)
- Faster document updates (change velocity measurement)

---

**Status:** Draft - Awaiting Approval
**Estimated Total Effort:** 13-18 hours (1-2h + 4-6h + 8-10h)
**Prerequisites:** None (can start immediately)
**Dependencies:** None (all work self-contained)
