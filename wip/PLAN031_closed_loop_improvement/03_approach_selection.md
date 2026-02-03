# PLAN031 Approach Selection

## Viable Approaches

### Approach A: Conditional Compilation

**Description:** Use Cargo feature flags to conditionally compile testing hooks
```rust
#[cfg(feature = "testing")]
if let Some(evaluator) = &self.test_evaluator {
    evaluator.record_result(...).await;
}
```

**Key Characteristics:**
- Zero runtime overhead when feature disabled
- Clear separation of test vs production code
- Requires feature flag management

### Approach B: Optional Evaluator Pattern

**Description:** Services hold `Option<Arc<TestEvaluator>>`, check at runtime
```rust
if let Some(evaluator) = &self.test_evaluator {
    evaluator.record_result(...).await;
}
```

**Key Characteristics:**
- Simple implementation
- Works with existing patterns (Arc already used)
- Minor Option check overhead

### Approach C: Trait-based Observer

**Description:** Define trait with default no-op, services accept trait object
```rust
trait ImportObserver: Send + Sync {
    async fn on_result(&self, _: &ImportResult) {} // default no-op
}
```

**Key Characteristics:**
- Maximum flexibility
- Good for extensibility
- More boilerplate

## Failure Mode Analysis

### Approach A: Conditional Compilation

| FM ID | Failure Mode | Probability | Impact | Residual Risk |
|-------|--------------|-------------|--------|---------------|
| FM-A-01 | Feature flag conflicts | LOW | Moderate | LOW |
| FM-A-02 | Debug/release behavior differs | MEDIUM | Major | MEDIUM |
| FM-A-03 | CI complexity | MEDIUM | Minor | LOW |

### Approach B: Optional Evaluator

| FM ID | Failure Mode | Probability | Impact | Residual Risk |
|-------|--------------|-------------|--------|---------------|
| FM-B-01 | Forgetting Option check | SPECULATIVE | None (Rust prevents) | LOW |
| FM-B-02 | Arc overhead | SPECULATIVE | Minor | LOW |
| FM-B-03 | Evaluator lifetime issues | LOW | Moderate | LOW |

### Approach C: Trait Observer

| FM ID | Failure Mode | Probability | Impact | Residual Risk |
|-------|--------------|-------------|--------|---------------|
| FM-C-01 | Trait dispatch overhead | CERTAIN | Minor | LOW |
| FM-C-02 | Boilerplate maintenance | HIGH | Moderate | MEDIUM |
| FM-C-03 | Async trait complexity | MEDIUM | Moderate | LOW-MEDIUM |

## Risk Summary

| Approach | Failure Modes | Highest Risk | Residual Risk | Confidence |
|----------|---------------|--------------|---------------|------------|
| A: Conditional | 3 | FM-A-02 (Major) | MEDIUM | HIGH |
| B: Optional | 3 | FM-B-03 (Moderate) | LOW | HIGH |
| C: Trait | 3 | FM-C-02 (Moderate) | LOW-MEDIUM | HIGH |

## Quality Characteristics

| Quality Attribute | Approach A | Approach B | Approach C |
|-------------------|------------|------------|------------|
| Maintainability | Medium | High | Medium |
| Test Coverage Achievable | 90% | 95% | 95% |
| Architecture Alignment | Good | Excellent | Good |
| Code Complexity | Medium | Low | Medium |
| Future Extensibility | Limited | Good | Excellent |

## Effort Comparison

| Effort Dimension | Approach A | Approach B | Approach C |
|------------------|------------|------------|------------|
| Implementation Hours | 18-22h | 18-25h | 22-28h |
| Test Hours | 6-8h | 5-7h | 7-9h |
| Documentation Hours | 2-3h | 2-3h | 3-4h |
| **Total** | **26-33h** | **25-35h** | **32-41h** |
| Confidence | HIGH (±25%) | HIGH (±25%) | MEDIUM (±30%) |

## Architecture Decision Record

**Status:** Approved
**Date:** 2025-12-13

**Context:**
PLAN031 requires non-invasive integration with existing import pipeline to record results for evaluation. Three integration approaches are viable.

**Decision:**
Recommend **Approach B: Optional Evaluator Pattern**

**Risk-Based Justification:**

Approach B has lowest residual risk (LOW) after mitigation:
- FM-B-01: Prevented by Rust type system (not a real failure mode)
- FM-B-02: Arc overhead negligible (<1ns per check)
- FM-B-03: Mitigated by standard Rust lifetime practices

Approach A has higher residual risk (MEDIUM):
- FM-A-02: Debug/release behavior differences can cause subtle bugs

Approach C has equivalent risk (LOW-MEDIUM) but:
- Higher implementation effort (+7h)
- Async trait complexity adds maintenance burden

**Quality Assessment:**
Approach B scores highest on maintainability and architecture alignment, matching existing WKMP patterns (Arc<T> used throughout).

**Effort Acknowledgment:**
All approaches have similar effort (~25-35h). Approach B is within the same range as A and lower than C.

Per CLAUDE.md Decision-Making Framework: Risk (primary) → Quality (secondary) → Effort (tertiary).

**Consequences:**
- Positive: Simple integration, follows existing patterns, easy to understand
- Negative: Runtime Option check (negligible overhead)
- Risks: None significant after analysis
