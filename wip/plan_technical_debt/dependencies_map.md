# Dependencies Map: Technical Debt Reduction

**Project:** WKMP Technical Debt Reduction
**Generated:** 2025-11-10

---

## 1. Internal Code Dependencies

### 1.1 Module Dependency Graph (wkmp-ai)

```
wkmp-ai
├── types.rs (core type definitions)
│   ├── Used by: extractors, fusers, validators, workflow
│   └── Exports: ConfidenceValue, MetadataExtraction, FusedMetadata, etc.
│
├── extractors/ (Tier 1: Data extraction)
│   ├── Depends on: types.rs, wkmp-common
│   ├── id3_extractor.rs
│   ├── chromaprint_analyzer.rs
│   ├── acoustid_client.rs
│   ├── musicbrainz_client.rs
│   ├── essentia_analyzer.rs
│   ├── audio_derived_extractor.rs
│   └── id3_genre_mapper.rs
│
├── fusion/ (Tier 2: Data fusion)
│   ├── Depends on: types.rs, extractors/
│   ├── identity_resolver.rs
│   ├── metadata_fuser.rs
│   └── flavor_synthesizer.rs
│
├── validators/ (Tier 3: Quality validation)
│   ├── Depends on: types.rs, fusion/
│   ├── consistency_validator.rs
│   ├── completeness_scorer.rs
│   └── quality_scorer.rs
│
├── workflow/ (Pipeline orchestration)
│   ├── Depends on: types.rs, extractors/, fusion/, validators/
│   ├── pipeline.rs (PLAN024 two-pass extraction)
│   ├── workflow_orchestrator.rs (2,253 lines - TARGET FOR REFACTORING)
│   ├── storage.rs (database persistence)
│   └── song_processor.rs (DEAD CODE - DELETE IN PHASE 1)
│
├── services/ (External API clients)
│   ├── Depends on: types.rs, wkmp-common
│   ├── acoustid_client.rs (rate-limited API client)
│   ├── musicbrainz_client.rs (rate-limited API client)
│   ├── acousticbrainz_client.rs (rate-limited API client)
│   ├── file_scanner.rs (filesystem traversal)
│   └── file_tracker.rs (import tracking)
│
├── api/ (HTTP API and UI)
│   ├── Depends on: services/, workflow/, wkmp-common
│   ├── ui.rs (1,308 lines - TARGET FOR REFACTORING)
│   └── routes.rs (Axum route registration)
│
├── db/ (Database layer)
│   ├── Depends on: wkmp-common
│   ├── mod.rs (connection pooling)
│   └── schema.rs (migrations, queries)
│
└── lib.rs (crate root)
    └── Module exports and initialization
```

### 1.2 Module Dependency Graph (wkmp-common)

```
wkmp-common
├── models/ (Database models)
│   ├── passage.rs (Passage entity)
│   ├── song.rs (Song entity)
│   ├── artist.rs (Artist entity)
│   ├── album.rs (Album entity)
│   ├── musical_flavor.rs (Flavor vector)
│   └── import_session.rs (Import tracking)
│
├── events.rs (1,711 lines - TARGET FOR REFACTORING)
│   ├── Used by: All microservices
│   ├── ImportEvents (workflow progress)
│   ├── PlaybackEvents (audio player)
│   ├── SystemEvents (configuration)
│   └── SSE serialization
│
├── params.rs (1,450 lines - TARGET FOR REFACTORING)
│   ├── Used by: wkmp-ai, wkmp-pd
│   ├── CrossfadeParams
│   ├── SelectorParams
│   ├── TimingParams
│   ├── FlavorParams
│   └── SystemParams
│
├── config/ (Configuration utilities)
│   ├── root_folder.rs (4-tier resolution)
│   └── initializer.rs (directory creation)
│
├── time.rs (time utilities)
│   └── **CRITICAL:** Contains blocking sleep in async context (line 37)
│
└── lib.rs (crate root)
    └── Shared exports
```

### 1.3 Cross-Crate Dependencies

```
wkmp-ai
  ↓ depends on
wkmp-common (models, events, params, config, utilities)
```

