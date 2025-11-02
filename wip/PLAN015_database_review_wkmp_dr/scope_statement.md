# Scope Statement: Database Review Module (wkmp-dr)

**Plan:** PLAN015 - Database Review Module (wkmp-dr)
**Created:** 2025-11-01
**Approach:** Approach B from analysis (new independent microservice)
**Version:** Full version only

---

## Executive Summary

Create new independent microservice **wkmp-dr** (Database Review) providing read-only inspection of wkmp.db database contents with table browsing, predefined filtered views, custom searches, and user preference persistence.

**Key Characteristics:**
- Standalone HTTP server on port 5725
- Read-only database access (safety-critical)
- On-demand specialized tool (launched from wkmp-ui)
- Full version packaging only
- Extensible architecture for future filters/searches

---

## ✅ In Scope

### Core Functionality
- [x] **Table browsing:** View all rows/columns for any database table
- [x] **Pagination:** 100 rows per page with navigation controls
- [x] **Row counts:** Display total rows per table
- [x] **Column sorting:** Ascending/descending sort on any column
- [x] **Predefined filters:**
  - Passages lacking MusicBrainz Recording ID
  - Files without passages
- [x] **Custom searches:**
  - Find passages by MusicBrainz Work ID
  - Search files by path pattern
- [x] **Saved searches:** Save/load favorite search configurations
- [x] **User preferences:** Persist settings across restarts (localStorage)

### Architecture & Integration
- [x] **Zero-config startup:** 4-tier root folder resolution
- [x] **Read-only database:** SQLite `mode=ro` connection
- [x] **API authentication:** Timestamp + SHA-256 hash per WKMP standard
- [x] **Health endpoint:** GET /health operational monitoring
- [x] **Port 5725:** Dedicated microservice port
- [x] **On-demand pattern:** Launched from wkmp-ui, opens in new tab
- [x] **wkmp-ui integration:** Launch button in Tools menu
- [x] **Full version packaging:** Binary included in Full version only

### UI & User Experience
- [x] **Inline HTML:** Following wkmp-ai pattern
- [x] **Vanilla JavaScript:** No frameworks, zero npm dependencies
- [x] **CSS custom properties:** WKMP theme consistency
- [x] **Mobile-responsive:** Adaptive layout for small screens
- [x] **Table rendering:** Data grid with pagination controls

### Extensibility (Architecture)
- [x] **Plugin-like filter system:** Easy addition of new predefined views
- [x] **Documentation:** How to add custom filters/searches
- [x] **Query abstraction:** SQL queries separated from UI logic

---

## ❌ Out of Scope

### Write Operations (Explicitly Excluded)
- ❌ Database editing (INSERT, UPDATE, DELETE)
- ❌ Schema modifications (ALTER TABLE, CREATE TABLE)
- ❌ Settings changes (except user preferences in localStorage)
- **Rationale:** Read-only tool for safety; prevents accidental corruption

### Advanced Features (Deferred to Future)
- ❌ SQL query console (arbitrary SQL execution)
  - **Rationale:** Security risk, out of MVP scope
  - **Future:** Could add with query validation/sandboxing
- ❌ Data export (CSV, JSON download)
  - **Rationale:** Not in user requirements, defer if time-constrained
  - **Future:** High-value feature, implement in next release
- ❌ Schema visualization (ER diagrams)
  - **Rationale:** Complex UI, low ROI for troubleshooting use case
  - **Future:** Consider if user feedback requests
- ❌ Real-time updates (SSE for database changes)
  - **Rationale:** Read-only snapshot sufficient for debugging
  - **Future:** Useful for monitoring, defer to P2 priority
- ❌ Advanced filtering (regex, date ranges, multi-column)
  - **Rationale:** Start with simple filters, iterate based on usage
  - **Future:** Add as user requests specific filters

### Other Modules (Not This Plan)
- ❌ Modifications to wkmp-ui (except launch button - in scope)
- ❌ Modifications to wkmp-ap, wkmp-pd, wkmp-ai, wkmp-le
- ❌ Database schema changes
- ❌ wkmp-common library changes (uses existing utilities)

### Platform Support (Same as WKMP)
- ❌ Windows support (WKMP targets Linux/macOS)
- ❌ Embedded systems (Raspberry Pi support TBD)

---

## Assumptions

**Critical Assumptions (Must Hold for Plan Success):**

1. **Database Schema Stable:**
   - Assume wkmp.db schema matches wkmp-common/src/db/init.rs
   - If schema changes, queries may break
   - **Risk Mitigation:** Use wkmp-common database models where possible

2. **wkmp-common Utilities Available:**
   - RootFolderResolver, RootFolderInitializer exist and work correctly
   - Database initialization utilities functional
   - **Risk Mitigation:** Covered by integration tests

3. **Port 5725 Available:**
   - Assume no other software uses port 5725
   - **Risk Mitigation:** Log error if port unavailable, document conflict resolution

