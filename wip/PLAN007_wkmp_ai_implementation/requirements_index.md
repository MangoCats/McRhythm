# PLAN007: Requirements Index

**Source:** [SPEC032-audio_ingest_architecture.md](../../docs/SPEC032-audio_ingest_architecture.md)
**Requirements Found:** 26
**Document Size:** 501 lines
**Extraction Date:** 2025-10-28

---

## Requirements Summary

| Category | Count | Description |
|----------|-------|-------------|
| **Overview** | 1 | Module identity and purpose |
| **Microservices** | 1 | Integration with other WKMP modules |
| **UI Architecture** | 3 | Web UI and wkmp-ui integration |
| **Database** | 1 | Database access and tables |
| **Components** | 1 | Component responsibility matrix |
| **Workflow** | 2 | State machine and persistence |
| **Async Processing** | 2 | Background jobs and parallelization |
| **Progress Updates** | 2 | SSE and polling endpoints |
| **Integration** | 3 | SPEC008, IMPL005, tick-based timing |
| **Error Handling** | 2 | Error categorization and reporting |
| **Performance** | 2 | Performance targets and optimizations |
| **Security** | 2 | Input validation and API key storage |
| **Testing** | 3 | Unit, integration, and E2E test requirements |
| **Future** | 1 | Out-of-scope enhancements |

**Total:** 26 requirements

---

## Requirements by Priority

### P0 - Critical (Must Have for MVP)

| Req ID | Brief Description | Line | Category |
|--------|-------------------|------|----------|
| AIA-OV-010 | Module identity: Audio Ingest microservice | 13 | Overview |
| AIA-MS-010 | Microservices integration (wkmp-ui primary) | 34 | Integration |
| AIA-UI-010 | Web UI architecture (Axum server, routes) | 47 | UI |
| AIA-UI-020 | wkmp-ui integration (health check, launch button) | 58 | UI |
| AIA-DB-010 | Database access (tables written/read) | 73 | Database |
| AIA-COMP-010 | Component responsibilities matrix | 123 | Architecture |
| AIA-WF-010 | Import workflow state machine (7 states) | 143 | Workflow |
| AIA-WF-020 | Session state persistence (in-memory) | 189 | Workflow |
| AIA-ASYNC-010 | Background job processing (Tokio tasks) | 205 | Async |
| AIA-SSE-010 | Server-Sent Events for progress | 254 | Events |
| AIA-POLL-010 | Polling fallback endpoint | 305 | Events |
| AIA-INT-010 | SPEC008 workflow implementation | 340 | Integration |
| AIA-INT-020 | IMPL005 segmentation workflow | 354 | Integration |
| AIA-INT-030 | Tick-based timing (28,224,000 ticks/sec) | 365 | Integration |
| AIA-ERR-010 | Error categorization (severity levels) | 376 | Error Handling |
| AIA-ERR-020 | Error reporting (SSE + polling + logs) | 386 | Error Handling |
| AIA-SEC-010 | Input validation (paths, parameters) | 431 | Security |
| AIA-TEST-010 | Unit test coverage requirements | 449 | Testing |
| AIA-TEST-020 | Integration test requirements | 457 | Testing |

**P0 Count:** 19 requirements

### P1 - High Priority (Should Have)

| Req ID | Brief Description | Line | Category |
|--------|-------------------|------|----------|
| AIA-UI-030 | Import completion and return navigation | 64 | UI |
| AIA-ASYNC-020 | File processing parallelization (4 workers) | 229 | Async |
| AIA-PERF-010 | Performance targets (100 files in 2-5 min) | 403 | Performance |
| AIA-PERF-020 | Performance optimizations (caching, rate limits) | 418 | Performance |
| AIA-SEC-020 | Secure API key storage (credentials table) | 438 | Security |
| AIA-TEST-030 | E2E test requirements | 464 | Testing |

**P1 Count:** 6 requirements

### P3 - Future Enhancements (Out of Scope for MVP)

| Req ID | Brief Description | Line | Category |
|--------|-------------------|------|----------|
| AIA-FUTURE-010 | Future enhancements (ML identification, etc.) | 474 | Future |

**P3 Count:** 1 requirement

---

## Detailed Requirements Table

