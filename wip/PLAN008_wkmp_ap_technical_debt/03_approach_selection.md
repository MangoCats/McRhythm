# Approach Selection - wkmp-ap Technical Debt Remediation

**Plan:** PLAN008
**Phase:** 4 - Approach Selection
**Date:** 2025-10-29

---

## Overview

This document evaluates implementation approaches for each major debt item, performs risk assessment, and selects the lowest-risk approach per CLAUDE.md Decision-Making Framework.

**Decision Framework (in priority order):**
1. **Risk** (Primary) - Minimize failure probability
2. **Quality** (Secondary) - Tiebreaker when risks equivalent
3. **Effort** (Tertiary) - Only when risk and quality equivalent

---

## Debt Item 1: POST/PUT Authentication

### Approaches Evaluated

#### Approach A: Body-Based Authentication (Recommended)
**Description:** Extract `shared_secret` from JSON request body

**Implementation:**
```rust
Method::POST | Method::PUT => {
    // Extract JSON body
    let body_bytes = hyper::body::to_bytes(req.body_mut()).await?;
    let json: Value = serde_json::from_slice(&body_bytes)?;

    // Extract and validate secret
    if let Some(secret) = json.get("shared_secret").and_then(|v| v.as_str()) {
        if secret == server_secret {
            // Reconstruct request with body
            return Ok(Authenticated);
        }
    }

    Err(auth_error_response(401, "authentication_required", "..."))
}
```

**Risk Assessment:**

| Failure Mode | Probability | Impact | Mitigation | Residual Risk |
|--------------|-------------|--------|------------|---------------|
| Body consumed before handler | Low | Medium | Reconstruct request with buffered body | Low |
| Malformed JSON breaks parsing | Low | Low | Catch parse errors, return 401 | Low |
| Large body DoS attack | Low | Low | Axum has size limits built-in | Low |
| Client compatibility issues | Low | Medium | Document API, provide examples | Low |

**Overall Residual Risk:** Low

**Quality Characteristics:**
- Maintainability: High (simple, follows existing pattern)
- Test coverage: High (easy to test with reqwest client)
- Architectural alignment: High (matches query-based GET auth pattern)

**Effort:** 4 hours (implementation + 6 tests)

---

#### Approach B: Header-Based Authentication
**Description:** Use `Authorization: Bearer <secret>` header

**Implementation:**
```rust
Method::POST | Method::PUT => {
    if let Some(auth_header) = req.headers().get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                let token = &auth_str[7..];
                if token == server_secret {
                    return Ok(Authenticated);
                }
            }
        }
    }
    Err(auth_error_response(401, "authentication_required", "..."))
}
```

**Risk Assessment:**

| Failure Mode | Probability | Impact | Mitigation | Residual Risk |
|--------------|-------------|--------|------------|---------------|
| Inconsistent with GET auth | High | Medium | Document inconsistency | Medium |
| Client must change from query to header | High | High | Breaking change for existing clients | **High** |
| Header injection attacks | Low | Low | Axum validates headers | Low |

**Overall Residual Risk:** **Medium-High** (breaking change)

**Quality Characteristics:**
- Maintainability: High (standard pattern)
- Test coverage: High
- Architectural alignment: **Low** (inconsistent with GET auth)

**Effort:** 3 hours

---

#### Approach C: Middleware Pattern with State Extraction
**Description:** Use Axum middleware to extract body and validate before routing

**Risk Assessment:**

| Failure Mode | Probability | Impact | Mitigation | Residual Risk |
|--------------|-------------|--------|------------|---------------|
| Axum 0.7 state extraction complexity | Medium | High | Research Axum docs, trial implementation | Medium |
| Body buffering performance overhead | Low | Low | Profile if needed | Low |
| Middleware ordering issues | Medium | Medium | Careful router configuration | Low-Medium |

**Overall Residual Risk:** **Medium** (complexity)

**Quality Characteristics:**
- Maintainability: Medium (more complex than A)
- Test coverage: Medium (middleware testing harder)
- Architectural alignment: Medium (cleaner separation but more complex)

**Effort:** 8 hours (research + implementation)

---

### Decision: Approach A (Body-Based Authentication)

**Rationale:**

**Risk (Primary Criterion):**
- Approach A: Low risk (all failure modes mitigated)
- Approach B: Medium-High risk (breaking change for clients)
- Approach C: Medium risk (Axum complexity)

**Winner by lowest risk:** Approach A

**Quality (Secondary - Not Needed):**
All approaches have acceptable quality, but risk already determined winner.

**Effort (Tertiary - Not Needed):**
Not evaluated as decision made on risk alone.

