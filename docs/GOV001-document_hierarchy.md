# WKMP Document Hierarchy

> **All project documents reference this hierarchy specification**

---

**⚖️ TIER 0 - GOVERNANCE (META)**

This document defines the documentation framework itself. It sits outside the content hierarchy, establishing the rules for how all other documents relate and are maintained.

**Self-Reference Note:** This document is meta-documentation - it defines the hierarchy and includes itself in that definition at Tier 0 (Governance). It is not part of the requirements/design/implementation content flow; rather, it governs that flow.

---

## Filename Convention

All documentation files follow a systematic naming convention defined in [GOV003-filename_convention.md](GOV003-filename_convention.md). The convention uses prefixes (GOV, REQ, SPEC, IMPL, EXEC, etc.) to indicate document tier and type, making the documentation structure immediately obvious.

**Format:** `PREFIX###-descriptive_name.md`

**Examples:**
- `GOV001-document_hierarchy.md` (Tier 0: Governance)
- `REQ001-requirements.md` (Tier 1: Requirements)
- `SPEC001-architecture.md` (Tier 2: Specifications)
- `IMPL001-database_schema.md` (Tier 3: Implementation)
- `EXEC001-implementation_order.md` (Tier 4: Execution)

**Benefits:**
- Natural alphabetical grouping by document type
- Tier-based organization visible in filenames
- Easy to identify document purpose without opening files
- Scalable for future document additions
- Compatible with all file managers and tools

See [GOV003-filename_convention.md](GOV003-filename_convention.md) for complete specification, prefix definitions, numbering guidelines, and migration procedures.

---

## Document Classification

WKMP project documentation is organized into a strict hierarchy that governs how information flows between documents and when each document should be updated.

### User Documentation vs Technical Documentation

WKMP distinguishes between two major categories of documentation:

