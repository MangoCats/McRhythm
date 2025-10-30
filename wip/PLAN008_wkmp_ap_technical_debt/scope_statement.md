# Scope Statement - wkmp-ap Technical Debt Remediation

**Plan:** PLAN008
**Source:** wip/SPEC024-wkmp_ap_technical_debt_remediation.md
**Created:** 2025-10-29

---

## In Scope

**WILL be implemented in this plan:**

1. **Security Remediation**
   - POST/PUT authentication enforcement using shared_secret
   - Request body JSON field extraction for credentials
   - HTTP 401 error responses for failed authentication

2. **Decoder Error Reporting**
   - File path storage in ChunkedDecoder struct
   - File path inclusion in all decoder error messages
   - Error message improvements for debugging

3. **Buffer Configuration**
   - Database query for buffer_capacity_samples setting
   - Database query for buffer_headroom_samples setting
   - Graceful fallback to compiled defaults when NULL

4. **Developer UI Telemetry**
   - Decoder state reporting (Decoding/Paused/Complete)
   - Source sample rate reporting
   - Fade stage reporting (FadeIn/Playing/FadeOut)
   - Start timestamp tracking

5. **Passage Metadata**
   - Album UUID queries via database joins
   - song_albums field population in PassageStarted events
   - song_albums field population in PassageComplete events

6. **Duration Tracking**
   - Playback start time tracking in mixer
   - Elapsed time calculation for PassageComplete
   - Millisecond-precision duration reporting

7. **Code Quality**
   - Critical .unwrap() replacement in audio hot paths
   - engine.rs refactoring (split into 3 modules)
   - Compiler warning elimination (21 warnings)
   - Config file deduplication (config.rs vs config_new.rs)
   - Backup file removal (.backup, .backup2)

8. **Minor Enhancements**
   - Audio clipping warning logs
   - Outdated TODO comment removal

**Total:** 37 requirements across 13 debt items

---

## Out of Scope

**Will NOT be implemented in this plan:**

1. **Phase 5 Future Features**
   - Seeking support (set_position implementation)
   - Drain-based buffer management rework
   - Ring buffer underrun detection improvements
   - **Rationale:** Deferred to future roadmap per project plan

2. **Comprehensive .unwrap() Elimination**
   - Test code .unwrap() calls (acceptable in tests)
   - Non-critical paths (low-priority refactor)
   - **Rationale:** Focus on hot paths only (audio thread, decoders)

3. **Complete File Refactoring**
   - mixer.rs split (1,933 lines → remains as-is)
   - handlers.rs split (1,305 lines → remains as-is)
   - **Rationale:** engine.rs is highest priority; others deferred

4. **Performance Optimization**
   - Buffer tuning algorithm improvements
   - Database query optimization
   - **Rationale:** Not technical debt, separate optimization effort

5. **New Feature Development**
   - Additional diagnostics beyond specified telemetry
   - New API endpoints
   - **Rationale:** This is remediation only, not feature development

---

## Assumptions

**Explicit statements assumed true:**

1. **Existing Authentication Infrastructure**
   - `shared_secret` mechanism already implemented for GET requests
   - Secret validation logic exists and is tested
   - Can be reused for POST/PUT with minimal changes

2. **Database Schema**
   - `settings` table contains buffer configuration keys
   - `passage_albums` linking table exists
   - `albums` table has `guid` column for UUID queries

3. **Decoder Worker Architecture**
   - Decoder workers can be queried for telemetry
   - Telemetry queries are non-blocking
   - Minimal performance impact from telemetry tracking

4. **Test Infrastructure**
   - Existing test harness supports new security tests
   - Integration tests can be added without framework changes
   - Test utilities exist for passage/album creation

5. **Development Environment**
   - Rust stable toolchain available
   - cargo fix works for automatic warning fixes
   - No external dependencies required (all fixes use existing crates)

6. **Deployment**
   - Changes are backward-compatible (no API breaks)
   - Database migrations not required (schema already supports features)
   - Can deploy incrementally (sprint by sprint)

---

## Constraints

**Technical, schedule, and resource limitations:**

### Technical Constraints

1. **Backward Compatibility**
   - Public API must remain unchanged (internal refactoring only)
   - Existing clients must continue working without modifications
   - No breaking changes to event structures

2. **Performance Requirements**
   - Telemetry overhead must be <1% CPU impact
   - Database queries must not block audio thread
   - Mutex lock error handling must not degrade performance

3. **Architecture Constraints**
   - Must fit within existing microservices architecture
   - HTTP REST + SSE communication patterns unchanged
   - Single-stream audio pipeline unchanged