**Per CLAUDE.md:** "Risk is the primary decision factor. Choose the approach with lowest residual risk."

**ADR (Inline):**

```markdown
# ADR-DEBT-001: POST/PUT Authentication via JSON Body

**Status:** Accepted
**Date:** 2025-10-29
**Context:** wkmp-ap allows POST/PUT requests without authentication (security vulnerability)

**Decision:** Implement body-based authentication by extracting `shared_secret` from JSON request body

**Rationale:**
- **Lowest Risk:** All failure modes have low residual risk after mitigation
- **Consistency:** Matches existing query-based GET authentication pattern
- **Non-Breaking:** Additive change (clients add field to existing JSON)
- **Simple:** Straightforward implementation, easy to test

**Alternatives Rejected:**
- Header-based auth: Breaking change (HIGH risk), inconsistent with GET pattern
- Middleware pattern: Added complexity (MEDIUM risk), no quality benefit

**Consequences:**
- Positive: Security vulnerability eliminated, backward-compatible deployment possible
- Negative: Request body must be buffered (negligible performance impact)
- Neutral: Clients must update to include `shared_secret` field in JSON
```

---

## Debt Item 2-3: File Paths & Buffer Config

### Approach: Direct Implementation (No Alternatives)

**Rationale:** These are straightforward bug fixes with single obvious solution:

1. **File Paths:** Add field to struct, use in error construction
2. **Buffer Config:** Query database, use defaults if NULL

**Risk:** Low (simple changes, well-understood patterns)
**Quality:** High (improves debugging/configurability)
**Effort:** 2-3 hours each

**No ADR needed** - implementation approach is unambiguous

---

## Debt Item 4: Developer UI Telemetry

### Approaches Evaluated

#### Approach A: On-Demand Query (Recommended)
**Description:** Query decoder worker telemetry when `get_buffer_chain_state()` is called

**Risk Assessment:**

| Failure Mode | Probability | Impact | Mitigation | Residual Risk |
|--------------|-------------|--------|------------|---------------|
| Query overhead impacts UI responsiveness | Low | Low | Async query, lock-free reads | Low |
| Stale data shown in UI | Low | Low | Acceptable for diagnostics | Low |
| Decoder worker not available | Low | Low | Return None, UI handles gracefully | Low |

**Overall Residual Risk:** Low

**Quality:** High (simple, no background threads)
**Effort:** 4 hours

---

#### Approach B: Polling with Cached State
**Description:** Background task polls decoder state every 100ms, caches for UI queries

**Risk Assessment:**

| Failure Mode | Probability | Impact | Mitigation | Residual Risk |
|--------------|-------------|--------|------------|---------------|
| Polling overhead (CPU, locks) | Medium | Medium | Profile, tune interval | Low-Medium |
| Cache staleness | Low | Low | 100ms is fast enough | Low |
| Background task management | Low | Low | Use tokio::spawn | Low |

**Overall Residual Risk:** Low-Medium

**Quality:** Medium (adds complexity)
**Effort:** 8 hours

---

#### Approach C: Event-Driven Updates
**Description:** Decoder worker publishes telemetry events, UI subscribes

**Risk Assessment:**

| Failure Mode | Probability | Impact | Mitigation | Residual Risk |
|--------------|-------------|--------|------------|---------------|
| Event channel overhead | Low | Low | Bounded channels | Low |
| Event loss on high load | Low | Low | Diagnostics, not critical | Low |
| Complex pub/sub management | Medium | Medium | Careful channel lifecycle | Low-Medium |

**Overall Residual Risk:** Low-Medium

**Quality:** Medium (most complex)
**Effort:** 12 hours

---

### Decision: Approach A (On-Demand Query)

**Rationale:**

**Risk:** All approaches have Low to Low-Medium risk, but A is simplest (lowest probability of implementation errors)
**Quality:** A has highest maintainability (no background tasks, no event channels)
**Effort:** A is 50-67% less effort than alternatives

**Winner:** Approach A (lowest risk among equivalent-risk options, best quality)

**ADR (Inline):**

```markdown
# ADR-DEBT-004: On-Demand Telemetry Query

**Status:** Accepted
**Date:** 2025-10-29
**Context:** Developer UI missing decoder state, sample rate, fade stage

**Decision:** Query decoder worker telemetry on-demand when UI requests buffer chain state

**Rationale:**
- **Simplest:** No background tasks, no polling, no event channels
- **Low Risk:** Async query with lock-free reads, negligible overhead
- **Acceptable Latency:** UI queries every 500ms-1s, on-demand query fast enough

**Alternatives Rejected:**
- Polling: Adds background task complexity, 100% CPU overhead even when UI closed
- Event-driven: Most complex, over-engineering for diagnostics use case

**Consequences:**
- Positive: Simple implementation, no overhead when UI not in use
- Negative: Telemetry not available if decoder worker deallocated (acceptable - return None)
```

