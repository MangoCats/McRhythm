# Technical Debt Remediation - Requirements Index

**Source Specification:** SPEC_technical_debt_remediation.md
**Generated:** 2025-11-05
**Plan Workflow Phase:** Phase 1 - Input Validation and Scope Definition

---

## Requirements Summary

**Total Requirements:** 10 (6 Functional + 4 Non-Functional)

**Completion Target:** All 9 technical debt items resolved (6 code + 3 documentation)

---

## Functional Requirements

| ID | Title | Priority | Category | Description |
|----|-------|----------|----------|-------------|
| FR-001 | Code Organization | HIGH | Refactoring | Refactor core.rs (3,156 LOC) into modules <1,000 LOC each with clear boundaries |
| FR-002 | Code Cleanup | MEDIUM | Maintenance | Remove deprecated code, consolidate duplicates, remove obsolete files |
| FR-003 | Feature Completion | MEDIUM | Implementation | Complete 4 DEBT markers (DEBT-007, FUNC-002, FUNC-003, DEBT-002) |
| FR-004 | Code Quality | LOW | Quality | Address 19 clippy warnings, fix doctest failures, resolve dead code warnings |
| FR-005 | Documentation Completeness | LOW | Documentation | Create IMPL008, document buffer tuning, update IMPL003 |
| FR-006 | Documentation Accuracy | LOW | Documentation | Validate references, update obsolete docs, refresh READMEs |

---

## Non-Functional Requirements

| ID | Title | Priority | Constraint | Description |
|----|-------|----------|-----------|-------------|
| NFR-001 | Test Coverage Preservation | CRITICAL | Quality | All existing tests pass, no coverage reduction, 90%+ for new code |
| NFR-002 | Performance Maintenance | HIGH | Performance | No regressions in critical paths, benchmarks within ±5% |
| NFR-003 | API Compatibility | HIGH | Compatibility | No breaking changes to wkmp-common, use #[deprecated] annotations |
| NFR-004 | Documentation Quality | MEDIUM | Quality | All public APIs documented, module docs updated, traceability preserved |

---

## Technical Debt Items Mapping

**HIGH Severity (1 item):**
- TD-H-001: File size → **FR-001** (Code Organization)

**MEDIUM Severity (3 items):**
- TD-M-001: Diagnostics duplication → **FR-002** (Code Cleanup)
- TD-M-002: Deprecated auth middleware → **FR-002** (Code Cleanup)
- TD-M-003: DEBT markers (8 tracked) → **FR-003** (Feature Completion)

**LOW Severity (5 items):**
- TD-L-001: Code quality issues → **FR-004** (Code Quality)
- TD-L-002: Config fragmentation → **FR-002** (Code Cleanup)
- TD-L-003: Missing IMPL008 → **FR-005** (Documentation Completeness)
- TD-L-004: Buffer tuning workflow undocumented → **FR-005** (Documentation Completeness)
- TD-L-005: IMPL003 outdated → **FR-005** (Documentation Completeness)

---

## Requirements Detail

### FR-001: Code Organization

**Source:** SPEC lines 193-196

**Requirement Text:**
> Core.rs must be refactored into modules each <1,000 LOC
> Module boundaries must be clear and logical
> Public API surface preserved (no breaking changes)

**Target Files:**
- `wkmp-ap/src/playback/engine/core.rs` (3,156 LOC)

**Success Criteria:**
- All modules <1,000 LOC
- Clear module boundaries (no circular dependencies)
- Public API unchanged (no breaking changes)
- All existing tests pass

**Dependencies:**
- SPEC016-decoder_buffer_design.md (architecture guidance)
- SPEC028-playback_orchestration.md (engine structure)

---

### FR-002: Code Cleanup

**Source:** SPEC lines 198-201

**Requirement Text:**
> Deprecated code removed (auth_middleware.rs)
> Duplicate modules consolidated (diagnostics)
> Obsolete files removed (decoder_chain.rs legacy)

**Target Files:**
- `wkmp-ap/src/api/auth_middleware.rs` (lines 250-800 deprecated)
- `wkmp-ap/src/playback/diagnostics.rs` (514 LOC - duplicate)
- `wkmp-ap/src/playback/engine/diagnostics.rs` (860 LOC - canonical)
- Legacy decoder files (if present)

**Success Criteria:**
- Deprecated code removed (530+ LOC reduction)
- Single diagnostics module (no duplication)
- Obsolete files deleted
- All references updated
- All tests pass

---

### FR-003: Feature Completion

**Source:** SPEC lines 203-207

**Requirement Text:**
> DEBT-007: Source sample rate telemetry implementation
> REQ-DEBT-FUNC-002: Duration_played calculation
> REQ-DEBT-FUNC-003: Album metadata for events
> DEBT-002: Audio clipping detection (optional - LOW priority)

