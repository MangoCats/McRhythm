# PLAN012 - Phase 4: Approach Selection

**Specification:** SPEC025-api_key_configuration.md
**Plan Version:** 1.0
**Date:** 2025-10-30

---

## Executive Summary

**Approaches Evaluated:** 3
**Selected Approach:** Approach B (Module-Focused with Common Utilities)
**Selection Rationale:** Lowest residual risk (LOW) with best maintainability and architectural alignment
**Key Decision Factors:** Risk-first framework prioritizes minimizing failure modes over implementation effort

---

## Approach Candidates

### Approach A: Extend wkmp-ap Pattern Directly

**Description:**
- Place all resolver logic in wkmp-ai/src/config.rs
- Duplicate TOML utilities locally in wkmp-ai
- Minimal changes to wkmp-common (only TomlConfig struct extension)

**Advantages:**
- Self-contained implementation (no cross-module dependencies)
- Fast initial implementation (copy existing pattern)
- Independent testing (no wkmp-common integration)

**Disadvantages:**
- Code duplication (TOML utilities duplicated per module)
- Future extensibility difficult (each module reimplements)
- Violates DRY principle (maintenance burden)

---

### Approach B: Module-Focused with Common Utilities (SELECTED)

**Description:**
- wkmp-common provides TOML utilities (atomic write, field preservation, permissions)
- wkmp-ai provides resolver, database accessors, settings sync
- Generic sync mechanism uses HashMap interface (extensible)

**Advantages:**
- Code reuse (TOML utilities shared across all future modules)
- Clear separation of concerns (common utilities vs module-specific logic)
- Easy extensibility (future modules reuse utilities)
- Matches existing wkmp-common architecture (config module already exists)

**Disadvantages:**
- Slightly higher upfront effort (implement utilities generically)
- Cross-module testing required (wkmp-common + wkmp-ai integration)

---

### Approach C: Full Generic Framework in wkmp-common

**Description:**
- wkmp-common provides generic API key resolver (trait-based)
- wkmp-common provides generic settings sync framework
- wkmp-ai implements traits for AcoustID-specific behavior

**Advantages:**
- Maximum code reuse (entire resolver framework shared)
- Strong type safety (trait-based design)
- Best long-term extensibility

**Disadvantages:**
- Over-engineering for current needs (only one API key currently)
- Higher upfront design complexity (trait design, generics)
- YAGNI violation (may not need full framework)
- Higher testing burden (abstract framework + concrete implementations)

---

## Risk Assessment (Primary Decision Factor)

### Approach A Risk Assessment

**Failure Modes:**