**Technical Documentation** (docs/ root level):
- Intended for developers, architects, and contributors
- Follows the 5-tier hierarchy (Tier 0-4 + Tier R)
- Uses formal document naming conventions (GOV###, REQ###, SPEC###, etc.)
- Contains requirements, architecture, implementation specs, and execution plans
- Stored directly in `docs/` directory

**User Documentation** (docs/user/):
- Intended for end users and system administrators
- Focuses on installation, operation, configuration, and troubleshooting
- Uses descriptive filenames (QUICKSTART.md, TROUBLESHOOTING.md, etc.)
- Written in plain language with step-by-step instructions
- Stored in `docs/user/` subdirectory

**Key Principle:** User documentation is derived FROM technical documentation (especially requirements and specifications) but is written at a different level of abstraction. Changes to technical docs may require updates to user docs, but user docs do not influence technical docs.


## Document Purposes and Update Policies

### Tier 0: Governance (Meta)

**Document Registry:** See [REG001 Number Registry](../workflows/REG001_number_registry.md) for complete list, assignment history, and next available GOV number.

#### GOV001-document_hierarchy.md
**Purpose:** Defines the documentation framework and governance rules

**Contains:**
- Document tier classifications
- Update policies for each document
- Information flow rules (upward/downward)
- Change control processes
- Document maintenance procedures

**Update Policy:**
- ✅ Update when document structure changes (new docs added, tiers reorganized)
- ✅ Update when governance processes evolve
- ✅ Update when document relationships need clarification
- ⚠️ This document governs others; changes here affect the entire documentation system

**Change Control:**
- Changes require review by technical lead and documentation lead
- Major changes (new tiers, flow rules) require team consensus
- Document is self-referential but must remain logically consistent

**Maintained By:** Technical lead, documentation lead

---

### Tier R: Review & Change Control (Meta)

**Document Registry:** See [REG001 Number Registry](../workflows/REG001_number_registry.md) for complete list, assignment history, and next available REV number.

These documents track the evolution of WKMP documentation and identify gaps requiring change control review. They are meta-documentation alongside Tier 0 but serve a temporal/historical purpose rather than structural governance.

**Key Principle:** Tier R documents are **descriptive** (recording what happened) rather than **prescriptive** (defining what should happen). They feed the change control process but do not directly update content tiers.

#### REV###-*.md (Revision/Review Documents)

**Purpose:** Record major design decisions, architectural changes, or comprehensive design reviews

**Contains:**
- Design review findings (issue identification, recommendations)
- Architectural change baselines (before/after snapshots)
- Decision rationale and trade-off analysis
- Reference commits and dates
- Impact analysis (requirements satisfied, trade-offs)

**Examples:**
- REV001-wkmp_ap_design_review.md - Quality assurance review identifying 5 critical issues
- REV002-event_driven_architecture_update.md - Architectural change from timer-driven to event-driven
- REV003-dry_guidance_review.md - DRY guidance completeness analysis and enhancement recommendations

**Update Policy:**
- ✅ Created for major architectural changes
- ✅ Created for formal design reviews
- ❌ NEVER updated after creation (historical snapshots)
- ❌ NOT authoritative for current system state (Tier 1-4 documents are authoritative)
- ✅ New REV document created for subsequent changes/reviews

**Change Control:**
- REV documents identify issues and recommend changes
- Recommendations feed change control process
- Accepted recommendations → Update Tier 1-4 documents
- REV documents remain immutable (historical record)

**Maintained By:** Project architect, technical lead

**Authority:** **Historical reference only** - Tier 1-4 documents are authoritative for current system state

#### CHANGELOG-*.md (Change Log Documents)

**Purpose:** Provide detailed file-by-file accounting of documentation changes implementing a decision

**Contains:**
- Date and reference commit
- Summary of changes
- File-by-file modification lists
- Before/after snippets for major changes
- Rationale for each change
- Cross-references to related REV documents

**Examples:**
- [CHANGELOG-event_driven_architecture.md](archive/CHANGELOG-event_driven_architecture.md) - Documents all changes made for REV002 (archived)

**Update Policy:**
- ✅ Created alongside REV documents when changes are extensive
- ❌ NEVER updated after creation (audit trail)
- ✅ May have multiple changelogs for same topic (CHANGELOG-topic-v1.md, CHANGELOG-topic-v2.md)

**Maintained By:** Documentation lead, technical lead

**Authority:** **Audit trail only** - NOT authoritative (Tier 1-4 documents reflect final state)

#### ADDENDUM-*.md (Addendum Documents)

**Purpose:** Provide temporary supplementary clarification or enhancement spanning multiple specifications

**Contains:**
- Enhancement description
- Cross-references to enhanced documents
- Additional details, examples, or guidance
- Rationale for addendum (vs updating original docs)
- Integration notes

**Examples:**
- [ADDENDUM-interval_configurability.md](archive/ADDENDUM-interval_configurability.md) - Enhances event timing configuration documentation (archived)

**Update Policy:**
- ✅ Created when clarification spans multiple documents
- ✅ Created when enhancement is temporary (pending integration)
- ✅ May be updated until integrated into Tier 1-4 documents
- ✅ Should eventually be integrated into authoritative docs and archived
- ✅ After integration: Move to archive/ with note pointing to integrated location

**Maintained By:** Technical lead, documentation lead

**Authority:** **Temporary authoritative** - Valid until content is integrated into Tier 1-4 documents

#### MIGRATION-*.md (Migration/Deprecation Guides - Future)

**Purpose:** Document migration paths from deprecated to current implementations

**Contains:**
- Deprecated element description
- Replacement element description
- Migration steps
- Code examples (before/after)
- Timeline and version support
- Compatibility notes

**Examples (hypothetical):**
- MIGRATION-gstreamer_to_single_stream.md - Guide for migrating from GStreamer to single-stream

**Update Policy:**
- ✅ Created when major deprecations occur
- ✅ Updated during deprecation period
- ✅ Archived when migration complete

**Maintained By:** Technical lead, affected module leads

**Authority:** **Operational guidance** - Authoritative for migration process, not for final state

---

### Tier 1: Authoritative Requirements

**Document Registry:** See [REG001 Number Registry](../workflows/REG001_number_registry.md) for complete list, assignment history, and next available REQ number.

#### REQ001-requirements.md
**Purpose:** Defines WHAT WKMP must do from a product/user perspective

**Contains:**
- Functional requirements (features, behaviors)
- Non-functional requirements (performance, constraints)
- User workflows and use cases
- Version feature matrices (Full/Lite/Minimal)

**Update Policy:**
- ✅ Update when product requirements change (stakeholder-driven)
- ✅ Update when user needs evolve
- ❌ DO NOT update based on design or implementation documents
- ❌ DO NOT update to match implementation convenience

**Change Control:**
- Requirements changes must be intentional
- Driven by product decisions, not technical convenience
- If design/implementation reveals requirement gaps, explicitly review and decide whether to update requirements

**Maintained By:** Product owner, stakeholders, with technical input

#### REQ002-entity_definitions.md
**Purpose:** Defines core entity terminology used across all WKMP documentation

**Contains:**
- Track, Recording, Work, Artist definitions (MusicBrainz entities)
- Song definition (WKMP-specific: Recording + Artist(s))
- Passage definition (WKMP-specific: span of audio with timing points)
- Entity relationships and cardinality rules
- WKMP-specific entity constraints

**Update Policy:**
- ✅ Update when product terminology evolves
- ✅ Update when new entity types are added
- ❌ DO NOT update based on implementation details
- ❌ Definitions are authoritative; implementation adapts to them

**Change Control:**
- Terminology changes must be intentional (affects entire codebase)
- Driven by product clarity needs, not implementation convenience
- New entity types require explicit review of impact on all documents

**Maintained By:** Product owner, technical lead

**Related to:**
- requirements.md references these definitions
- database_schema.md implements these entities as tables
- All documents use this terminology consistently

---

### Tier 2: Design Specifications

**Document Registry:** See [REG001 Number Registry](../workflows/REG001_number_registry.md) for complete list, assignment history, and next available SPEC number.

These documents define HOW requirements are satisfied through design decisions.

#### SPEC001-architecture.md
**Purpose:** Defines system structure, components, and interactions

**Contains:**
- Component architecture
- Communication patterns
- Technology stack decisions
- Concurrency model
- Platform abstractions

**Update Policy:**
- ✅ Update to satisfy requirements.md
- ✅ Update when design decisions change
- ✅ May inform new requirements (via proper change control)
- ❌ DO NOT contradict requirements.md without explicit requirement change

**Maintained By:** Technical lead, architects

#### SPEC007-api_design.md
**Purpose:** Defines REST API structure and Server-Sent Events interface

**Contains:**
- REST API endpoint specifications
- Request/response formats
- SSE event streaming design
- Error response formats
- API versioning strategy

**Update Policy:**
- ✅ Update to satisfy requirements.md API requirements
- ✅ Update when API structure changes
- ✅ May propose requirement changes if API design needs adjustment
- ❌ Must support requirements.md features, not drive them

**Maintained By:** API designer, technical lead

#### SPEC008-library_management.md
**Purpose:** Defines file scanning, metadata extraction, and MusicBrainz integration workflows

**Contains:**
- File discovery and scanning algorithms
- Metadata extraction processes
- Audio fingerprinting workflow
- MusicBrainz/AcousticBrainz integration
- Multi-passage file handling
- Lyrics input/storage design

**Update Policy:**
- ✅ Update to satisfy requirements.md library requirements
- ✅ Update when metadata workflows change
- ✅ May propose requirement changes if workflow needs adjustment
- ❌ Must support requirements.md features, not drive them

**Maintained By:** Library subsystem lead, technical lead

#### SPEC002-crossfade.md
**Purpose:** Defines crossfade timing system and behavior

**Contains:**
- Six timing point definitions
- Fade curve specifications (exponential, cosine, linear)
- Crossfade behavior cases
- Volume calculation formulas

**Update Policy:**
- ✅ Update to satisfy playback requirements from requirements.md
- ✅ Update when timing/fade algorithms are refined
- ✅ May propose requirement changes if audio behavior needs adjustment
- ❌ Must remain consistent with requirements.md playback behaviors

**Maintained By:** Audio engineer, technical lead

#### SPEC003-musical_flavor.md
**Purpose:** Defines musical flavor characterization and distance calculation system

**Contains:**
- AcousticBrainz data structure
- Distance calculation formulas
- Flavor mapping rules (single-song, multi-song, zero-song passages)
- Usage in selection algorithm

**Update Policy:**
- ✅ Update to satisfy selection requirements from requirements.md
- ✅ Update when flavor algorithms are refined
- ✅ May propose requirement changes if selection behavior needs adjustment
- ❌ Must remain consistent with requirements.md selection system

**Maintained By:** Algorithm designer, technical lead

#### SPEC004-musical_taste.md
**Purpose:** Defines musical taste calculation from likes and dislikes

**Contains:**
- Simple taste calculation (centroid of flavors)
- Weighted taste calculation
- Time-based taste weighting (time of day, day of week, day of year, lunar phase)
- Taste as selection target

**Update Policy:**
- ✅ Update to satisfy taste-based selection requirements from requirements.md
- ✅ Update when taste algorithms are refined
- ✅ May propose requirement changes if taste behavior needs adjustment
- ❌ Must remain consistent with requirements.md and musical_flavor.md

**Maintained By:** Algorithm designer, technical lead

#### SPEC005-program_director.md
**Purpose:** Defines automatic passage selection algorithm (the "Program Director")

**Contains:**
- Selection request processing and target time calculation
- Musical flavor target determination (timeslot-based and temporary overrides)
- Five-stage selection algorithm (filtering, distance calculation, ranking, candidate selection, weighted random)
- Cooldown system (minimum and ramping periods for songs, artists, works)
- Base probability calculations
- User-configurable parameters
- Integration with musical taste (future)

**Update Policy:**
- ✅ Update to satisfy automatic selection requirements from requirements.md
- ✅ Update when selection algorithm is refined
- ✅ May propose requirement changes if selection behavior needs adjustment
- ❌ Must remain consistent with requirements.md, musical_flavor.md, and musical_taste.md

**Maintained By:** Algorithm designer, technical lead

#### SPEC006-like_dislike.md
**Purpose:** Defines like and dislike functionality and data collection

**Contains:**
- Like/Dislike user interactions (Full Like/Dislike, Lite Like/Dislike)
- Data storage per user UUID
- Integration with musical taste calculation
- UI controls and behaviors

**Update Policy:**
- ✅ Update to satisfy user feedback requirements from requirements.md
- ✅ Update when like/dislike mechanisms evolve
- ✅ May propose requirement changes if user interaction needs adjustment
- ❌ Must remain consistent with requirements.md and musical_taste.md

**Maintained By:** UX designer, technical lead

#### SPEC009-ui_specification.md
**Purpose:** Defines Web UI design, layout, and behavior

**Contains:**
- Authentication flow UI
- Playback controls and queue management
- Now Playing information display (passage title, song info, album artwork)
- Network status indicators
- Responsive design and accessibility
- Multi-user coordination UI considerations

**Update Policy:**
- ✅ Update to satisfy UI/UX requirements from requirements.md
- ✅ Update when UI design evolves
- ✅ May propose requirement changes if user experience needs adjustment
- ❌ Must remain consistent with requirements.md and api_design.md

**Maintained By:** UX designer, frontend lead, technical lead

#### SPEC010-user_identity.md
**Purpose:** Defines user identity, authentication, and account management

**Contains:**
- Three authentication modes (Anonymous, Create Account, Login)
- Browser-based UUID persistence (one-year rolling expiration)
- User account creation and login workflows
- Anonymous user handling
- Multi-user session coordination

**Update Policy:**
- ✅ Update to satisfy authentication requirements from requirements.md
- ✅ Update when authentication mechanisms evolve
- ✅ May propose requirement changes if security/UX needs adjustment
- ❌ Must remain consistent with requirements.md and api_design.md

**Maintained By:** Security lead, backend lead, technical lead

#### SPEC011-event_system.md
**Purpose:** Defines event-driven communication architecture

**Contains:**
- EventBus design and implementation
- Event type enumeration (WkmpEvent)
- Communication pattern selection (broadcast vs channels vs shared state)
- Event flows and examples
- Testing patterns for event-driven code

**Update Policy:**
- ✅ Update to satisfy multi-user and real-time requirements from requirements.md
- ✅ Update when new event types are needed
- ✅ Update when communication patterns evolve
- ❌ Must support requirements.md features, not drive them

**Maintained By:** Software architect, technical lead

#### SPEC012-multi_user_coordination.md
**Purpose:** Defines the mechanism for coordinating actions from multiple users

**Contains:**
- Detailed specification for handling concurrent user actions
- The role and payload of the `UserAction` event
- Sequence diagrams for each edge case (skip-throttling, concurrent queue operations, lyric editing)

**Update Policy:**
- ✅ Update to satisfy multi-user requirements from requirements.md
- ✅ Update when new multi-user edge cases are identified
- ❌ Must remain consistent with requirements.md and api_design.md

**Maintained By:** Software architect, technical lead

#### SPEC013-single_stream_playback.md
**Purpose:** Defines single-stream audio playback architecture

**Contains:**
- Component architecture (decoder, buffer, mixer, output)
- Passage buffer specifications
- Crossfade mixer implementation
- Performance characteristics

**Update Policy:**
- ✅ Update to satisfy playback requirements from requirements.md
- ✅ Update when playback architecture evolves
- ❌ Must remain consistent with requirements.md and crossfade.md

**Maintained By:** Audio engineer, technical lead

#### SPEC014-single_stream_design.md
**Purpose:** Defines single-stream audio architecture design decisions

**Contains:**
- Motivation for single-stream approach
- Decoder pool sizing and management
- Buffer management strategies
- Performance optimization

**Update Policy:**
- ✅ Update to satisfy playback requirements from requirements.md
- ✅ Update when design decisions evolve
- ❌ Must remain consistent with requirements.md and architecture.md

**Maintained By:** Audio engineer, technical lead

#### SPEC016-decoder_buffer_design.md
**Purpose:** Defines decoder-buffer chain architecture and dataflow

**Contains:**
- Operating parameters (working_sample_rate, buffer sizes, decode scheduling)

See [SPEC016 Operating Parameters](SPEC016-decoder_buffer_design.md#operating-parameters) ([DBD-PARAM-010] through [DBD-PARAM-100]).

- Decoder-buffer chain component specifications
- Backpressure mechanisms
- Resampling, fade application, and mixer behavior
- Sample format specifications

**Update Policy:**
- ✅ Update to satisfy playback requirements from requirements.md
- ✅ Update when decoder-buffer architecture evolves
- ✅ Must remain consistent with single_stream_playback.md and crossfade.md
- ❌ Must not contradict requirements.md without explicit requirement change

**Maintained By:** Audio engineer, technical lead

#### SPEC017-sample_rate_conversion.md
**Purpose:** Defines sample rate conversion and tick-based timing system

**Contains:**
- Tick-based timing system design (LCM approach)
- Supported audio sample rates
- Tick-to-sample and sample-to-tick conversion formulas
- Database storage format (integer ticks)
- API representation format (milliseconds)
- Working sample rate integration
- Dual timing system coexistence

**Update Policy:**
- ✅ Update to satisfy timing precision requirements from requirements.md
- ✅ Update when timing system design evolves
- ✅ Must remain consistent with decoder_buffer_design.md and database_schema.md
- ❌ Must not contradict requirements.md without explicit requirement change

**Maintained By:** Audio engineer, technical lead

#### SPEC018-crossfade_completion_coordination.md
**Purpose:** Defines coordination between decoder and mixer for crossfade completion detection

**Contains:**
- Event-driven crossfade completion notification
- Buffer chain completion events
- Timing coordination between components

**Update Policy:**
- ✅ Update to support crossfade and audio architecture requirements
- ❌ Must not contradict requirements.md

**Maintained By:** Audio engineer, technical lead

#### SPEC019-sse_based_developer_ui.md
**Purpose:** Defines Server-Sent Events (SSE) architecture for real-time UI updates

**Contains:**
- SSE event types and formats
- Real-time progress updates
- Developer UI requirements

**Update Policy:**
- ✅ Update to support UI and event system requirements
- ❌ Must not contradict requirements.md

**Maintained By:** UI lead, backend developers

#### SPEC020-developer_ui_design.md
**Purpose:** Defines developer-focused UI for Audio Ingest module

**Contains:**
- Web-based UI specifications
- Real-time progress visualization
- Import workflow user experience

**Update Policy:**
- ✅ Update to support Audio Ingest requirements
- ❌ Must not contradict requirements.md

**Maintained By:** UI lead, UX designer

#### SPEC021-error_handling.md
**Purpose:** Defines error handling patterns and recovery strategies

**Contains:**
- Error categorization
- Recovery strategies
- User-facing error messages

**Update Policy:**
- ✅ Update to support reliability requirements
- ❌ Must not contradict requirements.md

**Maintained By:** Technical lead

#### SPEC022-performance_targets.md
**Purpose:** Defines performance goals and measurement criteria

**Contains:**
- Response time targets
- Resource usage limits
- Performance testing methodology

**Update Policy:**
- ✅ Update to support non-functional requirements
- ❌ Must not contradict requirements.md

**Maintained By:** Technical lead, performance engineer

#### SPEC023-timing_terminology.md
**Purpose:** Defines timing terminology and conventions across WKMP

**Contains:**
- Timing measurement units
- Terminology standardization
- Cross-module timing contracts

**Update Policy:**
- ✅ Update to clarify timing concepts
- ❌ Must not contradict requirements.md

**Maintained By:** Audio engineer, technical lead

#### SPEC024-audio_ingest_architecture.md
**Purpose:** Defines architecture for Audio Ingest module (wkmp-ai)

**Contains:**
- Microservice architecture
- Import workflow design
- File scanning and metadata extraction
- MusicBrainz integration

**Update Policy:**
- ✅ Update to support Audio Ingest requirements
- ❌ Must not contradict requirements.md

**Maintained By:** Backend developers, audio engineer

#### SPEC025-amplitude_analysis.md
**Purpose:** Defines amplitude analysis for crossfade timing

**Contains:**
- Peak amplitude detection
- Crossfade timing recommendations
- Audio analysis algorithms

**Update Policy:**
- ✅ Update to support crossfade requirements
- ❌ Must not contradict requirements.md

**Maintained By:** Audio engineer

#### SPEC026-api_key_configuration.md
**Purpose:** Defines multi-tier API key configuration system

**Contains:**
- Database, ENV, TOML resolution priority
- Automatic key migration and write-back
- Security considerations
- Web UI integration for settings

**Update Policy:**
- ✅ Update to support configuration requirements
- ✅ Update when adding new API keys
- ❌ Must not contradict requirements.md

**Maintained By:** Backend developers, security lead

---

### Tier 3: Implementation Specifications

**Document Registry:** See [REG001 Number Registry](../workflows/REG001_number_registry.md) for complete list, assignment history, and next available IMPL number.

These documents translate design into concrete implementation details.

#### IMPL001-database_schema.md
**Purpose:** Defines data structures and database schema

**Contains:**
- Table definitions
- Relationships and foreign keys
- Indexes and triggers
- Data type specifications
- Migration strategy

**Update Policy:**
- ✅ Update to support requirements.md and design documents
- ✅ Update when data model needs refinement
- ✅ May inform design document updates if schema reveals design issues
- ❌ Schema is derived FROM requirements/design, not vice versa

**Maintained By:** Database engineer, backend developers

#### IMPL002-coding_conventions.md
**Purpose:** Defines code organization, style, and quality standards

**Contains:**
- Module organization patterns
- Code style guidelines
- Testing requirements
- Documentation standards
- Requirement traceability conventions

**Update Policy:**
- ✅ Update to support requirements.md and design patterns
- ✅ Update when coding standards evolve
- ✅ Update based on team decisions and best practices
- ❌ Conventions serve the architecture, not vice versa

**Maintained By:** Technical lead, development team

#### IMPL005-audio_file_segmentation.md
**Purpose:** Defines workflow for segmenting a single audio file into multiple passages

**Contains:**
- Passage boundary detection and definition
- User interface for defining passage start/end points
- Timing point configuration per passage
- Multi-passage file handling workflow
- Validation and constraints

**Update Policy:**
- ✅ Update to support library_management.md and requirements.md
- ✅ Update when segmentation workflow needs refinement
- ✅ May inform design document updates if workflow reveals design issues
- ❌ Workflow is derived FROM requirements/design, not vice versa

**Maintained By:** Library subsystem lead, UX designer, technical lead

#### ARCH002-gstreamer_design.md
**Purpose:** Defines GStreamer pipeline architecture and implementation details

**Contains:**
- Dual pipeline architecture (Pipeline A/B with audiomixer)
- GStreamer element specifications and configurations
- Crossfade implementation using volume automation
- Audio device enumeration and selection
- Buffer management and synchronization
- Platform-specific considerations (Linux/Windows/macOS)

**Update Policy:**
- ✅ Update to support crossfade.md and architecture.md audio requirements
- ✅ Update when GStreamer implementation details need refinement
- ✅ May inform design document updates if pipeline reveals design issues
- ❌ Pipeline specs are derived FROM requirements/design, not vice versa

**Maintained By:** Audio engineer, backend developers

#### IMPL003-project_structure.md
**Purpose:** Defines Rust workspace structure and project organization

**Contains:**
- Cargo workspace configuration
- Module/crate organization (wkmp-ap, wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le)
- Shared library organization (wkmp-common)
- Dependency management strategy
- Testing infrastructure layout
- Build configuration

**Update Policy:**
- ✅ Update to support architecture.md microservices design
- ✅ Update when new modules are added or removed
- ✅ Update when shared code organization needs refinement
- ❌ Project structure is derived FROM requirements/design, not vice versa

**Maintained By:** Technical lead, build engineer

#### IMPL004-deployment.md
**Purpose:** Defines deployment, process management, and operational configuration

**Contains:**
- Module binary locations and configuration file paths
- Configuration file format for each module (TOML)
- Root folder location and database access
- Startup order and module dependencies
- Process management (systemd, launchd, Windows Services)
- Health checks and monitoring
- Graceful shutdown procedures
- Backup and recovery processes
- Version-specific deployments (Full, Lite, Minimal)
- Security considerations and network architecture

**Update Policy:**
- ✅ Update to support architecture.md microservices design
- ✅ Update when deployment strategy changes
- ✅ Update when new modules are added or removed
- ✅ May inform design document updates if deployment reveals operational issues
- ❌ Deployment specs are derived FROM requirements/design, not vice versa

**Maintained By:** DevOps lead, technical lead

---

### Tier 4: Execution Plan

**Document Registry:** See [REG001 Number Registry](../workflows/REG001_number_registry.md) for complete list, assignment history, and next available EXEC number.

#### EXEC001-implementation_order.md
**Purpose:** Defines WHEN features are built and in what sequence

**Contains:**
- Phase-by-phase implementation plan
- Task breakdown with dependencies
- References to all upstream specifications
- Requirement ID and coding convention mappings
- Critical path and blockers

**Update Policy:**
- ✅ ALWAYS update when any upstream document changes
- ✅ Update when implementation sequence needs adjustment
- ✅ Update when blockers are resolved
- ❌ NEVER update upstream documents based on this document
- ❌ This is a DOWNSTREAM AGGREGATOR only

**Maintained By:** Project manager, technical lead

---

### Cross-Cutting: Process & Standards

#### GOV002-requirements_enumeration.md
**Purpose:** Defines requirement ID scheme for traceability

**Contains:**
- ID format specifications (DOC-CAT-NNN)
- Document codes (REQ, ARCH, XFD, FLV, DB, etc.)
- Category codes for each document
- Numbering guidelines and hierarchy
- Usage examples

**Update Policy:**
- ✅ Update when new documents are added
- ✅ Update when ID scheme needs refinement
- ✅ Applied TO all documents, but doesn't define their content
- ❌ Provides structure, not substance

**Maintained By:** Technical lead, documentation lead

---

## Information Flow Rules

### Governance Flow (Meta-level)
```
document_hierarchy.md (Tier 0)
    ↓ governs structure and policies for ↓
All other documents (Tiers R, 1-4, Cross-cutting)
```

**Rule:** document_hierarchy.md governs all other documents but is not influenced by their content. It defines the framework; they provide the substance.

### Review & Change Control Flow (Meta-level)
```
Tier 1-4 (Content Documents)
    ↓ Major change or review needed
REV###-*.md (Design review or architectural change)
    ├─> CHANGELOG-*.md (Detailed change tracking)
    ├─> ADDENDUM-*.md (Temporary clarifications)
    └─> Change Control Process
        ├─> Accept → Update Tier 1-4 documents
        └─> Reject → Document reason in REV

Tier 1-4 (Content Documents)
    ↓ Issue/gap discovered during implementation
REV###-*.md (Issue identification)
    └─> Recommendations feed Change Control
        ├─> Accept → Update Tier 1-4 documents
        └─> REV document remains immutable (historical record)
```

**Rule:** Tier R documents are **descriptive** (historical snapshots) not **prescriptive** (authoritative content). They feed change control but do not directly update content tiers.

### Downward Flow (Normal)
```
requirements.md + entity_definitions.md (Tier 1)
    ↓ Design satisfies requirements and uses terminology
architecture.md, api_design.md, library_management.md, crossfade.md,
musical_flavor.md, musical_taste.md, program_director.md, like_dislike.md,
ui_specification.md, user_identity.md, event_system.md, multi_user_coordination.md (Tier 2)
    ↓ Implementation specs support design
database_schema.md, coding_conventions.md, audio_file_segmentation.md,
gstreamer_design.md, project_structure.md, deployment.md (Tier 3)
    ↓ Execution plan aggregates all specs
implementation_order.md (Tier 4)
```

**Rule:** Lower-tier documents are updated when higher-tier documents change.

### Upward Flow (Controlled)
```
implementation_order.md (Tier 4)
    ↑ Discovers gap/issue
database_schema.md, coding_conventions.md, audio_file_segmentation.md,
gstreamer_design.md, project_structure.md, deployment.md (Tier 3)
    ↑ May reveal design issue
architecture.md, api_design.md, library_management.md, crossfade.md,
musical_flavor.md, musical_taste.md, program_director.md, like_dislike.md,
ui_specification.md, user_identity.md, event_system.md, multi_user_coordination.md (Tier 2)
    ↑ May reveal requirement or terminology gap (via change control)
requirements.md + entity_definitions.md (Tier 1)
    ↑ Does NOT affect (governance is separate)
document_hierarchy.md (Tier 0)
```

**Rule:** Upward flow requires explicit review and approval. Implementation discoveries don't automatically change requirements or governance.

### Cross-Tier References

**Allowed:**
- Design documents (Tier 2) MAY reference each other
  - Example: architecture.md references crossfade.md for audio engine design
  - Example: event_system.md references architecture.md for component communication

- Implementation specs (Tier 3) MAY reference Tier 2 documents
  - Example: coding_conventions.md references event_system.md for async patterns
  - Example: database_schema.md references musical_flavor.md for vector storage

**Not Allowed:**
- Tier 1 (requirements.md) NEVER references lower-tier documents
- Tier 4 (implementation_order.md) NEVER influences higher-tier documents

## Change Control Process

### Requirements Changes (Tier 1)

1. **Identify Need:** Implementation/design reveals requirement gap or conflict
2. **Document Issue:** Create issue describing the gap with context
3. **Review:** Product owner + technical lead review
4. **Decision:** Accept as requirement change OR reject (design must adapt)
5. **Update:** If accepted, update requirements.md with change tracking
6. **Cascade:** Update all affected downstream documents

**Example:**
```
Issue: Cooldown period for 'Works' is not defined (REQ-SEL-060)
Review: Is this a requirement gap or can we proceed with a default?
Decision: Requirement gap - product needs to define this behavior.
Update: Add cooldown times to requirements.md under 'Cooldown System'.
Cascade: Update implementation_order.md to remove the 'specification needed' blocker.
```

### Design Changes (Tier 2)

1. **Identify Need:** Implementation reveals design issue
2. **Verify:** Does this violate requirements.md? (If yes, see Requirements Changes)
3. **Propose:** Document design change with rationale
4. **Review:** Technical lead + relevant stakeholders review
5. **Update:** Update design document(s)
6. **Cascade:** Update Tier 3 & 4 documents

**Example:**
```
Issue: GStreamer pipeline design doesn't handle FLAC crossfades efficiently
Verify: Doesn't violate playback requirements, just performance
Propose: Modify pipeline architecture for better buffering
Update: Modify architecture.md audio engine section
Cascade: Update implementation_order.md phase 3 tasks
```

### Implementation Spec Changes (Tier 3)

1. **Identify Need:** Implementation detail needs adjustment
2. **Verify:** Does this violate design or requirements? (If yes, escalate)
3. **Update:** Update implementation spec document
4. **Cascade:** Update implementation_order.md

**Example:**
```
Issue: SQLite JSON1 query performance issue
Verify: Doesn't violate data model requirements
Update: Modify database_schema.md to add indexes
Cascade: Update implementation_order.md with optimization task
```

## Document Maintenance Checklist

### When Creating New Features

- [ ] Start with requirements.md: Does this satisfy a requirement?
- [ ] Update design docs (Tier 2): How will this be designed?
- [ ] Update implementation specs (Tier 3): What concrete details are needed?
- [ ] Update implementation_order.md (Tier 4): When/where in the plan?
- [ ] Add requirement IDs following GOV002-requirements_enumeration.md

### When Discovering Issues

- [ ] Identify tier: Is this requirement/design/implementation issue?
- [ ] Follow upward flow if needed: Does higher tier need updating?
- [ ] Get approval before updating higher-tier documents
- [ ] Update affected document(s)
- [ ] Cascade changes downward to all affected documents

### When Reviewing Pull Requests

- [ ] Verify code satisfies requirements.md
- [ ] Verify code follows architecture.md design
- [ ] Verify code follows coding_conventions.md
- [ ] Verify requirement IDs are present (per GOV002-requirements_enumeration.md)
- [ ] Check if implementation_order.md needs updating

## Document Size and Structure Standards

**Effective:** 2025-10-25
**Status:** Active (approved Phase 2, PLAN003)
**Rationale:** Context window optimization for AI and human readers (PLAN003)

### Size Thresholds

**For documents 300-1200 lines:**
- **Required:** Executive summary section (<300 lines) at document beginning
- **Required:** Clear section headers with brief summaries
- **Optional:** Modular file structure (single file acceptable)

**For documents >1200 lines:**
- **MANDATORY:** Modular folder structure
- **Folder format:** `[document_name]/`
- **Structure:**
  ```
  [document_name]/
  ├── 00_SUMMARY.md               # <500 lines - READ THIS FIRST
  ├── 01_[topic].md               # <300 lines per section
  ├── 02_[topic].md               # <300 lines per section
  ├── ...
  └── FULL_DOCUMENT.md            # Consolidated (archival/search only)
  ```

**Exemptions:**
- README.md files
- Quick reference guides (<100 lines)
- Templates
- User-facing documentation (separate structure rules apply)

### Progressive Disclosure Requirements

**All modular documents MUST provide:**
- Summary with navigation links to detailed sections
- Each section self-contained (brief context, content, references)
- Full document used only for archival, comprehensive review, or search

**Target reading loads:**
- Quick overview: Summary only (~300-500 lines)
- Focused reading: Summary + 1-2 relevant sections (~600-1100 lines)
- Comprehensive: FULL_DOCUMENT.md (only when necessary)

### Templates

**Use:** [templates/modular_document/](../../templates/modular_document/)
- README.md - Usage guide
- 00_SUMMARY.md - Executive summary template
- 01_section_template.md - Section file template

### Enforcement

**Workflow integration:**
- `/doc-name` workflow checks document size and recommends structure
- `/think` and `/plan` workflows output modular structures automatically
- Code reviews verify new documents follow standards

**Migration:**
- Legacy documents (GOV001, SPEC001-017, REQ001-002) grandfathered
- Refactoring to modular structure optional (defer to Phase 3 if context issues persist)
- New documents created after 2025-10-25 MUST follow these standards

## Common Pitfalls to Avoid

❌ **Don't:** Update requirements.md because implementation is easier a different way<br/>
✅ **Do:** Update design/implementation to satisfy requirements as written, or formally propose requirement change

❌ **Don't:** Let implementation_order.md define new requirements<br/>
✅ **Do:** Use implementation_order.md to discover requirement gaps, then update requirements.md

❌ **Don't:** Update architecture.md to match what was accidentally implemented<br/>
✅ **Do:** Fix implementation to match architecture, or formally update architecture with review

❌ **Don't:** Create circular references between documents<br/>
✅ **Do:** Follow strict hierarchy: higher tiers inform lower tiers, never reverse

❌ **Don't:** Skip change control for "small" requirement changes<br/>
✅ **Do:** All requirement changes go through review, no matter how small

## Document Update Summary

| Document | Tier | Updates When | Influences | Updated By |
|----------|------|--------------|------------|------------|
| document_hierarchy.md | 0 (Meta) | Documentation structure changes | Governs all documents | Tech lead, doc lead |
| REV###-*.md | R (Review) | Never (snapshot) | Change control process | Project architect, tech lead |
| CHANGELOG-*.md | R (Review) | Never (audit trail) | None (historical record) | Doc lead, tech lead |
| ADDENDUM-*.md | R (Review) | Until integrated | Tier 1-4 (temporary) | Tech lead, doc lead |
| MIGRATION-*.md | R (Review) | During deprecation period | Migration process | Tech lead, module leads |
| requirements.md | 1 | Product needs change | All content documents | Product owner |
| entity_definitions.md | 1 | Entity terminology changes | Tier 2, 3, 4 | Product owner, tech lead |
| architecture.md | 2 | Design decisions change | Tier 3, 4 | Tech lead |
| api_design.md | 2 | API structure changes | Tier 3, 4 | API designer |
| library_management.md | 2 | Library workflow changes | Tier 3, 4 | Library subsystem lead |
| crossfade.md | 2 | Audio design changes | Tier 3, 4 | Audio engineer |
| musical_flavor.md | 2 | Flavor algorithm changes | Tier 3, 4 | Algorithm designer |
| musical_taste.md | 2 | Taste algorithm changes | Tier 3, 4 | Algorithm designer |
| program_director.md | 2 | Selection algorithm changes | Tier 3, 4 | Algorithm designer |
| like_dislike.md | 2 | Like/Dislike UX changes | Tier 3, 4 | UX designer |
| ui_specification.md | 2 | UI design changes | Tier 3, 4 | UX designer, frontend lead |
| user_identity.md | 2 | Authentication design changes | Tier 3, 4 | Security lead, backend lead |
| event_system.md | 2 | Communication design changes | Tier 3, 4 | Architect |
| multi_user_coordination.md | 2 | Multi-user edge case changes | Tier 3, 4 | Architect |
| single_stream_playback.md | 2 | Playback architecture changes | Tier 3, 4 | Audio engineer |
| single_stream_design.md | 2 | Single-stream design changes | Tier 3, 4 | Audio engineer |
| decoder_buffer_design.md | 2 | Decoder-buffer chain changes | Tier 3, 4 | Audio engineer |
| sample_rate_conversion.md | 2 | Timing system changes | Tier 3, 4 | Audio engineer |
| database_schema.md | 3 | Data model changes | Tier 4 | DB engineer |
| coding_conventions.md | 3 | Standards evolve | Tier 4 | Tech lead |
| audio_file_segmentation.md | 3 | Segmentation workflow changes | Tier 4 | Library subsystem lead, UX designer |
| gstreamer_design.md | 3 | GStreamer pipeline changes | Tier 4 | Audio engineer |
| project_structure.md | 3 | Workspace structure changes | Tier 4 | Tech lead, build engineer |
| deployment.md | 3 | Deployment strategy changes | Tier 4 | DevOps lead |
| implementation_order.md | 4 | Any upstream change | None (downstream only) | Project manager |
| GOV002-requirements_enumeration.md | Cross-cutting | ID scheme changes | ID format in all docs | Doc lead |
| **User Documentation (docs/user/)** | N/A | Requirements/specs change | None (derived only) | Documentation lead |

## Quick Reference: "Should I Update This Document?"

### I found a gap/issue in document_hierarchy.md
→ Does this affect documentation structure/governance? → Review with tech lead and doc lead
→ Major changes (new tiers, flow rules) → Requires team consensus
→ This affects the entire documentation system, proceed carefully

### I found a gap/issue in implementation_order.md
→ Update implementation_order.md directly (it's downstream)

### I found a gap/issue in any Tier 3 implementation spec
(database_schema.md, coding_conventions.md, audio_file_segmentation.md, gstreamer_design.md, project_structure.md, deployment.md)
→ Can I fix it without changing design? Yes → Update directly
→ Does it affect design? → Review with tech lead, may need Tier 2 update

### I found a gap/issue in any Tier 2 design document
(architecture.md, api_design.md, library_management.md, crossfade.md, musical_flavor.md, musical_taste.md, program_director.md, like_dislike.md, ui_specification.md, user_identity.md, event_system.md, multi_user_coordination.md)
→ Can I fix it without changing requirements? Yes → Update with review
→ Does it affect requirements? → Must go through requirements change control

### I found a gap/issue in requirements.md
→ MUST go through formal change control with product owner
→ This is a product decision, not a technical decision

### GOV002-requirements_enumeration.md needs updating
→ Update when adding new documents or categories
→ Inform all document maintainers of ID scheme changes

---

**Document Version:** 1.6-DRAFT
**Last Updated:** 2025-10-25
**Maintained By:** Technical Lead

**Change Log:**
- v1.6 (2025-10-25): Added "Document Size and Structure Standards" section (Phase 2 approved per PLAN003)
  - Added size thresholds for documents (300-1200 lines, >1200 lines)
  - Defined modular folder structure requirements for large documents
  - Specified progressive disclosure requirements
  - Added template references for modular documents
  - Defined enforcement and migration strategy
  - Added DRAFT note at document top
- v1.5 (2025-10-19): Added SPEC016 and SPEC017 documents
  - Added SPEC013-single_stream_playback.md to Tier 2 section
  - Added SPEC014-single_stream_design.md to Tier 2 section
  - Added SPEC016-decoder_buffer_design.md to Tier 2 section
  - Added SPEC017-sample_rate_conversion.md to Tier 2 section
  - Updated Document Update Summary table with all four documents
  - Established decoder-buffer chain architecture specifications
  - Established tick-based timing system specifications
- v1.4 (2025-10-18): Added User Documentation section and guidance
  - Created docs/user/ folder for user-facing documentation
  - Distinguished between technical docs (developers) and user docs (end users)
  - Added user documentation entry to document update summary table
  - Established principle: user docs derived FROM technical docs
- v1.3 (2025-10-18): Added Tier R (Review & Change Control) classification
  - Added REV###-*.md (Revision/Review Documents)
  - Added CHANGELOG-*.md (Change Log Documents)
  - Added ADDENDUM-*.md (Addendum Documents)
  - Added MIGRATION-*.md (Migration/Deprecation Guides)
  - Updated information flow section with Tier R flow diagram
  - Updated document summary table to include Tier R documents
  - Formalized historical review and change control document classification
- v1.2 (2025-10-10): Added missing Tier 3 documents: gstreamer_design.md, project_structure.md (deployment.md was already documented in text but not in diagram)
- v1.1 (2025-10-08): Added missing documents: musical_taste.md, program_director.md, like_dislike.md, ui_specification.md, user_identity.md, audio_file_segmentation.md
- v1.0 (2025-10-05): Initial version

For questions about document hierarchy or update procedures, consult the technical lead or refer to this specification.

----
End of document - WKMP Document Hierarchy