4. **Rust Language Constraints**
   - Must use safe Rust (no new unsafe blocks)
   - Async/await with tokio runtime
   - Error handling via Result types

### Schedule Constraints

1. **Three-Sprint Timeline**
   - Sprint 1: 1 week (security + critical)
   - Sprint 2: 1 week (functionality + diagnostics)
   - Sprint 3: 1 week (code health)
   - **Total:** 3 weeks

2. **Incremental Delivery**
   - Each sprint must deliver working functionality
   - Must pass all tests at end of each sprint
   - No "big bang" integration

### Resource Constraints

1. **Single Developer**
   - Plan assumes one developer full-time
   - Some tasks can be parallelized if multiple developers available

2. **No External Dependencies**
   - Cannot introduce new crate dependencies
   - Must use existing infrastructure

3. **Testing Environment**
   - Must test on existing hardware
   - No specialized audio equipment required

---

## Dependencies

### Existing Code Required

**Must exist and be functional:**

1. **Authentication System** (`wkmp-ap/src/api/auth_middleware.rs`)
   - shared_secret validation for GET requests
   - Error response generation
   - State management with Arc<RwLock<Config>>

2. **Database Layer** (`wkmp-ap/src/db/`)
   - settings.rs: get_setting_* functions
   - passages.rs: passage query functions
   - SQLite connection pool

3. **Audio Pipeline** (`wkmp-ap/src/audio/`, `wkmp-ap/src/playback/`)
   - ChunkedDecoder struct
   - PlayoutRingBuffer
   - CrossfadeMixer
   - DecoderWorker pool

4. **Event System** (`wkmp-ap/src/events.rs`)
   - PlaybackEvent enum
   - EventBus for publishing
   - PassageStarted/PassageComplete variants

### External Libraries Used

**Already in Cargo.toml:**

- `axum` - HTTP server framework
- `tokio` - Async runtime
- `sqlx` - Database queries
- `serde` / `serde_json` - JSON serialization
- `uuid` - UUID types
- `tracing` - Logging
- `symphonia` - Audio decoding

**No new dependencies required**

### Database Schema Required

**Must exist:**

- `settings` table with TEXT key-value pairs
- `passages` table with guid, file_id, timing fields
- `passage_albums` table (passage_id, album_id)
- `albums` table with guid column

**Status:** All exist per IMPL001-database_schema.md

### Hardware/Environment Requirements

**Development:**
- Linux/macOS/Windows (Rust cross-platform)
- Audio output device (for playback tests)
- SQLite (included with sqlx)

**Production:** (same as current wkmp-ap requirements)

---

## Success Criteria

**This plan is successful when:**

### Sprint 1 (Security & Critical)
- ✅ POST/PUT requests require authentication
- ✅ Invalid credentials return HTTP 401
- ✅ Valid credentials allow request through
- ✅ Decoder errors include file paths
- ✅ Buffer settings read from database

### Sprint 2 (Functionality & Diagnostics)
- ✅ Developer UI shows decoder state, sample rate, fade stage
- ✅ PassageStarted/Complete events include album UUIDs
- ✅ Duration played calculated accurately (±100ms)
- ✅ Single config file (duplication resolved)
- ✅ No backup files in repository

### Sprint 3 (Code Health)
- ✅ Zero compiler warnings (`cargo build -p wkmp-ap`)
- ✅ No .unwrap() in audio thread critical paths
- ✅ engine.rs split into 3 modules (<1500 lines each)
- ✅ Audio clipping warnings logged

### Overall Completion
- ✅ All 37 requirements met
- ✅ All acceptance tests pass
- ✅ Integration tests pass
- ✅ Performance within constraints (<1% overhead)
- ✅ Code review approved
- ✅ Documentation updated

---

## Risk Mitigation

**Key risks and mitigations:**

1. **Authentication breaks existing clients**
   - Mitigation: Comprehensive security test suite
   - Fallback: Can deploy with warning-only mode initially

2. **Refactoring introduces regressions**
   - Mitigation: Run full test suite before/after
   - Fallback: Incremental refactoring with frequent testing

3. **Database queries slow initialization**
   - Mitigation: Cache settings at startup
   - Monitoring: Add timing logs for settings queries

4. **Telemetry adds overhead**
   - Mitigation: Profile before/after changes
   - Constraint: Must be <1% CPU impact

---

## Approval

**This scope statement defines boundaries for PLAN008 implementation.**

- Scope clearly defined: IN/OUT explicit
- Assumptions documented and verified
- Constraints identified and acceptable
- Dependencies confirmed available
- Success criteria measurable

**Ready to proceed to Phase 2: Specification Completeness Verification**
