# Increment 9: System Documentation Updates

**Plan:** PLAN015 - Database Review Module (wkmp-dr)
**Increment:** 9 of 9
**Priority:** P0 (Required for release)
**Estimated Effort:** 3-4 hours
**Dependencies:** Increments 1-8 complete (wkmp-dr module implemented)

---

## Objective

Update WKMP system documentation across all 5 tiers to capture wkmp-dr module requirements, design, implementation, and execution plans.

**Rationale:** WKMP maintains strict documentation hierarchy (GOV001). New modules MUST be documented across all tiers before release to maintain architectural traceability and governance.

---

## Deliverables

### 1. Tier 0 Updates (Governance)
- ✅ GOV002-requirements_enumeration.md - Add DR document code and category codes

### 2. Tier 1 Updates (Requirements)
- ✅ REQ001-requirements.md - Add Database Review requirements section (REQ-DR-010 through REQ-DR-060)

### 3. Tier 2 Updates (Design Specifications)
- ✅ SPEC007-api_design.md - Add Database Review API section (API-DR-010 through API-DR-060)
- ✅ **SPEC027-database_review.md** - Create new design specification (NEW FILE)

### 4. Tier 3 Updates (Implementation)
- ✅ IMPL003-project_structure.md - Add wkmp-dr directory structure and Cargo.toml

### 5. Tier 4 Updates (Execution)
- ✅ EXEC001-implementation_order.md - Add Phase 13 (Database Review Module)

### 6. Other Documentation
- ✅ CLAUDE.md - Update microservices table, counts, on-demand pattern examples
- ✅ REG001_number_registry.md - Register SPEC027, increment counters

**Total Files:** 8 (7 updates + 1 new)

---

## Implementation Steps

### Step 1: Update Tier 0 (Governance) - 15 minutes

**File:** `docs/GOV002-requirements_enumeration.md`

**Changes:**

1. **Add DR document code** (lines 56-79):
   ```markdown
   | DR | database_review.md | Database Review module specifications |
   ```

2. **Add DR category codes** (new section):
   ```markdown
   ### DR (database_review.md)

   | Code | Section | Scope |
   |------|---------|-------|
   | F | Functional | Table browsing, filtering, searching |
   | NF | Non-Functional | Zero-config, read-only, authentication |
   | UI | User Interface | Inline HTML, vanilla JS, styling |
   ```

**Test:** Verify DR code follows existing pattern (AP, UI, PD, AI, LE, **DR**)

---

### Step 2: Update Tier 1 (Requirements) - 30 minutes

**File:** `docs/REQ001-requirements.md`

**Changes:**

1. **Update microservices table** (lines 18-31):
   - Add wkmp-dr row after wkmp-le

2. **Add to Full version features** (line 127):
   - "Database inspection and troubleshooting"

3. **Add new requirements section** (after line 108, before "## Additional Features"):
   ```markdown
   ### Database Review (Full version only)

   **[REQ-DR-010]** Read-only database inspection
     - **[REQ-DR-011]** Table-by-table content viewing
     - **[REQ-DR-012]** Pagination (100 rows per page)
     - **[REQ-DR-013]** Row counts per table

   **[REQ-DR-020]** Predefined filtered views
     - **[REQ-DR-021]** Passages lacking MusicBrainz Recording ID
     - **[REQ-DR-022]** Files without passages

   **[REQ-DR-030]** Custom search capabilities
     - **[REQ-DR-031]** Search by MusicBrainz Work ID
     - **[REQ-DR-032]** Search by file path pattern

   **[REQ-DR-040]** User preferences
     - **[REQ-DR-041]** Column sorting (ascending/descending)
     - **[REQ-DR-042]** Save favorite searches
     - **[REQ-DR-043]** Preference persistence (browser localStorage)

   **[REQ-DR-050]** Zero-config startup per REQ-NF-030 through REQ-NF-037

   **[REQ-DR-060]** Read-only database access (safety requirement)

   > **See:** [On-Demand Microservices](../CLAUDE.md#on-demand-microservices) for architectural pattern
   ```