4. **Browser Compatibility:**
   - Assume modern browser (Chrome 90+, Firefox 88+, Safari 14+)
   - localStorage and ES6 JavaScript supported
   - **Risk Mitigation:** Document minimum browser requirements

5. **Read-Only Access Sufficient:**
   - Assume users only need to VIEW data, not modify
   - **Risk Mitigation:** User approved read-only scope

6. **Shared Secret Exists:**
   - Assume `settings` table contains `shared_secret` key
   - **Risk Mitigation:** Module initialization checks for secret, creates if missing

7. **wkmp-ui Modification Acceptable:**
   - Assume adding launch button to wkmp-ui is approved
   - **Risk Mitigation:** Minimal UI change, documented in plan

8. **Development Environment:**
   - Rust stable toolchain available
   - cargo build works without issues
   - sqlx CLI available for migrations (if needed)

---

## Constraints

### Technical Constraints

**MANDATORY (Must Comply):**
1. **Rust Language:** All code in Rust (stable channel)
2. **Async Runtime:** Tokio (matches WKMP stack)
3. **HTTP Framework:** Axum (matches wkmp-ai, wkmp-le)
4. **Database:** SQLite 3 via sqlx (async, compile-time verification)
5. **Read-Only:** Database connection MUST be read-only
6. **Zero External Frontend Dependencies:** No npm, no build step
7. **Port 5725:** Must use assigned port (non-negotiable)
8. **Authentication:** MUST implement WKMP auth protocol

**RECOMMENDED (Should Follow):**
1. **UI Pattern:** Inline HTML (consistency with wkmp-ai)
2. **CSS Approach:** Custom properties (WKMP theme)
3. **JavaScript:** Vanilla ES6+ (zero frameworks)
4. **Module Structure:** Mirror wkmp-ai organization (api/, db/, models/)

### Process Constraints

1. **Test Coverage:** Aim for 80%+ unit/integration test coverage
2. **Documentation:** All public APIs documented
3. **Code Review:** All code reviewed before merge
4. **Traceability:** All requirements traced to tests and implementation

### Resource Constraints

1. **Development Time:** Estimated 23-36 hours (per analysis)
2. **Developer Availability:** Assumes 2-3 hours/day development capacity
3. **Testing Time:** 4-6 hours allocated for comprehensive testing

### Architecture Constraints

1. **Microservices Pattern:** Must follow WKMP microservices architecture
2. **On-Demand Pattern:** Must follow ARCH-OD-010 exactly
3. **No Shared State:** Each module instance independent (no inter-module memory)
4. **HTTP Only:** Communication via HTTP/SSE only (no shared memory, no IPC)

### Deployment Constraints

1. **Full Version Only:** Not included in Lite or Minimal versions
2. **Binary Distribution:** Single executable (wkmp-dr)
3. **No Installation:** Must run without installation (zero-config)

---

## Dependencies

See **dependencies_map.md** for detailed dependency inventory.

**Critical Dependencies:**
- wkmp-common library (config, database utilities)
- Existing wkmp.db database file
- wkmp-ui (for launch button integration)

---

## Success Metrics

### Quantitative Metrics

1. **Requirements Coverage:** 100% of P0 requirements implemented and tested
2. **Test Coverage:** ≥80% code coverage (unit + integration)
3. **Performance:**
   - Table load time: <500ms for 1000 rows
   - Search response: <1s for 10,000 rows
   - Page navigation: <200ms
4. **Reliability:** Zero crashes during 1-hour stress test
5. **Startup Time:** <2s from launch to ready state

### Qualitative Metrics

1. **Usability:** User can find passages lacking MBID in <30 seconds (no training)
2. **Consistency:** UI matches wkmp-ai styling (visual inspection)
3. **Maintainability:** New filter added in <1 hour by developer
4. **Architectural Fit:** Follows WKMP patterns (code review approval)

---

## Scope Verification

**Approved Scope Changes:**
- None yet (initial plan)

**Rejected Scope Changes:**
- None yet

**Process:**
- All scope changes require explicit user approval
- Document in this section with date, rationale, approval
- Update requirements_index.md if requirements change

---

## Scope Boundaries - Quick Reference

| Feature | Status | Rationale |
|---------|--------|-----------|
| Table browsing | ✅ In Scope | Core requirement |
| Filtered views (predefined) | ✅ In Scope | Core requirement |
| Custom searches | ✅ In Scope | Core requirement |
| Saved searches | ✅ In Scope | Core requirement |
| Read-only database | ✅ In Scope | Safety-critical |
| Database editing | ❌ Out of Scope | Safety violation |
| SQL query console | ❌ Out of Scope | Security risk, defer |
| Data export | ❌ Out of Scope | Not in requirements, defer |
| Real-time updates | ❌ Out of Scope | Complexity, defer |
| wkmp-ui launch button | ✅ In Scope | Integration requirement |
| Modifications to other modules | ❌ Out of Scope | Except wkmp-ui button |

---

**Scope Statement Complete**
**Status:** Ready for Phase 2 (Specification Verification)
