# Dependencies Map: WKMP-AI Audio Import System Recode

**Plan:** PLAN024
**Created:** 2025-11-09

---

## Existing WKMP Components (Read-Only Dependencies)

### wkmp-common (Shared Library)

**Database Utilities:**
- ✅ `database::Pool` - SQLite connection pool
- ✅ `database::get_pool()` - Database initialization
- ⚠️ **VERIFY:** SPEC031 SchemaSync trait and implementation
- ⚠️ **VERIFY:** SPEC017 tick conversion utilities (`ticks_to_seconds()`, `seconds_to_ticks()`)

**Event Bus:**
- ✅ `events::EventBus` - tokio::broadcast-based event distribution
- ✅ `events::Event` enum - Shared event types

**Models:**
- ✅ `models::Passage` - Database passage entity
- ⚠️ **NEEDS EXTENSION:** Add 17 new fields per REQ-AI-080

**Configuration:**
- ✅ `config::RootFolderResolver` - 4-tier root folder resolution
- ✅ `config::RootFolderInitializer` - Directory creation

**Status:** EXISTING - Read-only dependency, may need extensions

---

## External Libraries (Cargo Dependencies)

### Audio Processing

**symphonia** (Audio Decoding)
- **Purpose:** Decode audio files to PCM
- **Usage:** Extract audio segments for passage processing
- **Version:** Latest stable
- **Status:** REQUIRED

**rubato** (Resampling - if needed)
- **Purpose:** Resample audio to target sample rate
- **Usage:** Normalize sample rates before analysis
- **Version:** Latest stable
- **Status:** CONDITIONAL (only if Essentia requires specific sample rate)

### Fingerprinting & Identification

**chromaprint** (Rust binding or FFI)
- **Purpose:** Generate audio fingerprints
- **Usage:** ChromaprintAnalyzer (Tier 1)
- **Crate:** Search for `chromaprint-rust` or use FFI to C library
- **Status:** REQUIRED

### HTTP & Async

**tokio** (Async Runtime)
- **Purpose:** Async/await runtime, parallel execution
- **Features:** `["full"]` for all features
- **Usage:** Per-song parallel extraction (`tokio::join!`), async HTTP
- **Status:** REQUIRED (existing)

**axum** (HTTP Framework)
- **Purpose:** HTTP server, SSE endpoint
- **Usage:** POST /import/start, GET /import/events (SSE)
- **Status:** REQUIRED (existing)

**reqwest** (HTTP Client)
- **Purpose:** API calls to AcoustID, MusicBrainz
- **Features:** `["json"]` for JSON parsing
- **Usage:** AcoustIDClient, MusicBrainzClient (Tier 1)
- **Status:** REQUIRED

**tower** (Middleware)
- **Purpose:** Rate limiting middleware
- **Usage:** Limit AcoustID (3 req/s), MusicBrainz (1 req/s)
- **Crate:** `tower::limit::RateLimit`
- **Status:** REQUIRED

### Database

**rusqlite** (SQLite Driver)
- **Purpose:** SQLite database access
- **Features:** `["bundled", "json"]` for JSON1 extension
- **Usage:** Database operations, JSON storage
- **Status:** REQUIRED (existing via wkmp-common)

### Musical Analysis

**essentia** (Optional - External Library)
- **Purpose:** Musical flavor characteristic computation
- **Usage:** EssentiaAnalyzer (Tier 1)
- **Installation:** System package (`apt-get install essentia-extractor` or `brew install essentia`)
- **Rust Binding:** May require FFI or command-line wrapper
- **Status:** OPTIONAL (graceful degradation to AudioDerived if unavailable)
- **Confidence:** 0.9 (high quality) if available, 0.6 (AudioDerived) if not

### String Utilities

**strsim** (String Similarity)
- **Purpose:** Levenshtein distance for title consistency check
- **Usage:** ConsistencyValidator (Tier 3)
- **Function:** `strsim::normalized_levenshtein()`
- **Status:** REQUIRED

**uuid** (UUID Generation)
- **Purpose:** Generate UUIDs for passage IDs, provenance log IDs
- **Features:** `["v4", "serde"]`
- **Status:** REQUIRED (existing)

### Serialization

**serde** (Serialization Framework)
- **Purpose:** JSON serialization for database storage
- **Features:** `["derive"]`
- **Usage:** Serialize flavor vectors, metadata, validation reports
- **Status:** REQUIRED (existing)

**serde_json** (JSON Support)
- **Purpose:** JSON encoding/decoding
- **Usage:** Store complex data in SQLite TEXT columns
- **Status:** REQUIRED (existing)

---