| Req ID | Priority | Type | Description | Input | Output | Line # | Dependencies |
|--------|----------|------|-------------|-------|--------|--------|--------------|
| **AIA-OV-010** | P0 | Functional | wkmp-ai module identity and purpose | User music collection | Passages with MusicBrainz metadata | 13 | None |
| **AIA-MS-010** | P0 | Integration | Integration with wkmp-ui (launch point) | - | HTTP/SSE communication | 34 | wkmp-ui:5720 |
| **AIA-UI-010** | P0 | UI | Web UI with Axum (routes: /, /import-progress, /segment-editor, /api/*) | HTTP requests | HTML/JSON responses | 47 | Axum, HTML/CSS/JS |
| **AIA-UI-020** | P0 | UI | wkmp-ui health check and launch button | Health check request | Import wizard in new tab | 58 | wkmp-ui health endpoint |
| **AIA-UI-030** | P1 | UI | Import completion screen with return link | Import session complete | "Return to WKMP" button | 64 | wkmp-ui:5720 |
| **AIA-DB-010** | P0 | Database | Shared SQLite access (write 9 tables, read settings) | - | Database operations | 73 | SQLite, IMPL001 schema |
| **AIA-COMP-010** | P0 | Architecture | Component responsibility matrix (9 services) | - | Component interfaces | 123 | See table lines 123-135 |
| **AIA-WF-010** | P0 | Workflow | 7-state workflow (SCANNING→...→COMPLETED) | Import request | State transitions | 143 | State machine design |
| **AIA-WF-020** | P0 | Workflow | In-memory session state (UUID, state, progress) | Session ID | Session data | 189 | Tokio, UUID |
| **AIA-ASYNC-010** | P0 | Async | Tokio background tasks for import | Import params | Session ID | 205 | Tokio, broadcast channel |
| **AIA-ASYNC-020** | P1 | Async | Parallel file processing (4 concurrent workers) | File list | Processed files | 229 | futures::stream |
| **AIA-SSE-010** | P0 | Events | Server-Sent Events endpoint (/events?session_id=...) | Session ID | Event stream | 254 | SSE, Axum |
| **AIA-POLL-010** | P0 | Events | Polling fallback (/import/status/{session_id}) | Session ID | Status JSON | 305 | HTTP, JSON |
| **AIA-INT-010** | P0 | Integration | SPEC008 Library Management workflows | Directory paths | File discovery | 340 | SPEC008 |
| **AIA-INT-020** | P0 | Integration | IMPL005 Segmentation workflow (Steps 1-5) | Audio file | Passage boundaries | 354 | IMPL005 |
| **AIA-INT-030** | P0 | Integration | Tick-based timing (28,224,000 ticks/sec) | Time in seconds | Tick values | 365 | wkmp-common |
| **AIA-ERR-010** | P0 | Error | Error severity levels (CRITICAL, WARNING, INFO) | Errors | Categorized errors | 376 | Error types |
| **AIA-ERR-020** | P0 | Error | Error reporting (SSE + polling + logs) | Error events | User notification | 386 | SSE, polling, logging |
| **AIA-PERF-010** | P1 | Performance | Performance targets (100 files in 2-5 min) | File count | Processing time | 403 | Benchmarking |
| **AIA-PERF-020** | P1 | Performance | Optimizations (caching, rate limits, batch inserts) | API calls | Cached responses | 418 | Cache tables |
| **AIA-SEC-010** | P0 | Security | Input validation (path traversal, symlinks, parameters) | User inputs | Validated data | 431 | Security validation |
| **AIA-SEC-020** | P1 | Security | Secure API key storage (credentials table) | API keys | Encrypted storage | 438 | credentials table |
| **AIA-TEST-010** | P0 | Testing | Unit tests for 8 categories | Components | Test coverage | 449 | Test framework |
| **AIA-TEST-020** | P0 | Testing | Integration tests (HTTP, SSE, database) | Microservices | Integration verification | 457 | Test environment |
| **AIA-TEST-030** | P1 | Testing | E2E tests (full import workflow) | Audio files | Complete passages | 464 | E2E framework |
| **AIA-FUTURE-010** | P3 | Future | Future enhancements (ML, collaborative filtering, etc.) | - | Out of scope | 474 | Future work |

---

## Requirements by Component

### HTTP Server & Routing
- AIA-OV-010: Module identity (port 5723, Axum server)
- AIA-UI-010: Web UI routes (/, /import-progress, /segment-editor, /api/*)
- AIA-UI-020: Health endpoint for wkmp-ui integration
- AIA-SSE-010: SSE endpoint for real-time events
- AIA-POLL-010: Polling endpoint for status

### Workflow State Machine
- AIA-WF-010: 7-state workflow (SCANNING→EXTRACTING→FINGERPRINTING→SEGMENTING→ANALYZING→FLAVORING→COMPLETED)
- AIA-WF-020: In-memory session persistence
- AIA-ERR-010: Error state transitions (→FAILED, →CANCELLED)

### Async Processing
- AIA-ASYNC-010: Tokio background tasks
- AIA-ASYNC-020: Parallel file processing (4 workers)

### Components (from AIA-COMP-010)
- file_scanner: Directory traversal
- metadata_extractor: Tag parsing (lofty)
- fingerprinter: Chromaprint integration
- musicbrainz_client: MusicBrainz API (rate limited 1 req/s)
- acousticbrainz_client: AcousticBrainz API
- amplitude_analyzer: RMS-based lead-in/lead-out detection
- silence_detector: Silence-based passage boundaries
- essentia_runner: Essentia subprocess (fallback)
- parameter_manager: Parameter storage/retrieval

### Database Integration
- AIA-DB-010: Write 9 tables (files, passages, songs, artists, works, albums, passage_songs, passage_albums, caches)
- AIA-INT-030: Tick conversion (28,224,000 ticks/second)

### External Integration
- AIA-INT-010: SPEC008 Library Management workflows
- AIA-INT-020: IMPL005 Segmentation workflow
- AIA-MS-010: wkmp-ui microservice communication

### Security & Validation
- AIA-SEC-010: Path traversal validation, symlink checking, parameter bounds
- AIA-SEC-020: API key storage in credentials table

### Error Handling & Reporting
- AIA-ERR-010: Severity categorization (CRITICAL, WARNING, INFO)
- AIA-ERR-020: Multi-channel reporting (SSE, polling, logs)

### Performance & Optimization
- AIA-PERF-010: Performance targets (100 files in 2-5 min Pi Zero2W, 30-60 sec desktop)
- AIA-PERF-020: Caching (AcoustID, MusicBrainz, AcousticBrainz), rate limiting, batch DB inserts

### Testing Requirements
- AIA-TEST-010: Unit tests (8 categories)
- AIA-TEST-020: Integration tests (HTTP, SSE, DB, external APIs)
- AIA-TEST-030: E2E tests (complete import scenarios)

### UI & User Experience
- AIA-UI-010: Dedicated web UI (HTML/CSS/JS)
- AIA-UI-020: wkmp-ui integration (health check, launch button)
- AIA-UI-030: Import completion and return navigation

---

## Missing or Implied Requirements

Based on review of SPEC024, the following areas are **fully specified**:
- ✅ HTTP server and routing (AIA-UI-010, AIA-SSE-010, AIA-POLL-010)
- ✅ Workflow state machine (AIA-WF-010, AIA-WF-020)
- ✅ Component architecture (AIA-COMP-010 with detailed table)
- ✅ Database integration (AIA-DB-010)
- ✅ Error handling (AIA-ERR-010, AIA-ERR-020)
- ✅ Testing requirements (AIA-TEST-010, AIA-TEST-020, AIA-TEST-030)

**Potentially Underspecified Areas** (to verify in Phase 2):
- ⚠️ Web UI implementation details (HTML/CSS/JS frameworks, specific UI components)
- ⚠️ Cancellation mechanism (referenced in workflow diagram but not detailed)
- ⚠️ Session cleanup (how long sessions persist in memory)
- ⚠️ Retry logic for external API failures
- ⚠️ Concurrency limits for API calls (rate limiting details)

---

## Traceability to External Documents

| External Doc | Referenced Requirements | Purpose |
|--------------|-------------------------|---------|
| **SPEC008** | AIA-INT-010 | Library management workflows (file discovery, deduplication) |
| **IMPL005** | AIA-INT-020 | Audio file segmentation (Steps 1-5) |
| **SPEC025** | Implied by AIA-COMP-010 | Amplitude analysis specification |
| **IMPL008** | Implied by AIA-UI-010 | API endpoint specifications |
| **IMPL009** | Implied by AIA-COMP-010 | Amplitude analyzer implementation |
| **IMPL010** | Implied by AIA-COMP-010 | Parameter management implementation |
| **IMPL011** | Implied by AIA-COMP-010 | MusicBrainz client implementation |
| **IMPL012** | Implied by AIA-COMP-010 | AcoustID client implementation |
| **IMPL013** | Implied by AIA-COMP-010 | File scanner implementation |
| **IMPL014** | Implied by AIA-COMP-010 | Database queries implementation |
| **IMPL001** | AIA-DB-010 | Database schema (tables, migrations) |
| **GOV002** | All AIA-* | Requirement numbering scheme |

---

## Next Steps

**Phase 2: Specification Completeness Verification**
- Verify each requirement has sufficient detail for implementation
- Identify ambiguities and missing information
- Check for conflicts and inconsistencies
- Assess testability of requirements

**Phase 3: Acceptance Test Definition**
- Define unit tests for each component (AIA-TEST-010)
- Define integration tests for microservice communication (AIA-TEST-020)
- Define E2E tests for complete workflows (AIA-TEST-030)
- Create traceability matrix (requirements ↔ tests)

---

**End of Requirements Index**