**DEBT Markers to Implement:**
1. **DEBT-007:** Source sample rate telemetry (track source file sample rates)
2. **REQ-DEBT-FUNC-002:** Duration_played calculation (track cumulative playback time)
3. **REQ-DEBT-FUNC-003:** Album metadata for events (add album UUID to events)
4. **DEBT-002:** Audio clipping detection (OPTIONAL - may defer)

**Success Criteria:**
- Each DEBT marker implemented per specification
- New functionality tested (unit + integration)
- Events include required metadata
- Telemetry accessible via API

**Open Questions:**
- Complete specification for DEBT-007 (what sample rates to track? where stored?)
- Album UUID fetching logic location (db layer? event layer?)
- Duration_played calculation point (event emission? stored state?)

---

### FR-004: Code Quality

**Source:** SPEC lines 209-212

**Requirement Text:**
> All clippy warnings addressed
> Doctest failures fixed
> Dead code warnings resolved or marked with rationale

**Known Issues:**
- 19 clippy warnings in wkmp-ap
- 5 clippy warnings in wkmp-common
- 1 doctest failure in api/handlers.rs
- 76 dead code warnings (expected after pragma removal)

**Success Criteria:**
- Clippy clean: `cargo clippy --workspace -- -W clippy::all` passes
- All doctests pass
- Dead code warnings resolved (code used, removed, or rationale documented)

---

### FR-005: Documentation Completeness

**Source:** SPEC lines 214-219

**Requirement Text:**
> IMPL008-decoder_worker_implementation.md created
> Buffer tuning workflow documented for operators
> IMPL003-project_structure.md updated to reflect current organization
> Module-level documentation updated for refactored code
> Public API documentation completeness verified

**Deliverables:**

1. **IMPL008-decoder_worker_implementation.md** (NEW)
   - Target: 500-1000 lines
   - Content: Architecture, component structure, algorithms, performance, integration
   - Audience: Future developers, maintainers
   - Success: Standalone comprehensible, enables modification confidence

2. **Buffer Tuning Workflow** (NEW)
   - Target: 300-500 lines
   - Format: IMPL009 or section in wkmp-ap README.md
   - Content: When to tune, process, parameters, applying results, troubleshooting
   - Audience: System operators, DevOps
   - Success: Non-developer can successfully tune system

3. **IMPL003-project_structure.md** (UPDATE)
   - Update wkmp-ap module structure section
   - Reflect current src/ organization
   - Remove obsolete module references
   - Update key files list

4. **Module-Level Documentation** (UPDATE)
   - Add comprehensive `//!` docs to new modules from core.rs refactoring
   - Explain module role, public API, link to specs

5. **Public API Documentation** (VERIFY)
   - Run `cargo doc --workspace --no-deps --open`
   - Check for missing docs, incomplete docs, broken links
   - Priority: Items exported from lib.rs

**Success Criteria:**
- All deliverables complete per specification
- Documentation passes quality standards (clarity, accuracy, completeness)
- Validation checklist completed

---

### FR-006: Documentation Accuracy

**Source:** SPEC lines 221-225

**Requirement Text:**
> All code-to-documentation references validated
> Obsolete documentation removed or marked deprecated
> README files updated where applicable
> Architecture diagrams updated if structure changed

**Activities:**

1. **Code-to-Doc Reference Validation**
   - Grep for `[REQ-*]`, `[SPEC-*]`, `[DEBT-*]` markers
   - Cross-reference with specifications
   - Update or remove outdated references

2. **README Updates**
   - wkmp-ap/README.md: Overview, architecture, components, build/test
   - wkmp-common/README.md: Verify accuracy after changes

3. **CHANGELOG Entry**
   - Summarize technical debt remediation
   - Document major changes (core.rs refactoring)
   - List completed features (DEBT markers)
   - Note documentation updates

**Success Criteria:**
- No broken traceability references
- READMEs accurate and current
- CHANGELOG entry complete

---

### NFR-001: Test Coverage Preservation

**Source:** SPEC lines 229-232

**Requirement Text:**
> All existing tests must pass after modifications
> No reduction in test coverage percentage
> New code adequately tested (target: 90%+ line coverage)

**Test Execution Protocol:**
```bash
# After each modification:
cargo test --workspace

# Detailed validation:
cargo test --package wkmp-ap --lib
cargo test --package wkmp-ap --test '*'
cargo test --package wkmp-common --lib

# Affected downstream modules (if wkmp-common changed):
cargo test --package wkmp-ui --lib
cargo test --package wkmp-pd --lib
# etc.
```