## External Services (Runtime Dependencies)

### AcoustID API

**Endpoint:** `https://api.acoustid.org/v2/lookup`
**Authentication:** API key required (free tier available)
**Rate Limit:** 3 requests/second
**Usage:** AcoustIDClient (Tier 1)
**Failure Mode:** Graceful degradation - proceed without AcoustID MBID
**Status:** REQUIRED (but optional at runtime)

### MusicBrainz API

**Endpoint:** `https://musicbrainz.org/ws/2/`
**Authentication:** None (public API)
**Rate Limit:** 1 request/second (50 req/s if private server)
**Usage:** MusicBrainzClient (Tier 1)
**Failure Mode:** Graceful degradation - use ID3 metadata only
**Status:** REQUIRED (but optional at runtime)

### AcousticBrainz API (Historical Data Only)

**Status:** SERVICE ENDED 2022
**Usage:** Historical data may be available via cached lookups
**Impact:** Specification assumes AcousticBrainz is unavailable
**Mitigation:** Essentia + AudioDerived provide musical flavor

---

## WKMP Specifications (Reference Documents)

### GOV001 - Document Hierarchy
- **Purpose:** Understand documentation framework
- **Usage:** Follow 5-tier documentation structure
- **Status:** READ-ONLY

### GOV002 - Requirements Enumeration
- **Purpose:** Requirement ID numbering scheme
- **Usage:** Verify REQ-AI-XXX format compliance
- **Status:** READ-ONLY

### SPEC001 - WKMP Architecture
- **Purpose:** Microservices architecture overview
- **Usage:** Understand wkmp-ai's role in system
- **Status:** READ-ONLY

### SPEC003 - Musical Flavor
- **Purpose:** Musical flavor characteristics taxonomy
- **Usage:** Reference for expected characteristics count (completeness scoring)
- **Open Question:** What is total count? (Estimate: 50-100 characteristics)
- **Status:** READ-ONLY

### SPEC017 - Tick-Based Timing
- **Purpose:** Sample-accurate timing representation
- **Usage:** Convert passage boundaries to ticks (28,224,000 Hz)
- **Dependencies:** Tick conversion utilities in wkmp-common
- **Status:** READ-ONLY + wkmp-common utilities

### SPEC030 - Software Legibility Patterns
- **Purpose:** Concept-based architecture guidance (HOW to design)
- **Usage:** Apply patterns during implementation (not prescriptive)
- **Impact:** Ground-up recode follows SPEC030 patterns
- **Status:** GUIDANCE (patterns, not requirements)

### SPEC031 - Data-Driven Schema Maintenance
- **Purpose:** Zero-configuration database schema evolution
- **Usage:** Automatic column addition for 17 new passages table fields
- **Dependencies:** SchemaSync trait in wkmp-common
- **Critical:** MUST verify SPEC031 is implemented before beginning
- **Status:** DEPENDENCY (must exist in wkmp-common)

### REQ001 - WKMP Requirements
- **Purpose:** System-wide requirements
- **Usage:** Understand passage entity definition
- **Status:** READ-ONLY

### REQ002 - Entity Definitions
- **Purpose:** Canonical entity definitions
- **Usage:** Passage, Song, Musical Flavor entities
- **Status:** READ-ONLY

---

## Integration Points

### wkmp-ui (User Interface Microservice)

**HTTP API (wkmp-ai provides):**
- `POST /import/start` - Initiate import from wkmp-ui
- `GET /import/status/{session_id}` - Query import status
- `GET /import/events` - SSE endpoint for real-time progress

**Impact on wkmp-ui:**
- wkmp-ui must consume 10 new SSE event types
- wkmp-ui must display per-song progress (not just file-level)
- wkmp-ui may need UI updates to show validation warnings

**Status:** INTEGRATION REQUIRED (wkmp-ui changes may be needed)

### wkmp-ap (Audio Player Microservice)

**Database Dependency:**
- wkmp-ap reads passages table
- 17 new columns must not break wkmp-ap queries
- **Mitigation:** SPEC031 guarantees backward compatibility (NULL columns ignored)

**Impact on wkmp-ap:**
- None expected (wkmp-ap reads existing columns only)
- New columns (quality scores, provenance) are additional metadata

**Status:** NO CHANGES NEEDED

### wkmp-pd (Program Director Microservice)

**Database Dependency:**
- wkmp-pd reads passages table for selection
- wkmp-pd may benefit from quality scores for selection weighting

**Impact on wkmp-pd:**
- None required (wkmp-pd can ignore new columns)
- Future enhancement: Use `overall_quality_score` for selection bias

**Status:** NO CHANGES NEEDED

