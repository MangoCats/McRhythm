# WKMP Requirements Enumeration Scheme

> **Related Documentation:** [Requirements](REQ001-requirements.md) | [Architecture](SPEC001-architecture.md) | [Database Schema](IMPL001-database_schema.md) | [Crossfade Design](SPEC002-crossfade.md) | [Musical Flavor](SPEC003-musical_flavor.md) | [Implementation Order](EXEC001-implementation_order.md) | [Document Hierarchy](GOV001-document_hierarchy.md)

---

**⚖️ TIER 0 - GOVERNANCE (META)**

This document defines the requirement ID enumeration scheme for traceability across all WKMP documentation. It governs how requirements, specifications, and implementation details are uniquely identified and cross-referenced throughout the documentation set.

---

## Overview

This document defines the comprehensive enumeration scheme for all requirements, design specifications, and architectural decisions across the WKMP documentation set. The scheme provides unique, maintainable, compact identifiers that enable precise requirement tracking, traceability, and cross-referencing.

## Enumeration Format

### Primary Format: `DOC-CAT-NNN`

**Components:**
- `DOC` = 2-4 letter document code (identifies source document)
- `CAT` = 2-4 letter category/section code (identifies section within document)
- `NNN` = Three-digit number incremented by 10 (010, 020, 030, 040...) allowing future insertions

**Examples:**
- `REQ-CF-010` - Requirements document, Core Features section, item 010
- `XFD-TP-020` - Crossfade document, Timing Points section, item 020
- `DB-PASS-030` - Database Schema document, Passages table section, item 030

### Sub-requirement Formats