**Implications for Refactoring:**
- Changes to wkmp-common types affect wkmp-ai
- Breaking changes to wkmp-common require wkmp-ai updates
- Test both crates after wkmp-common changes

---

## 2. External Dependencies

### 2.1 Core Dependencies (Required)

| Crate | Version | Purpose | Used In |
|-------|---------|---------|---------|
| **tokio** | 1.35+ | Async runtime, timers, channels | All modules |
| **sqlx** | 0.7+ | Database (SQLite), migrations | db/, models/ |
| **axum** | 0.7+ | HTTP server, SSE support | api/ |
| **reqwest** | 0.11+ | HTTP client (MusicBrainz, AcoustID, etc.) | services/ |
| **serde** | 1.0+ | Serialization framework | All modules |
| **serde_json** | 1.0+ | JSON serialization | events.rs, params.rs |
| **uuid** | 1.6+ | UUID generation (passage IDs, etc.) | models/, types.rs |
| **anyhow** | 1.0+ | Error handling with context | All modules |
| **thiserror** | 1.0+ | Custom error types | types.rs |
| **tracing** | 0.1+ | Structured logging | All modules |

### 2.2 Audio Processing Dependencies

| Crate | Version | Purpose | Used In |
|-------|---------|---------|---------|
| **chromaprint-sys** | 0.2+ | Audio fingerprinting (FFI) | chromaprint_analyzer.rs |
| **symphonia** | 0.5+ | Audio decoding (MP3, FLAC, etc.) | extractors/ |

### 2.3 Development Dependencies (Tests)

| Crate | Version | Purpose | Used In |
|-------|---------|---------|---------|
| **tokio-test** | 0.4+ | Async test utilities | Test modules |
| **tempfile** | 3.8+ | Temporary directories for tests | file_scanner tests |

---

## 3. Dependency Constraints

### 3.1 Rate Limiter Duplication (PHASE 5 TARGET)

**Current State:** 4 duplicate rate limiter implementations

| Location | Lines | Rate Limit | Target |
|----------|-------|------------|--------|
| wkmp-ai/src/services/acoustid_client.rs | 90 | 3 req/sec | 🔄 Extract |
| wkmp-ai/src/services/musicbrainz_client.rs | 107 | 1 req/sec | 🔄 Extract |
| wkmp-ai/src/extractors/musicbrainz_client.rs | 114 | 1 req/sec | 🔄 Extract |
| wkmp-ai/src/services/acousticbrainz_client.rs | 192 | 1 req/sec | 🔄 Extract |

**Refactoring Plan (Phase 5):**
- Create `wkmp-common/src/rate_limiter.rs`
- Extract shared `RateLimiter` utility
- Update all 4 clients to use shared utility
- Eliminate 4 duplicates → 1 implementation

**Benefit:** DRY principle, fix bugs once

---

## 4. File Size Dependencies (Refactoring Targets)

### 4.1 Phase 2 Targets (File Organization)

| File | Lines | Target Modules | Priority |
|------|-------|----------------|----------|
| workflow_orchestrator.rs | 2,253 | 7-8 modules (phase_*.rs) | CRITICAL |
| events.rs | 1,711 | 3-4 modules (by category) | HIGH |
| params.rs | 1,450 | 4-5 modules (by param group) | HIGH |
| api/ui.rs | 1,308 | 5-6 modules (by page) | MEDIUM |

**Refactoring Dependencies:**
- workflow_orchestrator.rs depends on: extractors/, fusion/, validators/, storage.rs
- events.rs: Independent (pure data types)
- params.rs: Independent (pure data types)
- api/ui.rs depends on: services/, workflow/

**Impact:** No circular dependencies introduced

---

## 5. Test Dependencies

### 5.1 Test Structure

```
wkmp-ai/src/**/tests
├── Unit tests (inline #[cfg(test)] modules)
├── Integration tests (tests/ directory)
└── Test helpers (test utilities)

wkmp-common/src/**/tests
├── Unit tests (inline #[cfg(test)] modules)
└── Test utilities
```

**Test Count:** 216 tests total
- wkmp-ai: ~180 tests
- wkmp-common: ~36 tests