---

## Debt Item 5-6: Album Metadata & Duration Tracking

### Approach: Direct Implementation (No Alternatives)

**Rationale:**
- **Albums:** Database query joining tables (standard pattern)
- **Duration:** Track start time, calculate elapsed (simple timing)

**Risk:** Low (well-understood patterns, testable)
**Quality:** High (improves event metadata accuracy)
**Effort:** 3-4 hours each

**No ADR needed** - implementation approach is unambiguous

---

## Debt Item 7: .unwrap() Audit

### Approaches Evaluated

#### Approach A: Targeted Hot-Path Replacement (Recommended)
**Description:** Replace .unwrap() in audio thread and decoder workers only

**Risk Assessment:**

| Failure Mode | Probability | Impact | Mitigation | Residual Risk |
|--------------|-------------|--------|------------|---------------|
| Miss critical .unwrap() location | Low | Medium | Review audio callback path thoroughly | Low |
| Error propagation adds verbosity | Low | Low | Acceptable for safety | Low |
| Test code .unwrap() remains | Low | Low | Acceptable in tests | Low |

**Overall Residual Risk:** Low

**Quality:** High (focuses on highest-risk code)
**Effort:** 6 hours (11 mutex locks + 3 event channels)

---

#### Approach B: Comprehensive Elimination
**Description:** Replace all 376 .unwrap() calls project-wide

**Risk Assessment:**

| Failure Mode | Probability | Impact | Mitigation | Residual Risk |
|--------------|-------------|--------|------------|---------------|
| Introduce bugs in low-risk code | Medium | Low | Careful refactoring | Low-Medium |
| Massive code churn | High | Medium | Extensive testing required | Medium |
| Test code becomes verbose | Medium | Low | Allow in tests | Low |

**Overall Residual Risk:** Medium

**Quality:** Highest (most comprehensive)
**Effort:** 40 hours (376 instances)

---

### Decision: Approach A (Targeted Hot-Path Replacement)

**Rationale:**

**Risk:**
- Approach A: Low risk (focused scope)
- Approach B: Medium risk (large churn, potential bugs)

**Winner by lowest risk:** Approach A

**Effort:** B requires 6x more effort for marginal safety improvement in non-critical code

**Per CLAUDE.md:** "If the lowest-risk approach requires 2x effort versus a higher-risk approach, choose the lowest-risk approach. Effort differential is secondary to risk reduction."

However, here the *lower-risk* approach (A) also requires *less* effort - clear winner.

**ADR (Inline):**

```markdown
# ADR-DEBT-007: Targeted .unwrap() Replacement in Hot Paths

**Status:** Accepted
**Date:** 2025-10-29
**Context:** 376 .unwrap() calls create panic risk; audio thread safety critical

**Decision:** Replace .unwrap() in audio hot paths only (buffer.rs, events.rs)

**Rationale:**
- **Risk-Focused:** Audio thread panics are highest-impact failure mode
- **Lower Risk:** Smaller scope reduces chance of introducing bugs
- **Efficient:** 15% effort for 80% safety improvement (Pareto principle)

**Alternatives Rejected:**
- Comprehensive elimination: Higher risk (large churn), 6x effort, marginal benefit

**Consequences:**
- Positive: Audio thread safe from mutex poison panics, manageable scope
- Negative: Test code retains .unwrap() (acceptable trade-off)
- Future: Can audit remaining .unwrap() in separate effort if needed
```

---

## Debt Item 8: engine.rs Refactoring

### Approaches Evaluated

#### Approach A: Incremental Module Extraction (Recommended)
**Description:** Extract 3 modules one at a time, test after each

**Steps:**
1. Extract `engine/diagnostics.rs` (status queries, telemetry)
2. Extract `engine/queue.rs` (enqueue, skip, queue operations)
3. Extract `engine/core.rs` (state management, lifecycle)
4. Create `engine/mod.rs` re-exporting public API

**Risk Assessment:**

| Failure Mode | Probability | Impact | Mitigation | Residual Risk |
|--------------|-------------|--------|------------|---------------|
| Break public API accidentally | Low | High | Run tests after each extraction | Low |
| Circular dependencies | Medium | Medium | Careful dependency ordering | Low |
| Merge conflicts if parallel work | Low | Low | Coordinate with team | Low |