**STOP Protocol:**
- If ANY test fails after modification, STOP immediately
- Present failure details to user
- Await approval before modifying tests

**Success Criteria:**
- All tests pass at each increment boundary
- Coverage maintained or improved
- New features have 90%+ coverage

---

### NFR-002: Performance Maintenance

**Source:** SPEC lines 234-237

**Requirement Text:**
> No performance regressions in critical paths
> Benchmark suite results within ±5% of baseline
> Audio callback latency unchanged

**Validation:**
```bash
# After performance-sensitive changes (core.rs refactoring):
cargo bench --package wkmp-ap --no-run  # Verify compilation
cargo bench --package wkmp-ap           # Run benchmarks
```

**Success Criteria:**
- Benchmark results within ±5% of baseline
- Audio callback latency unchanged
- No regressions in critical paths

---

### NFR-003: API Compatibility

**Source:** SPEC lines 239-242

**Requirement Text:**
> No breaking changes to wkmp-common public API
> Deprecated items use #[deprecated] annotation with guidance
> Semantic versioning followed for any API changes

**Constraints:**
- wkmp-common public API MUST NOT change (breaking)
- If deprecation needed, use `#[deprecated(since = "version", note = "guidance")]`
- All downstream modules (wkmp-ui, wkmp-pd, etc.) must continue working

**Success Criteria:**
- Full workspace test suite passes
- No breaking changes introduced
- Deprecated items properly annotated

---

### NFR-004: Documentation Quality

**Source:** SPEC lines 244-248

**Requirement Text:**
> All public APIs documented
> Module-level documentation updated
> README files updated if structure changes
> Traceability comments preserved

**Quality Standards (SPEC lines 919-949):**

- **Clarity:** Target audience identified, examples provided, jargon defined
- **Accuracy:** Matches code, no obsolete references, dated updates
- **Completeness:** No TBD/TODO, cross-refs resolve, examples compile
- **Maintainability:** Sections <300 lines, clear headers, TOC if >500 lines
- **Traceability:** Links to specs, requirement IDs cited, source refs included

**Success Criteria:**
- All public APIs documented
- Documentation passes quality checklist
- Traceability preserved

---

## Priorities Summary

**CRITICAL:**
- NFR-001: Test Coverage Preservation (gates all work)

**HIGH:**
- FR-001: Code Organization (3,156 LOC → <1,000 LOC modules)
- NFR-002: Performance Maintenance
- NFR-003: API Compatibility

**MEDIUM:**
- FR-002: Code Cleanup
- FR-003: Feature Completion
- NFR-004: Documentation Quality

**LOW:**
- FR-004: Code Quality
- FR-005: Documentation Completeness
- FR-006: Documentation Accuracy

---

## Dependencies Between Requirements

```
NFR-001 (Test Coverage) → GATES ALL OTHER REQUIREMENTS
                             ↓
FR-001 (Refactor core.rs) → FR-005 (Update module docs)
                           → FR-006 (Validate references)
                           → NFR-002 (Benchmark validation)
                             ↓
FR-002 (Code Cleanup)     → FR-006 (Update docs)
                             ↓
FR-003 (DEBT Completion)  → FR-006 (Remove DEBT markers from docs)
                             ↓
FR-004 (Code Quality)     → (Independent)
                             ↓
FR-005, FR-006 (Documentation) → FINAL STEP (after all code complete)
```

---

## Open Questions for Phase 2

These questions require specification completeness verification:

1. **core.rs Refactoring Strategy:**
   - What is the optimal module split? (by feature? by layer?)
   - Chain management: separate module or within engine?
   - How to minimize public API surface?

2. **DEBT Marker Specifications:**
   - DEBT-007: What sample rates to track? Where stored?
   - FUNC-002/003: Album UUID fetching location? Duration calculation point?

3. **Configuration Cleanup:**
   - Is Config struct actually used? (requires grep)
   - External dependencies preventing removal?
   - Phased deprecation needed?

4. **Test Baseline:**
   - Pre-existing failures in any module?
   - Current benchmark baseline?
   - Known flaky tests to exclude?

5. **Documentation Decisions:**
   - Buffer tuning guide location (IMPL009 vs README vs docs/operators/)?
   - IMPL008 performance benchmark inclusion?
   - DecoderWorker state machine detail level?
   - Obsolete modules in IMPL003: historical or removed?

---

## Next Steps

**Phase 1 Complete:** Requirements extracted and indexed.

**Phase 2:** Specification Completeness Verification
- Identify gaps, ambiguities, conflicts
- Answer open questions
- Validate requirement testability

**Phase 3:** Acceptance Test Definition
- Define Given/When/Then tests for each requirement
- Create traceability matrix (100% coverage)

---

*End of Requirements Index*