**Test Dependencies:**
- Tests depend on production code
- Integration tests depend on database (SQLite in-memory)
- Tests use tempfile for filesystem operations
- Tests use tokio-test for async utilities

**Critical Constraint:** All 216 tests MUST pass after each increment

---

## 6. Build Dependencies

### 6.1 Build Process

```bash
# Build all modules
cargo build -p wkmp-ai -p wkmp-common

# Run all tests
cargo test -p wkmp-ai -p wkmp-common --all-features

# Lint checks
cargo clippy -p wkmp-ai -p wkmp-common
cargo fmt --check
```

### 6.2 Build Constraints

- **Rust Stable:** No nightly features
- **Edition 2021:** Modern Rust idioms
- **Incremental Compilation:** Enabled for development
- **Link-Time Optimization:** Disabled for development (faster builds)

---

## 7. API Contract Dependencies

### 7.1 Public API (wkmp-common)

**CRITICAL: Backward compatibility required**

| Module | Public Types | Consumers |
|--------|--------------|-----------|
| models/ | Passage, Song, Artist, Album, etc. | wkmp-ai, wkmp-pd, wkmp-ui |
| events.rs | All event types | All microservices (SSE) |
| params.rs | All parameter types | wkmp-ai, wkmp-pd |
| config/ | RootFolderResolver, RootFolderInitializer | All microservices |

**Constraint:** NO breaking changes allowed

### 7.2 Public API (wkmp-ai)

| Module | Public Types | Consumers |
|--------|--------------|-----------|
| types.rs | ConfidenceValue, MetadataExtraction, etc. | Internal only (not exported from lib.rs) |
| api/ | HTTP endpoints | wkmp-ui (HTTP client) |

**Constraint:** HTTP API contracts preserved

---

## 8. Refactoring Dependencies

### 8.1 Phase Order Dependencies

```
Phase 1 (Quick Wins)
  ↓ Reduces noise (warnings, dead code)
Phase 2 (File Organization)
  ↓ Improves navigability for Phase 3
Phase 3 (Error Handling)
  ↓ Cleaner error paths for Phase 4 docs
Phase 4 (Documentation)
  ↓ Complete docs before Phase 5 refactoring
Phase 5 (Code Quality)
```

**Recommendation:** Complete phases in order

### 8.2 Increment Dependencies

**Within Each Phase:**
1. Make change (edit files)
2. Run tests (`cargo test`)
3. Fix failures
4. Commit (tests passing)
5. Repeat

**No Blocking Dependencies:** Each increment is independent

---

## 9. Dependency Risks

### Risk 1: Circular Dependencies (Phase 2)
- **Probability:** Low
- **Impact:** High (prevents modularization)
- **Mitigation:** Careful module design, avoid cross-module dependencies

### Risk 2: Breaking wkmp-common API (All Phases)
- **Probability:** Medium
- **Impact:** Critical (breaks all microservices)
- **Mitigation:** Review all public API changes, test downstream consumers

### Risk 3: Test Dependencies on Internal Structure
- **Probability:** Medium
- **Impact:** Medium (tests fail after refactoring)
- **Mitigation:** Update test imports, verify behavior unchanged

### Risk 4: External Dependency Version Conflicts
- **Probability:** Low
- **Impact:** Medium (build failures)
- **Mitigation:** Use existing dependency versions, no upgrades

---

## 10. Dependency Summary

**Key Takeaways:**

1. **Internal Dependencies:**
   - wkmp-ai depends on wkmp-common (backward compatibility critical)
   - workflow_orchestrator.rs depends on all other modules (refactor last in Phase 2)
   - No circular dependencies currently

2. **External Dependencies:**
   - No new dependencies required
   - Existing dependencies support refactoring (anyhow for error context)

3. **Test Dependencies:**
   - 216 tests must pass after each increment
   - Tests provide regression detection

4. **Refactoring Order:**
   - Complete phases in order (1 → 5)
   - Within Phase 2: events.rs and params.rs first (independent), then orchestrator

5. **Backward Compatibility:**
   - All public APIs in wkmp-common preserved
   - HTTP APIs in wkmp-ai preserved
   - Semantic versioning: patch bumps only