**Overall Residual Risk:** Low

**Quality:** High (testable at each step)
**Effort:** 10 hours (3-4 hours per module)

---

#### Approach B: Big-Bang Refactoring
**Description:** Reorganize all 3,573 lines in one large change

**Risk Assessment:**

| Failure Mode | Probability | Impact | Mitigation | Residual Risk |
|--------------|-------------|--------|------------|---------------|
| Break functionality unnoticed | High | High | Comprehensive testing | **Medium-High** |
| Difficult code review | High | Medium | Split commits | Medium |
| Hard to debug if issues found | High | High | Revert entire change | Medium |

**Overall Residual Risk:** **Medium-High**

**Quality:** Medium (harder to review)
**Effort:** 8 hours (slightly faster but much riskier)

---

### Decision: Approach A (Incremental Module Extraction)

**Rationale:**

**Risk:**
- Approach A: Low risk (testable increments, easy rollback)
- Approach B: Medium-High risk (hard to test/review, all-or-nothing)

**Winner by lowest risk:** Approach A

**Effort:** A requires 2 hours more, but risk differential justifies it (per CLAUDE.md)

**ADR (Inline):**

```markdown
# ADR-DEBT-008: Incremental engine.rs Refactoring

**Status:** Accepted
**Date:** 2025-10-29
**Context:** engine.rs is 3,573 lines (14% of codebase), difficult to maintain

**Decision:** Extract 3 modules incrementally with testing after each step

**Rationale:**
- **Lower Risk:** Test after each extraction, easy rollback if issues
- **Reviewable:** Smaller commits easier to review than 3,573-line diff
- **Safe:** Public API verified unchanged at each step

**Alternatives Rejected:**
- Big-bang refactoring: Much higher risk, difficult review, hard to debug

**Consequences:**
- Positive: Safe refactoring with verification at each step
- Negative: Takes 2 hours longer than big-bang (acceptable for risk reduction)
- Future: Can apply same pattern to mixer.rs (1,933 lines) later
```

---

## Debt Items 9-13: Warnings, Config, Cleanup, Clipping

### Approach: Direct Implementation (No Alternatives)

**Rationale:** These are trivial fixes with single obvious approach:

9. **Compiler Warnings:** Run `cargo fix`, remove unused code
10. **Config Duplication:** Check main.rs, delete obsolete file
11. **Backup Files:** Delete .backup files
12. **Clipping Logs:** Add warn! call with rate limiting
13. **Outdated TODO:** Delete comment

**Risk:** Negligible (mechanical changes, testable)
**Quality:** High (improves code cleanliness)
**Effort:** 1-2 hours combined

**No ADR needed** - implementation trivial

---

## Approach Selection Summary

| Debt Item | Selected Approach | Risk Level | Key Decision Factor |
|-----------|-------------------|------------|---------------------|
| 1. POST/PUT Auth | Body-Based (A) | Low | Lowest risk, non-breaking |
| 2. File Paths | Direct Implementation | Low | Unambiguous fix |
| 3. Buffer Config | Direct Implementation | Low | Unambiguous fix |
| 4. Telemetry | On-Demand Query (A) | Low | Simplest, no overhead |
| 5. Album Metadata | Direct Implementation | Low | Standard DB pattern |
| 6. Duration Tracking | Direct Implementation | Low | Simple timing |
| 7. .unwrap() Audit | Targeted Hot-Path (A) | Low | Risk-focused, efficient |
| 8. engine.rs Refactor | Incremental Extraction (A) | Low | Testable steps, safe |
| 9-13. Misc | Direct Implementation | Negligible | Trivial fixes |

**Overall Plan Risk:** LOW (all approaches selected for minimum failure probability)

---

## ADR Index

**Architecture Decision Records created in this phase:**

1. **ADR-DEBT-001:** POST/PUT Authentication via JSON Body
2. **ADR-DEBT-004:** On-Demand Telemetry Query
3. **ADR-DEBT-007:** Targeted .unwrap() Replacement in Hot Paths
4. **ADR-DEBT-008:** Incremental engine.rs Refactoring

**Status:** All Accepted
**Rationale:** All decisions based on risk minimization per CLAUDE.md framework

---

## Phase 4 Complete

**Deliverables:**
- ✅ Multiple approaches evaluated for major debt items
- ✅ Risk assessment performed for each approach
- ✅ Lowest-risk approaches selected
- ✅ Decisions documented as inline ADRs
- ✅ Risk-based rationale provided per CLAUDE.md

**Next Phase:** Phase 5 - Implementation Breakdown (sized increments)