**Level 1 Sub-requirements:** Add single digit
- Format: `DOC-CAT-NNN#` (where # is 1-9)
- Example: `REQ-CF-011`, `REQ-CF-012`, `REQ-CF-013`
- Use: Direct child requirements or sub-items

**Level 2 Sub-requirements:** Add letter suffix
- Format: `DOC-CAT-NNN#L` (where L is A-Z)
- Example: `REQ-SEL-031A`, `REQ-SEL-031B`
- Use: Nested requirements under level 1

**Level 3 Sub-requirements:** Add digit after letter
- Format: `DOC-CAT-NNN#LN` (where N is 1-9)
- Example: `REQ-CF-061A1`, `REQ-CF-061A2`
- Use: Deeply nested requirements (rare, avoid if possible)

### Alternative Sub-item Notation

For simple lists where hierarchy is clear from context:
- Use bullet points without separate IDs
- Parent requirement ID covers all sub-items
- Example: `REQ-API-010` covers all endpoints listed beneath it

## Document Codes

| Code | Document | Purpose |
|------|----------|---------|
| REQ | requirements.md | Functional and non-functional requirements |
| ENT | entity_definitions.md | Core entity terminology and relationships |
| ARCH | architecture.md | System architecture and component design |
| API | api_design.md | REST API and SSE interface specifications |
| LIB | library_management.md | File scanning and metadata workflows |
| XFD | crossfade.md | Crossfade timing and behavior specifications |
| MFL | musical_flavor.md | Musical flavor characterization definitions |
| MTA | musical_taste.md | Musical taste and preference definitions |
| EVT | event_system.md | Event-driven communication architecture |
| DB | database_schema.md | Database schema specifications |
| CODE | coding_conventions.md | Code organization and quality standards |
| IMPL | implementation_order.md | Implementation phases and dependencies |
| MUC | multi_user_coordination.md | Multi-user coordination mechanisms |
| LD | like_dislike.md | Like and Dislike functionality |
| UID | user_identity.md | User identity and authentication |
| AFS | audio_file_segmentation.md | Audio file segmentation workflow |
| SSD | single-stream-design.md | Single stream audio architecture design |

## Category Codes by Document

### REQ (requirements.md)

| Code | Section | Scope |
|------|---------|-------|
| OV | Overview | High-level system description |
| CF | Core Features | Primary functional capabilities |
| VER | Three Versions | Full/Lite/Minimal version specifications |
| TECH | Technical Requirements | Platform and technology stack |
| DEF | Definitions | Terminology and concept definitions |
| FLV | Musical Flavor | Musical flavor target system |
| NF | Non-functional Requirements | Error handling, logging, performance |
| PI | Passage Identification | File discovery and library management |
| FD | File Discovery | Initial file scanning |
| META | Metadata Extraction | Tag reading and parsing |
| FP | Fingerprinting | Audio fingerprinting (Chromaprint) |
| MPF | Multi-passage Files | Multiple passages per file handling |
| MB | MusicBrainz Integration | MusicBrainz API integration |
| LYR | Lyrics | Lyrics input and storage |
| PB | Playback Behaviors | Playback control and behavior |
| XFD | Crossfade Handling | Crossfade behavior (references crossfade.md) |
| PAUS | Pause Behavior | Fade-in when resuming from pause |
| SEL | Automatic Selection | Passage selection algorithm |
| PROB | Base Probabilities | Base probability system |
| QUE | Queue Management | Queue operations and management |
| UQ | User Queue | Manual user queue additions |
| HIST | Play History | Play event recording |
| ART | Album Art | Album art handling |
| PF | Player Functionality | Player controls and capabilities |
| CTL | Manual Controls | User control buttons and actions |
| UI | Web UI | Web interface specifications |
| DISP | Status Display | UI display elements |
| CPAN | Control Panel | UI control elements |
| NEXT | Next Up Queue | Queue display in UI |
| API | API Endpoints | REST API endpoint specifications |
| PERS | State Persistence | State saving and loading |
| ERR | Error Handling | Error handling strategy |
| NET | Network Errors | Network-specific error handling |

### ENT (entity_definitions.md)

| Code | Section | Scope |
|------|---------|-------|
| MB | MusicBrainz | MusicBrainz entities (Track, Recording, Work, Artist) |
| MP | WKMP | WKMP-specific entities (Song, Passage) |
| REL | Relationships | Entity relationships and cardinality |
| CARD | Cardinality | Cardinality rules |
| CNST | Constraints | WKMP-specific entity constraints |

### ARCH (architecture.md)

| Code | Section | Scope |
|------|---------|-------|
| OV | Overview | Architecture overview |
| SYS | System Architecture | Layered architecture description |
| COMP | Core Components | Component descriptions |
| PC | Playback Controller | Playback controller component |
| VOL | Volume | How volume is defined |
| SNGC | Song Change Notification | Realtime notification of system components that a song has started, or ended. |
| PD | Program Director | Program director component |
| BM | Backup Manager | Database backup schedule and rotation |
| QM | Queue Manager | Queue manager component |
| QP | Queue Persistence | Preservation of queue contents through shutdown into next session |
| STRT | Startup Procedures | Session startup restoration of state and related procedures |
| HIST | Historian | Historian component |
| FM | Flavor Manager | Flavor manager component |
| AE | Audio Engine | Audio engine architecture |
| LM | Library Manager | Library manager component |
| EXT | External Integration | External API clients |
| IMPL | Implementation Details | Component implementation specifics |
| CONC | Concurrency Model | Threading and async architecture |
| THR | Threading | Thread architecture |
| COM | Communication | Inter-component communication |
| DATA | Data Model | Data model overview |
| DES | Design Decisions | Key design decisions |
| VER | Version Differentiation | Full/Lite/Minimal differences |
| PLAT | Platform Abstraction | Platform-specific abstractions |
| AUDV | Audio Device | Audio output platform handling |
| INT | System Integration | Auto-start and system integration |
| SEC | Security | Security considerations |
| USER | User Identification | User Identification specifics |
| PERF | Performance Targets | Performance goals |
| ERRH | Error Handling | Error handling strategy |
| CAT | Error Categories | Error categorization |
| LOG | Logging | Logging strategy |
| DEP | Deployment | Deployment considerations |
| MIG | Migrations | Database migration strategy |
| CONF | Configuration | Configuration management |
| DIST | Distribution | Distribution and packaging |
| FUT | Future Considerations | Future architecture plans |

### API (api_design.md)

| Code | Section | Scope |
|------|---------|-------|
| OV | Overview | API overview |
| REST | REST Endpoints | REST API endpoint definitions |
| PB | Playback Control | Playback control endpoints |
| QUE | Queue Management | Queue operation endpoints |
| FB | User Feedback | Like/dislike endpoints |
| LYR | Lyrics | Lyrics endpoints |
| LIB | Library Management | Import/scan endpoints |
| OUT | Audio Output | Audio sink selection endpoints |
| SSE | Server-Sent Events | SSE endpoint and event streaming |
| ERR | Error Responses | Error response formats |
| CORS | CORS Policy | Cross-origin resource sharing |
| VER | API Versioning | Versioning strategy |
| IMPL | Implementation Notes | API implementation details |

### LIB (library_management.md)

| Code | Section | Scope |
|------|---------|-------|
| OV | Overview | Library management overview |
| DISC | File Discovery | File scanning and traversal |
| INIT | Initial Scan | First-time library scan |
| INCR | Incremental Scan | Subsequent scans and change detection |
| META | Metadata Extraction | Tag parsing and extraction |
| TAG | Tag Parsing | Format-specific tag reading |
| ART | Cover Art | Cover art extraction and storage |
| FP | Fingerprinting | Audio fingerprinting workflow |
| CHROMA | Chromaprint | Chromaprint integration |
| ACID | AcoustID | AcoustID API integration |
| MB | MusicBrainz | MusicBrainz API integration |
| REC | Recording Lookup | Recording metadata lookup |
| WORK | Work Lookup | Work metadata lookup |
| COV | Cover Art Fetch | External cover art fetching |
| MPF | Multi-Passage Files | Multi-passage file handling |
| UI | User Workflow | Multi-passage editor UI |
| SIL | Silence Detection | Automatic boundary detection |
| ASSOC | MusicBrainz Association | Passage-to-MusicBrainz mapping |
| AB | AcousticBrainz | AcousticBrainz integration |
| ESS | Essentia | Local Essentia analysis |
| LYR | Lyrics Input | Lyrics editor interface |
| PROG | Progress Reporting | Import progress and events |
| TEST | Testing | Test considerations |

### XFD (crossfade.md)

| Code | Section | Scope |
|------|---------|-------|
| OV | Overview | Crossfade overview |
| TP | Timing Points | Six timing point definitions |
| PT | Point Definitions | Individual point descriptions |
| DUR | Durations | Duration calculations |
| CONS | Constraints | Timing constraints |
| CURV | Fade Curves | Fade curve profiles |
| EXP | Exponential/Logarithmic | Exponential curve specification |
| COS | Cosine | Cosine curve specification |
| LIN | Linear | Linear curve specification |
| BEH | Crossfade Behavior | Crossfade timing behavior |
| C1 | Case 1 | Longer lead-in duration case |
| C2 | Case 2 | Shorter lead-in duration case |
| C3 | Case 3 | No overlap case |
| FADE | Fade Behavior | Fade behavior during crossfade |
| VOL | Volume Calculations | Volume mixing calculations |
| PAUS | Resume from Pause | Pause/resume fade behavior |
| DEF | Default Configuration | Default timing values |
| DB | Database Storage | Database field storage |
| UIX | UI Considerations | User interface considerations |
| VIS | Visual Editor | Visual editor requirements |
| VAL | Validation | User input validation |
| EDGE | Edge Cases | Edge case handling |
| FIRST | First Passage | First passage in queue |
| LAST | Last Passage | Last passage in queue |
| SKIP | User Skip | Skip during crossfade |
| PAUSE | Pause | Pause during crossfade |
| QMOD | Queue Modification | Queue changes during crossfade |

### MFL (musical_flavor.md)

| Code | Section |
|------|---------|
| DEF | Quantitative Definition |
| UDEF | Additional characteristics |
| DIST | Flavor Distance |
| CALC | Single Recording Calculation |
| EDGE | Edge Cases |
| MULT | More than one Recording per Passage |
| USE | Usage of Musical Flavor |
| TARG | Taste as Selection Target |
| ALGO | Selection Algorithm |

### LD (like_dislike.md)

| Code | Section |
|------|---------|
| DESC | Description |
| LIKE | Likes and Dislikes |
| APPL | Applying Likes and Dislikes to Passages |


### MTA (musical_taste.md)

| Code | Section |
|------|---------|
| DESC | Description |
| SMPL | Simple Taste |
| WGHT | Weighted Taste |
| TIME | Time |
| TOD | Time of Day |
| DOW | Day of Week |
| DOY | Day of Year |
| LUN | Phase of the Moon |

### DB (database_schema.md)

| Code | Section | Scope |
|------|---------|-------|
| OV | Overview | Schema overview |
| VER | Schema Version | schema_version table |
| CORE | Core Entities | Core entity tables |
| FILE | Files | files table |
| PASS | Passages | passages table |
| SONG | Songs | songs table |
| ART | Artists | artists table |
| WORK | Works | works table |
| ALB | Albums | albums table |
| REL | Relationships | Relationship tables |
| PS | Passage-Songs | passage_songs table |
| PA | Passage-Albums | passage_albums table |
| SW | Song-Works | song_works table |
| PLAY | Playback & History | Playback-related tables |
| HIST | Play History | play_history table |
| LIKE | Likes/Dislikes | likes_dislikes table |
| QUE | Queue | queue table |
| TIME | Time-based Flavor | Timeslot system tables |
| TS | Timeslots | timeslots table |
| TSP | Timeslot Passages | timeslot_passages table |
| CONF | Configuration | Configuration tables |
| SET | Settings | settings table |
| CACHE | External Caching | External API cache tables |
| ACID | AcoustID Cache | acoustid_cache table |
| MBC | MusicBrainz Cache | musicbrainz_cache table |
| ABC | AcousticBrainz Cache | acousticbrainz_cache table |
| TRIG | Triggers | Database triggers |
| UPD | Update Triggers | Update timestamp triggers |
| COOL | Cooldown Triggers | Last-played update triggers |
| NOTE | Notes | Design notes and considerations |
| TYPE | Data Types | SQLite data type usage |
| UUID | UUID Keys | UUID primary key strategy |
| FLV | Flavor Vectors | Musical flavor vector storage |
| MIG | Migration | Migration strategy |
| PERF | Performance | Performance considerations |
| VERX | Version-Specific | Version-specific table usage |
| FUT | Future | Future schema enhancements |

### IMPL (implementation_order.md)

| Code | Section | Scope |
|------|---------|-------|
| P1 | Phase 1 | Foundation phase |
| P1-1 | Database Schema | Database schema implementation |
| P1-2 | File Scanner | File scanner implementation |
| P1-3 | Playback Engine | Simple playback engine |
| P1-4 | API & UI | Basic REST API and UI |
| P2 | Phase 2 | Core player features phase |
| P2-5 | Queue Management | Queue implementation |
| P2-6 | Play History | History tracking |
| P2-7 | SSE | Real-time updates |
| P2-8 | Album Art | Album art handling |
| P3 | Phase 3 | Crossfade & advanced playback phase |
| P3-9 | Boundary Editor | Passage boundary editor |
| P3-10 | Crossfade Engine | Dual-pipeline crossfade |
| P3-11 | Multi-passage | Multi-passage file handling |
| P4 | Phase 4 | External integration phase |
| P4-12 | Fingerprinting | Audio fingerprinting |
| P4-13 | MusicBrainz | MusicBrainz integration |
| P4-14 | AcousticBrainz | AcousticBrainz integration |
| P4-15 | Flavor Position | Musical flavor calculation |
| P5 | Phase 5 | Musical flavor selection phase |
| P5-16 | Distance Calc | Distance calculation |
| P5-17 | Time-of-day | Time-of-day flavor system |
| P5-18 | Base Probability | Base probability system |
| P5-19 | Cooldown | Cooldown system |
| P5-20 | Selection Algorithm | Flavor-based selection |
| P5-21 | Auto Queue | Automatic queue replenishment |
| P6 | Phase 6 | User feedback & refinement phase |
| P6-22 | Like/Dislike | Like/dislike functionality |
| P6-23 | ListenBrainz | ListenBrainz integration |
| P6-24 | Lyrics | Lyrics functionality |
| P7 | Phase 7 | Platform support & versions phase |
| P7-25 | Startup | Platform-specific startup |
| P7-26 | Audio Sink | Audio sink selection |
| P7-27 | Version Builds | Full/Lite/Minimal builds |
| P8 | Phase 8 | Polish & optimization phase |
| P8-28 | RPi Optimization | Raspberry Pi optimization |
| P8-29 | Error Handling | Error handling improvements |
| P8-30 | UI/UX | UI/UX refinements |
| P8-31 | Testing | Comprehensive testing |
| OPT | Optional | Optional/future enhancements |
| OPT-32 | Visualizations | Advanced visualizations |
| OPT-33 | Advanced | Advanced features |
| OPT-34 | Mobile | Mobile platforms |
| CRIT | Critical Path | Critical path dependencies |

### MUC (multi_user_coordination.md)

| Code | Section | Scope |
|------|---------|-------|
| OV | Overview | High-level overview |
| EVT | UserAction Event | The `UserAction` event definition |
| SPEC | Edge Case Specifications | General specification for edge cases |
| SKIP | Skip Throttling | Skip throttling mechanism |
| QUE | Concurrent Queue Removal | Concurrent queue removal mechanism |
| LYR | Concurrent Lyric Editing | Concurrent lyric editing mechanism |

### SSD (single-stream-design.md)

| Code | Section | Scope |
|------|---------|-------|
| OV | Overview | Architecture overview and motivation |
| ARCH | Architecture | High-level design and component structure |
| DEC | Decoder | Decoding and decode-and-skip strategy |
| BUF | Buffer Management | Buffer management strategies and policies |
| FBUF | Full Buffer | Full decode strategy for current/next passages |
| PBUF | Partial Buffer | Partial buffer strategy for queued passages |
| UND | Underrun Handling | Buffer underrun detection and recovery |
| FADE | Fade Application | Fade curve application timing |
| MIX | Crossfade Mixer | Crossfade mixing and sample-accurate timing |
| CLIP | Clipping Detection | Crossfade summation clipping detection |
| OUT | Audio Output | Audio output thread and device interface |
| LOG | Logging | Logging requirements and diagnostics |
| FLOW | Data Flow | Complete playback sequence and timing |
| PERF | Performance | Performance characteristics and optimization |
| TEST | Testing | Testing strategy and coverage |

## Numbering Guidelines

### Increment by 10 Rule

**Primary requirement numbers increment by 10:**
- Allows insertion of up to 9 new requirements between any two existing requirements
- Example: Between `REQ-CF-010` and `REQ-CF-020`, can insert `REQ-CF-011`, `REQ-CF-012`, etc.
- Preserves logical ordering without renumbering

**When to create sub-requirements (add digit) vs new requirement (increment by 10):**
- **Sub-requirement:** Directly elaborates or qualifies the parent requirement
  - Example: `REQ-CF-010` → `REQ-CF-011`, `REQ-CF-012` (different aspects of same feature)
- **New requirement:** Stands alone as separate requirement
  - Example: `REQ-CF-010` → `REQ-CF-020` (different feature)

### Hierarchy Depth

**Recommended maximum depth: 3 levels**
- Level 0: `REQ-CF-010` (primary)
- Level 1: `REQ-CF-011` (sub-item)
- Level 2: `REQ-CF-011A` (nested sub-item)
- Level 3: `REQ-CF-011A1` (deeply nested - avoid if possible)

**When hierarchy becomes too deep:**
- Consider splitting into multiple top-level requirements
- Use prose references instead of formal nesting
- Create cross-references between related requirements

## Usage Examples

### Simple Requirement

```markdown
[REQ-CF-010] Plays passages from local files (.mp3 and similar)
```

### Requirement with Sub-items

```markdown
[REQ-CF-030] Cross references passages to the MusicBrainz database for:
  - [REQ-CF-031] identification of the song(s) contained in the passage
  - [REQ-CF-032] identification of other relationships that may influence selection
```

### Multi-level Hierarchy

```markdown
[REQ-CF-060] Web-based UI
  - [REQ-CF-061] Primary mode of operation is automatic, without user intervention
    - [REQ-CF-061A] Auto-start on boot (of Linux / Windows / OS-X systems)
      - [REQ-CF-061A1] systemd service on Linux
      - [REQ-CF-061A2] Task scheduler launched service on Windows
      - [REQ-CF-061A3] launchd on OS-X
```

### Cross-document References

```markdown
[REQ-PB-010] Each passage has six configurable timing points that control crossfade behavior. See XFD-TP-010 through XFD-TP-060 for detailed timing point specifications.

[ARCH-PC-020] Implements three fade profiles as specified in XFD-CURV-010, XFD-CURV-020, and XFD-CURV-030.

[DB-PASS-015] The `passages` table stores timing values as defined in XFD-DUR-010 through XFD-DUR-040.
```

### Specification Gaps (TBD)

```markdown
[REQ-PROB-050] Passage base probability calculation

[DB-WORK-020] Work cooldown defaults (TBD: specification needed, see REQ-SEL-060)
```

## Integration in Documentation

### Markdown Formatting

**Use bracket format for requirement IDs:**
```markdown
[REQ-CF-010] Requirement text here
```

**Inline references use code formatting:**
```markdown
The crossfade timing (see `XFD-BEH-C1`) depends on lead-in duration (`XFD-DUR-030`).
```

### Table Integration

```markdown
| ID | Requirement | Priority |
|----|-------------|----------|
| REQ-CF-010 | Plays passages from local files | P0 |
| REQ-CF-020 | Records when passages are played | P0 |
| REQ-CF-030 | Cross references to MusicBrainz | P1 |
```

### Implementation Tracking

```markdown
## Phase 1: Foundation

**Status:** In Progress

- [x] IMPL-P1-1: Database schema (`DB-VER-010` through `DB-TRIG-030`)
- [x] IMPL-P1-2: File scanner (`REQ-FD-010` through `REQ-FD-050`)
- [ ] IMPL-P1-3: Playback engine (`REQ-PB-010` through `REQ-PB-040`)
- [ ] IMPL-P1-4: REST API (`REQ-API-010` through `REQ-API-090`)
```

## Benefits of This Scheme

### Unique Identification
- Globally unique across entire documentation set
- Hierarchical structure prevents collisions
- Document source immediately identifiable

### Maintainability
- Increment-by-10 allows insertions without renumbering
- Letter/number suffixes handle unlimited nesting
- Easy to reorganize within sections

### Compactness
- Typical length: 10-12 characters
- Examples: `REQ-CF-010`, `ARCH-PC-020`, `XFD-CURV-010`
- Short enough for inline references
- Memorable patterns

### Traceability
- Requirements → Architecture: `REQ-CF-010` → `ARCH-PC-015`
- Architecture → Database: `ARCH-PC-020` → `DB-PASS-020`
- Implementation → Requirements: `IMPL-P3-10` → `REQ-PB-010`, `XFD-BEH-010`
- Tests → Requirements: `test_crossfade_case1()` → `XFD-BEH-C1`

### Cross-referencing
- Easy to reference in commit messages: "Implements REQ-CF-010, REQ-CF-020"
- Issue tracking: "Bug in XFD-BEH-C2 implementation"
- Code comments: `// Satisfies REQ-SEL-031A (7-day song cooldown)`
- Test names: `test_req_sel_031a_song_cooldown_default()`

## Migration Strategy

### Phase 1: Document Creation
- [x] Create this enumeration specification document

### Phase 2: Requirements Document
- [ ] Apply enumeration to requirements.md
- [ ] Create mapping table of old text → new IDs
- [ ] Update all internal cross-references

### Phase 3: Design Documents
- [ ] Apply enumeration to crossfade.md
- [ ] Apply enumeration to musical_flavor.md
- [ ] Update cross-references from requirements.md

### Phase 4: Architecture & Schema
- [ ] Apply enumeration to architecture.md
- [ ] Apply enumeration to database_schema.md
- [ ] Update all cross-document references

### Phase 5: Implementation Order
- [ ] Apply enumeration to implementation_order.md
- [ ] Map implementation phases to requirement IDs
- [ ] Create traceability matrix

### Phase 6: Code Integration
- [ ] Add requirement IDs to code comments
- [ ] Update test names to reference requirement IDs
- [ ] Create requirements traceability report

## Tooling Opportunities

### Automated Validation
- Script to verify all references point to valid IDs
- Detect duplicate IDs
- Check numbering gaps and suggest insertions
- Validate hierarchy depth

### Traceability Matrix
- Generate requirement → implementation mapping
- Identify uncovered requirements
- Track requirement status (specified, implemented, tested)

### Documentation Generation
- Extract all requirements by category
- Generate requirement catalog
- Create cross-reference index
- Produce coverage reports

### Version Control Integration
- Git hooks to validate requirement references in commits
- Automated changelog generation based on requirement IDs
- Pull request templates with requirement ID fields

## Maintenance Guidelines

### Adding New Requirements

1. **Identify appropriate document and category**
2. **Find insertion point in numbering sequence**
3. **Assign next available number (increment by 10 if possible)**
4. **Update any cross-references**
5. **Add to traceability matrix if maintained**

### Deprecating Requirements

**Do not delete or renumber:**
- Mark as deprecated: `[REQ-CF-035] [DEPRECATED] Old requirement text`
- Add deprecation note: `[REQ-CF-035] [DEPRECATED as of 2025-10-15] Use REQ-CF-037 instead`
- Preserve in documentation for historical tracking

### Splitting Requirements

**When a requirement becomes too complex:**
1. Keep original ID for primary concept
2. Extract sub-concepts to new sub-requirement IDs
3. Add note: `[REQ-CF-040] [Split into REQ-CF-041, REQ-CF-042, REQ-CF-043 as of 2025-10-15]`

### Merging Requirements

**When requirements are redundant:**
1. Keep lower-numbered ID as canonical
2. Mark higher-numbered as merged: `[REQ-CF-045] [MERGED into REQ-CF-040]`
3. Update all references to use canonical ID

## Review and Approval

**Document Status:** Draft
**Version:** 1.0
**Last Updated:** 2025-10-05
**Author:** Claude Code

**Pending Decisions:**
- Confirm category code assignments for all sections
- Validate numbering conventions with stakeholders
- Establish tooling requirements for automated validation
- Define process for requirement ID assignment authority

**Next Steps:**
1. Review this specification
2. Apply enumeration to requirements.md (pilot)
3. Validate usefulness and adjust scheme if needed
4. Roll out to remaining documents
5. Establish maintenance process

----
End of document - WKMP Requirements Enumeration Scheme