---

## Testing Dependencies

### Test Fixtures

**Audio Files:**
- Multi-song test file (e.g., classical suite, DJ mix)
- Single-song test file (standard 3-minute track)
- Corrupted audio file (test error handling)
- Empty audio file (test edge case)

**Database:**
- Test database with existing passages
- Clean database (test zero-config startup)
- Database with missing columns (test SPEC031 schema sync)

**API Mocks:**
- Mock AcoustID API responses (test rate limiting)
- Mock MusicBrainz API responses (test metadata fusion)
- Simulated API failures (test graceful degradation)

**Test Data:**
- Known MBID values for test tracks
- Known musical flavor characteristics for test tracks
- Expected validation results for test tracks

**Status:** TO BE CREATED during Phase 3

---

## Dependency Resolution Strategy

### Phase 1 (Verification - Before Implementation)

1. ✅ Verify wkmp-common has SPEC031 SchemaSync
2. ✅ Verify wkmp-common has SPEC017 tick conversion utilities
3. ✅ Verify wkmp-common database pool and event bus
4. ⚠️ Identify chromaprint Rust binding availability
5. ⚠️ Identify Essentia installation method

### Phase 2 (Cargo Dependencies)

1. Add external crates to wkmp-ai/Cargo.toml
2. Verify version compatibility
3. Test compilation

### Phase 3 (External Services)

1. Obtain AcoustID API key
2. Test AcoustID API connectivity
3. Test MusicBrainz API connectivity
4. Verify rate limiting behavior

### Phase 4 (Integration Testing)

1. Test wkmp-ui SSE event consumption
2. Verify wkmp-ap/wkmp-pd database compatibility
3. Test zero-config startup with SPEC031

---

## Open Questions (Dependency-Related)

1. **Chromaprint Rust Binding:** Which crate to use? (FFI vs. pure Rust)
2. **Essentia Integration:** Command-line wrapper or FFI binding?
3. **SPEC031 Status:** Is SchemaSync already implemented in wkmp-common?
4. **SPEC017 Utilities:** Are tick conversion functions available?
5. **AcoustID API Key:** Who provides API key for deployment?

**Resolution:** Phase 2 specification verification will address these questions

---

## Dependency Graph Summary

```
WKMP-AI Recode
├── wkmp-common (EXISTING)
│   ├── Database Pool ✅
│   ├── Event Bus ✅
│   ├── SPEC031 SchemaSync ⚠️ VERIFY
│   └── SPEC017 Tick Utils ⚠️ VERIFY
├── External Crates
│   ├── symphonia (audio decode) ✅
│   ├── chromaprint (fingerprint) ⚠️ IDENTIFY
│   ├── tokio (async) ✅
│   ├── axum (HTTP) ✅
│   ├── reqwest (HTTP client) ✅
│   ├── tower (rate limiting) ⚠️ NEW
│   ├── rusqlite (SQLite) ✅
│   ├── strsim (string similarity) ⚠️ NEW
│   ├── uuid ✅
│   ├── serde ✅
│   └── serde_json ✅
├── External Services
│   ├── AcoustID API ⚠️ RUNTIME
│   ├── MusicBrainz API ⚠️ RUNTIME
│   └── Essentia (optional) ⚠️ OPTIONAL
├── WKMP Specifications (Reference)
│   ├── GOV001, GOV002 ✅
│   ├── SPEC001, SPEC003 ✅
│   ├── SPEC017 ✅
│   ├── SPEC030 ✅
│   ├── SPEC031 ⚠️ CRITICAL
│   └── REQ001, REQ002 ✅
└── Integration Points
    ├── wkmp-ui (SSE events) ⚠️ MAY NEED UPDATES
    ├── wkmp-ap (database) ✅ NO CHANGES
    └── wkmp-pd (database) ✅ NO CHANGES

Legend:
✅ Exists, no issues
⚠️ Needs verification or action
❌ Missing, must create
```

---

## Risk Assessment (Dependency-Related)

**HIGH RISK:**
- SPEC031 SchemaSync not implemented → Blocks zero-config startup
- SPEC017 tick utilities missing → Blocks timing compliance
- Chromaprint binding unavailable → Blocks fingerprint generation

**MEDIUM RISK:**
- AcoustID/MusicBrainz API rate limits → Slows import processing
- Essentia unavailable → Reduces musical flavor quality (fallback to AudioDerived)

**LOW RISK:**
- wkmp-ui SSE updates needed → Can be implemented in parallel
- Test fixture creation → Standard test data generation

**Mitigation:** Phase 2 verification will identify and resolve CRITICAL dependencies before proceeding to implementation