**FM-A-001: Code Duplication Leads to Divergent Implementations**
- **Description:** TOML utilities duplicated per module drift over time (bug fixes applied inconsistently)
- **Probability:** MEDIUM (happens without strong code review)
- **Impact:** MEDIUM (inconsistent behavior, security vulnerabilities)
- **Risk Level:** MEDIUM
- **Mitigation:** Strict code review, shared test cases
- **Residual Risk:** MEDIUM (mitigation reduces but doesn't eliminate)

**FM-A-002: Future Module Integration Requires Refactoring**
- **Description:** Adding MusicBrainz API key requires refactoring both modules to extract common utilities
- **Probability:** HIGH (SPEC025 explicitly plans future API keys)
- **Impact:** MEDIUM (rework effort, potential regressions)
- **Risk Level:** MEDIUM-HIGH
- **Mitigation:** Plan refactoring sprint
- **Residual Risk:** MEDIUM (mitigation reduces impact but probability remains high)

**FM-A-003: TOML Write Atomicity Bugs Duplicated**
- **Description:** Subtle atomicity bugs (race conditions, partial writes) duplicated across modules
- **Probability:** LOW (well-understood pattern)
- **Impact:** HIGH (data corruption, lost configuration)
- **Risk Level:** MEDIUM
- **Mitigation:** Comprehensive testing
- **Residual Risk:** LOW-MEDIUM

**Overall Residual Risk:** MEDIUM (FM-A-001 and FM-A-002 cannot be fully mitigated)

---

### Approach B Risk Assessment (SELECTED)

**Failure Modes:**

**FM-B-001: TOML Utilities Regression Affects Multiple Modules**
- **Description:** Bug in wkmp-common TOML utilities breaks all modules using them
- **Probability:** LOW (comprehensive unit tests, single implementation)
- **Impact:** HIGH (multi-module failure)
- **Risk Level:** MEDIUM
- **Mitigation:**
  - Comprehensive unit tests (tc_u_toml_001-007)
  - Integration tests per module
  - Semantic versioning (breaking changes detected)
- **Residual Risk:** LOW (strong mitigation via testing)

**FM-B-002: Cross-Module Testing Gaps**
- **Description:** Integration between wkmp-common utilities and wkmp-ai resolver not adequately tested
- **Probability:** LOW (integration tests defined: tc_i_e2e_001-004)
- **Impact:** MEDIUM (runtime failures in production)
- **Risk Level:** LOW-MEDIUM
- **Mitigation:**
  - Integration tests for end-to-end startup (tc_i_e2e_*)
  - System tests for user workflows (tc_s_workflow_*)
  - 100% requirement coverage
- **Residual Risk:** LOW (comprehensive test coverage)

**FM-B-003: Generic Interface Not Flexible Enough**
- **Description:** HashMap-based sync interface inadequate for future API keys (encryption, validation)
- **Probability:** LOW (HashMap supports arbitrary string key-value, covers known use cases)
- **Impact:** LOW (refactor sync interface, low-risk change)
- **Risk Level:** LOW
- **Mitigation:**
  - Design review against future requirements (APIK-FUT-*)
  - Interface documented as extensible (can add validation param)
- **Residual Risk:** VERY LOW (low probability, low impact, easy to refactor)

**Overall Residual Risk:** LOW (all failure modes mitigated to LOW or LOW-MEDIUM)

---

### Approach C Risk Assessment

**Failure Modes:**

**FM-C-001: Over-Abstraction Leads to Complexity**
- **Description:** Trait-based framework harder to understand and maintain (fewer contributors can modify)
- **Probability:** MEDIUM (abstract code requires higher skill level)
- **Impact:** MEDIUM (slower maintenance, harder debugging)
- **Risk Level:** MEDIUM
- **Mitigation:** Extensive documentation, examples
- **Residual Risk:** MEDIUM (inherent complexity remains)

**FM-C-002: YAGNI - Framework Unused**
- **Description:** Full framework built but only one API key implemented (wasted effort)
- **Probability:** MEDIUM (future API keys uncertain timeline)
- **Impact:** LOW (works correctly, just over-engineered)
- **Risk Level:** LOW-MEDIUM
- **Mitigation:** Incremental approach (wait for second API key before abstracting)
- **Residual Risk:** LOW-MEDIUM (effort wasted, but functional)

**FM-C-003: Trait Design Errors Require Breaking Changes**
- **Description:** Trait interface design flaws discovered during first implementation require breaking changes
- **Probability:** MEDIUM (hard to get generic design right first time)
- **Impact:** HIGH (refactor entire framework, update all implementations)
- **Risk Level:** MEDIUM-HIGH
- **Mitigation:** Prototype with two concrete implementations before generalizing
- **Residual Risk:** MEDIUM (mitigation reduces but doesn't eliminate)

**Overall Residual Risk:** MEDIUM (FM-C-001 and FM-C-003 remain medium after mitigation)

---

## Quality Characteristics (Secondary Decision Factor)

### Maintainability

**Approach A:** LOW
- Code duplication increases maintenance burden
- Bug fixes must be applied to multiple modules
- Inconsistent implementations lead to confusion

**Approach B:** HIGH (BEST)
- Single implementation of utilities (maintain once)
- Clear module boundaries (common vs specific)
- Well-established pattern (matches existing wkmp-common structure)

**Approach C:** MEDIUM
- Trait abstraction increases cognitive load
- More complex debugging (trait dispatch)
- Higher learning curve for new contributors

**Winner:** Approach B (maintainability is critical for long-term project health)

---

### Test Coverage

**Approach A:** MEDIUM
- Unit tests per module (duplication)
- Integration tests straightforward
- Manual tests required per module

**Approach B:** HIGH (BEST)
- Unit tests for utilities (shared, tested once)
- Unit tests for resolver (module-specific)
- Integration tests cover cross-module interactions
- 100% requirement coverage achievable

**Approach C:** MEDIUM
- Abstract framework testing complex (trait implementations)
- More mocking required (trait boundaries)
- Higher testing effort for lower actual coverage

**Winner:** Approach B (test coverage easiest to achieve and maintain)

---

### Architectural Alignment

**Approach A:** LOW
- Violates DRY principle
- Inconsistent with wkmp-common patterns (utilities not shared)
- Forces future refactoring

**Approach B:** HIGH (BEST)
- Follows existing wkmp-common pattern (config.rs provides utilities)
- Matches wkmp-ap/src/db/settings.rs pattern (module-specific accessors)
- Natural extension of current architecture

**Approach C:** MEDIUM
- Introduces new trait-based pattern (inconsistent with rest of codebase)
- Higher abstraction level than current WKMP design
- May be future direction but premature now

**Winner:** Approach B (best alignment with existing architecture)

---

## Effort Estimation (Tertiary Consideration)

### Approach A: 18-24 hours

**Breakdown:**
- wkmp-ai resolver: 4-6 hours
- wkmp-ai TOML utilities (duplicate): 4-6 hours
- wkmp-ai database accessors: 2-3 hours
- wkmp-ai settings sync: 2-3 hours
- Web UI endpoint: 2-3 hours
- Web UI page: 2-3 hours
- Testing: 4-6 hours

**Future Cost:** +8-12 hours per additional API key (duplicate utilities, testing)

---

### Approach B: 20-28 hours (SELECTED)

**Breakdown:**
- wkmp-common TomlConfig extension: 1-2 hours
- wkmp-common TOML utilities (generic): 4-6 hours
- wkmp-common tests: 3-4 hours
- wkmp-ai resolver: 3-4 hours
- wkmp-ai database accessors: 2-3 hours
- wkmp-ai settings sync: 2-3 hours
- wkmp-ai tests: 3-4 hours
- Web UI endpoint: 2-3 hours
- Web UI page: 2-3 hours
- Integration tests: 3-4 hours

**Future Cost:** +4-6 hours per additional API key (reuse utilities, resolver pattern)

**Effort Comparison:** +2-4 hours upfront vs Approach A, but saves 4-6 hours per future API key

---

### Approach C: 32-42 hours

**Breakdown:**
- wkmp-common trait design: 4-6 hours
- wkmp-common generic resolver: 6-8 hours
- wkmp-common generic sync: 4-6 hours
- wkmp-common tests: 4-6 hours
- wkmp-ai trait implementation: 4-6 hours
- wkmp-ai tests: 4-6 hours
- Web UI endpoint: 2-3 hours
- Web UI page: 2-3 hours
- Integration tests: 4-6 hours

**Future Cost:** +2-3 hours per additional API key (implement trait only)

**Effort Comparison:** +12-14 hours upfront vs Approach B, saves 1-3 hours per future API key (diminishing returns)

---

## Decision Summary

### Risk Comparison (Primary Factor)

| Approach | Residual Risk | Risk Factors |
|----------|---------------|--------------|
| A | MEDIUM | Code duplication, future refactoring, divergent implementations |
| **B** | **LOW** | All failure modes mitigated to LOW or LOW-MEDIUM |
| C | MEDIUM | Over-abstraction, YAGNI, trait design errors |

**Winner by Risk:** Approach B (lowest residual risk)

---

### Quality Comparison (Secondary Factor)

| Approach | Maintainability | Test Coverage | Architectural Alignment |
|----------|-----------------|---------------|-------------------------|
| A | LOW | MEDIUM | LOW |
| **B** | **HIGH** | **HIGH** | **HIGH** |
| C | MEDIUM | MEDIUM | MEDIUM |

**Winner by Quality:** Approach B (best in all three quality dimensions)

---

### Effort Comparison (Tertiary Consideration)

| Approach | Upfront Effort | Future Cost per API Key | Total for 3 API Keys |
|----------|----------------|-------------------------|----------------------|
| A | 18-24h | +8-12h | 34-48h |
| **B** | **20-28h** | **+4-6h** | **28-40h** |
| C | 32-42h | +2-3h | 36-48h |

**Note:** Effort is NOT the decision factor (per Risk-First Framework), but Approach B is competitive even on effort.

---

## Selected Approach: Approach B

**Rationale (Risk-First Framework):**

1. **Lowest Residual Risk (PRIMARY):** Approach B achieves LOW overall residual risk
   - All failure modes mitigated to LOW or LOW-MEDIUM
   - Comprehensive test coverage eliminates most risks
   - Single implementation reduces divergence risk

2. **Best Quality (SECONDARY):** Approach B wins on all quality dimensions
   - HIGH maintainability (code reuse, clear boundaries)
   - HIGH test coverage (100% requirement coverage achievable)
   - HIGH architectural alignment (matches existing patterns)

3. **Competitive Effort (TERTIARY):** Approach B has lowest total effort for 3 API keys
   - Only +2-4h upfront vs Approach A
   - Saves 4-6h per future API key (ROI after 2nd API key)
   - Much lower effort than Approach C (-12-14h upfront)

**Risk-First Decision:** Choose Approach B (lowest risk, best quality, competitive effort)

---

## Architecture Decision Record (ADR)

### ADR-PLAN012-001: Module-Focused Implementation with Common Utilities

**Status:** ACCEPTED

**Context:**
- Need to implement multi-tier API key configuration for wkmp-ai (AcoustID)
- Future API keys planned (MusicBrainz, potentially others)
- Balance between code reuse and over-engineering
- Risk-first decision framework prioritizes failure risk reduction

**Decision:**
Implement API key configuration using Approach B (Module-Focused with Common Utilities):
- wkmp-common provides TOML utilities (atomic write, field preservation, permissions)
- wkmp-ai provides resolver, database accessors, settings sync
- Generic sync mechanism uses HashMap interface

**Consequences:**

**Positive:**
- Lowest residual risk (LOW) - all failure modes mitigated
- High code reuse without over-abstraction
- Future API keys benefit from shared utilities (4-6h savings per key)
- Strong architectural alignment with existing patterns
- Achievable 100% test coverage

**Negative:**
- Cross-module changes required (wkmp-common + wkmp-ai)
- Integration testing more complex than self-contained approach
- +2-4h upfront effort vs duplicating code

**Alternatives Considered:**
- Approach A: Lower upfront effort but MEDIUM risk (code duplication, future refactoring)
- Approach C: Best long-term reuse but MEDIUM risk (over-abstraction, YAGNI, trait design errors)

**Risk Mitigation:**
- Comprehensive unit tests for TOML utilities (tc_u_toml_001-007)
- Integration tests for cross-module interactions (tc_i_e2e_001-004)
- Semantic versioning for wkmp-common (detect breaking changes)
- Design review against future requirements (APIK-FUT-*)

**Risk Assessment:**
- FM-B-001: TOML utilities regression → LOW (comprehensive testing)
- FM-B-002: Cross-module testing gaps → LOW (integration tests defined)
- FM-B-003: Generic interface inflexible → VERY LOW (HashMap extensible)

**Overall Residual Risk:** LOW (acceptable for implementation)

**Alignment with Risk-First Framework:**
- Risk is primary decision factor: Approach B has lowest residual risk ✓
- Quality is secondary: Approach B wins on all quality dimensions ✓
- Effort is tertiary: Approach B competitive on effort (lowest total for 3 API keys) ✓

---

## Implementation Implications

### Component Responsibilities

**wkmp-common (4-6 hours):**
- Extend TomlConfig struct (add acoustid_api_key field)
- Implement write_toml_config() (atomic write with temp file + rename)
- Implement set_unix_permissions_0600() (Unix-only)
- Unit tests (tc_u_toml_001-007)

**wkmp-ai (12-16 hours):**
- Implement resolve_acoustid_api_key() (multi-tier resolution)
- Implement get/set_acoustid_api_key() (database accessors)
- Implement sync_settings_to_toml() (HashMap-based)
- Unit tests (tc_u_res_*, tc_u_wb_*, tc_u_val_*, tc_u_db_*, tc_u_sec_*)
- Integration with startup (main.rs)
- Web UI endpoint (POST /api/settings/acoustid_api_key)
- Web UI settings page (/settings)
- Integration tests (tc_i_e2e_*, tc_i_ui_*, tc_i_recovery_*, tc_i_concurrent_*)

**Testing (6-8 hours):**
- System tests (tc_s_workflow_001-003)
- Manual tests (tc_m_migration_001-003, tc_m_failure_001-003)
- Documentation updates

---

## Next Steps

**Phase 5: Implementation Breakdown**
- Break Approach B into 8-12 increments (2-4 hours each)
- Sequence by dependency (foundation → core → integration → UI)
- Define checkpoints every 5-10 increments

**Phase 6: Effort Estimation**
- Detailed time estimates per increment
- Identify parallel vs sequential work
- Add contingency (20-30%)

**Phase 7: Risk Assessment**
- Identify implementation-specific risks
- Define mitigation strategies
- Calculate residual risk

---

**Phase 4 (Approach Selection):** COMPLETE
**Selected Approach:** Approach B (Module-Focused with Common Utilities)
**Residual Risk:** LOW (acceptable for implementation)
**Next Phase:** Phase 5 - Implementation Breakdown