**Test:** Verify requirement IDs follow GOV002 enumeration (REQ-DR-010, REQ-DR-011, etc.)

---

### Step 3: Update Tier 2 (Design - API) - 30 minutes

**File:** `docs/SPEC007-api_design.md`

**Changes:**

1. **Update module endpoint table** (lines 18-25)
2. **Update shared secret scope** (line 129) - add wkmp-dr to list
3. **Update HTML serving modules** (line 146) - add Database Review UI
4. **Add Database Review API section** (after Lyric Editor, ~line 700-800):
   - API-DR-010 through API-DR-060
   - GET /health, GET /api/tables, GET /api/table/:name
   - GET /api/filter/:filter_name, GET /api/search/*

**Test:** Verify API-DR-xxx IDs follow existing pattern (API-AP, API-UI, etc.)

---

### Step 4: Create Tier 2 (Design - New Spec) - 45 minutes

**File:** `docs/SPEC027-database_review.md` (NEW)

**Content:** Full design specification (~400 lines)

**Sections:**
- Overview (DR-OV-010, DR-OV-020)
- Functional Design (DR-F-010 through DR-F-050)
- Technical Design (DR-NF-010 through DR-NF-040)
- User Interface (DR-UI-010)
- Extensibility (DR-EXT-010, DR-EXT-020)
- Integration Points (DR-INT-010, DR-INT-020)
- Performance Targets (DR-PERF-010, DR-PERF-020)
- Security Considerations (DR-SEC-010, DR-SEC-020)

**Template:** Follow SPEC001-architecture.md format

**Test:** Verify all DR-xxx IDs unique, cross-references resolve

---

### Step 5: Update Tier 3 (Implementation) - 30 minutes

**File:** `docs/IMPL003-project_structure.md`

**Changes:**

1. **Update workspace members** (line 230) - add "wkmp-dr"
2. **Add wkmp-dr directory structure** (after wkmp-ai section, ~line 180):
   - src/main.rs, api/, db/, filters/, ui/
   - Cargo.toml with dependencies
3. **Update version build scripts** (lines 720-730):
   - Full version: cargo build --release -p wkmp-dr
   - Copy to dist/full/

**Test:** Verify wkmp-dr follows same structure pattern as wkmp-ai, wkmp-le

---

### Step 6: Update Tier 4 (Execution) - 45 minutes

**File:** `docs/EXEC001-implementation_order.md`

**Changes:**

**Add Phase 13** (after latest phase, ~line 900):
```markdown
## Phase 13: Database Review Module (wkmp-dr)

*Goal: Implement read-only database inspection tool for troubleshooting.*

- **13.1. Module Scaffolding:**
  - Create wkmp-dr/ crate, Axum server on port 5725
  - Zero-config startup, read-only database connection
  - Health endpoint, API authentication
  - Test: Module starts, health check responds

- **13.2. Table Browsing:**
  - GET /api/tables, GET /api/table/:name
  - Pagination (100 rows default)
  - Test: Browse passages table

- **13.3. Predefined Filters:**
  - passages_without_mbid, files_without_passages
  - Test: Verify filter queries

- **13.4. Custom Searches:**
  - by_work, by_path endpoints
  - Test: Search for known Work ID

- **13.5. User Interface:**
  - Inline HTML, vanilla JS, CSS custom properties
  - Table rendering, saved searches
  - Test: UI renders, controls functional

- **13.6. wkmp-ui Integration:**
  - Add "Database Review" button to Tools menu
  - Test: Launch from wkmp-ui

- **13.7. Documentation & Testing:**
  - Update SPEC027, integration tests, performance tests
  - Test: All acceptance tests pass

**Dependencies:** Phase 1 (database), Phase 4 (wkmp-ui)
**Acceptance:** All REQ-DR-xxx implemented, tests pass
```

**Test:** Verify phase dependencies correct, acceptance criteria complete

---

### Step 7: Update CLAUDE.md - 20 minutes

**File:** `CLAUDE.md`

**Changes:**

1. **Update microservices count** (line 11): 5 → 6
2. **Update microservices table** (line 295): Add wkmp-dr row
3. **Update key directories** (line 283): Add wkmp-dr/
4. **Update on-demand microservices** (line 355): Add wkmp-dr to access methods
5. **Add User Flow Example** (after Import example): Database Review workflow
6. **Update version availability** (line 365): Add wkmp-dr to Full version

**Test:** Verify all references to "5 microservices" changed to "6"

---

### Step 8: Update Registry - 10 minutes

**File:** `workflows/REG001_number_registry.md`

**Changes:**

1. **Increment next available SPEC** (line 15): 027 → 028
2. **Add assignment history**: SPEC027 | database_review.md | 2025-11-01 | ...
3. **Update document count** (line 131): SPEC count 26 → 27

**Test:** Verify SPEC027 recorded, counters accurate

---

## Acceptance Tests

### TC-DOC-001: Tier 0 Verification
**Given:** GOV002-requirements_enumeration.md updated
**When:** Search for "DR" document code
**Then:** DR code present with 3 category codes (F, NF, UI)
**Pass:** DR follows same pattern as AP, UI, PD, AI, LE

### TC-DOC-002: Tier 1 Verification
**Given:** REQ001-requirements.md updated
**When:** Search for "REQ-DR-" requirement IDs
**Then:** Found REQ-DR-010 through REQ-DR-060 (6 top-level requirements)
**Pass:** All requirements present, IDs sequential

### TC-DOC-003: Tier 2 API Verification
**Given:** SPEC007-api_design.md updated
**When:** Search for "Database Review API" section
**Then:** API-DR-010 through API-DR-060 documented
**Pass:** All endpoints documented (tables, filters, searches)

### TC-DOC-004: Tier 2 Spec Creation
**Given:** SPEC027-database_review.md created
**When:** Verify file exists
**Then:** File present with DR-OV, DR-F, DR-NF, DR-UI, DR-INT sections
**Pass:** Complete design specification, all sections present

### TC-DOC-005: Tier 3 Verification
**Given:** IMPL003-project_structure.md updated
**When:** Search for "wkmp-dr/" directory structure
**Then:** Found wkmp-dr/ with src/, api/, db/, filters/, ui/
**Pass:** Structure follows wkmp-ai pattern

### TC-DOC-006: Tier 4 Verification
**Given:** EXEC001-implementation_order.md updated
**When:** Search for "Phase 13"
**Then:** Found Phase 13: Database Review Module with 7 sub-phases
**Pass:** Complete implementation phases defined

### TC-DOC-007: CLAUDE.md Verification
**Given:** CLAUDE.md updated
**When:** Search for "6 independent HTTP servers"
**Then:** All references to microservices count = 6
**Pass:** No lingering "5 microservices" references

### TC-DOC-008: Cross-Reference Verification
**Given:** All documents updated
**When:** Follow cross-references (e.g., REQ001 → SPEC027)
**Then:** All references resolve to correct sections
**Pass:** No broken links, all IDs valid

### TC-DOC-009: Consistency Verification
**Given:** All documents updated
**When:** Verify port 5725 consistent across all docs
**Then:** Port 5725 in REQ001, SPEC007, SPEC027, IMPL003, EXEC001, CLAUDE.md
**Pass:** Port consistently documented

### TC-DOC-010: Registry Verification
**Given:** REG001_number_registry.md updated
**When:** Verify SPEC027 assignment recorded
**Then:** SPEC027 in assignment history, next available = 028
**Pass:** Registry accurate, counters correct

---

## Success Criteria

**PASS if ALL conditions met:**
1. ✅ All 8 documents updated (or 1 created)
2. ✅ All 10 acceptance tests pass
3. ✅ No broken cross-references
4. ✅ Requirement IDs follow GOV002 enumeration scheme
5. ✅ wkmp-dr consistently documented across all tiers
6. ✅ Port 5725 consistently specified
7. ✅ "Full version only" noted in all relevant sections
8. ✅ On-demand pattern references include wkmp-dr
9. ✅ Documentation review approved (manual inspection)
10. ✅ Git commit includes all 8 files

---

## Documentation Testing Checklist

**Before marking increment complete:**

- [ ] GOV002 updated (DR codes added)
- [ ] REQ001 updated (REQ-DR-xxx requirements added)
- [ ] SPEC007 updated (API-DR-xxx endpoints added)
- [ ] SPEC027 created (full design spec)
- [ ] IMPL003 updated (wkmp-dr structure added)
- [ ] EXEC001 updated (Phase 13 added)
- [ ] CLAUDE.md updated (6 microservices, wkmp-dr in tables)
- [ ] REG001 updated (SPEC027 registered)
- [ ] All cross-references verified
- [ ] All acceptance tests pass
- [ ] Documentation consistency verified
- [ ] Peer review completed

---

## Traceability

**Requirements Coverage:**
- REQ-DR-010 through REQ-DR-060 → SPEC027 design → EXEC001 Phase 13

**Implementation Traceability:**
- SPEC027 design → IMPL003 structure → Code in wkmp-dr/src/

**Testing Traceability:**
- REQ-DR-xxx → TC-DOC-001 through TC-DOC-010 (documentation tests)
- REQ-DR-xxx → TC-U-xxx, TC-I-xxx, TC-S-xxx (from PLAN015 Phase 3)

---

## Risks and Mitigation

### Risk 1: Documentation Inconsistency
**Probability:** Medium
**Impact:** Medium (confusion, broken references)
**Mitigation:**
- Use automated cross-reference verification
- Peer review all changes
- Run TC-DOC-008 (cross-reference test)

### Risk 2: Requirement ID Conflicts
**Probability:** Low
**Impact:** High (violates GOV002)
**Mitigation:**
- Verify IDs against GOV002 enumeration scheme
- Check for duplicate IDs before committing
- Use REG001 to track assignments

### Risk 3: Forgotten Document Updates
**Probability:** Medium
**Impact:** Medium (incomplete documentation)
**Mitigation:**
- Use this increment's 8-document checklist
- Search codebase for "5 microservices" references
- Verify all documents listed in GOV001 checked

---

## Estimated Effort Breakdown

| Task | Effort | Cumulative |
|------|--------|------------|
| Tier 0 (GOV002) | 15 min | 0:15 |
| Tier 1 (REQ001) | 30 min | 0:45 |
| Tier 2 (SPEC007) | 30 min | 1:15 |
| Tier 2 (SPEC027 creation) | 45 min | 2:00 |
| Tier 3 (IMPL003) | 30 min | 2:30 |
| Tier 4 (EXEC001) | 45 min | 3:15 |
| Other (CLAUDE.md) | 20 min | 3:35 |
| Other (REG001) | 10 min | 3:45 |
| Testing (10 tests) | 30 min | 4:15 |
| Review & corrections | 15 min | 4:30 |

**Total:** 3:45 to 4:30 hours

---

## Dependencies

**Upstream (must complete before this increment):**
- Increments 1-8: wkmp-dr module implemented and tested
- All code complete (documentation reflects implementation)

**Downstream (blocked by this increment):**
- None - documentation is final step before release
- However, release CANNOT proceed without complete documentation

---

## Notes

**Information Flow:**
- This increment follows **downward information flow** (normal pattern)
- Higher tiers (Requirements) inform lower tiers (Implementation)
- No upward flow (implementation does not change requirements)

**Approval Process:**
- Tier 1 (Requirements) changes: Stakeholder approval recommended
- Tier 2-4 changes: Technical lead approval
- CLAUDE.md changes: Team review

**Version Control:**
- Commit all 8 files together (atomic commit)
- Commit message: "docs: Add wkmp-dr (Database Review) module documentation across all tiers"
- Reference: PLAN015, Increment 9

---

**Increment 9 Complete Definition:**
- All 8 documents updated/created
- All 10 acceptance tests pass
- Documentation review approved
- Git commit includes all changes

**Next Step:** Release wkmp-dr module (all implementation + documentation complete)
