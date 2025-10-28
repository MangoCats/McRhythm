# Architectural Options - Risk-Based Comparisons

**Navigation:** [← Back to Summary](00_SUMMARY.md) | [Next: Recommendations →](06_recommendations.md)

---

## Decision Framework

All comparisons use **CLAUDE.md Risk-First Decision Framework:**

**Priority Order:**
1. **Risk Assessment** (PRIMARY) - Lowest residual risk wins
2. **Quality Characteristics** (SECONDARY) - Tiebreaker when risks equivalent
3. **Implementation Effort** (TERTIARY) - Acknowledged but not a decision factor

**Risk Categories:**
- Low = Low (equivalent)
- Low-Medium = Low-Medium (equivalent)
- Low ≠ Low-Medium (NOT equivalent - choose Low)

---

## DECISION 1: Amplitude Analysis Method

### APPROACH A: RMS with A-Weighting

**Risk Assessment:**
- **Failure Risk:** Low
- **Failure Modes:**
  1. RMS doesn't perfectly match human perception - Probability: High - Impact: Low
  2. A-weighting implementation bugs - Probability: Low - Impact: Low
- **Mitigation:**
  - Use proven `dasp` or `audio-processor-analysis` crate
  - User-adjustable thresholds compensate
- **Residual Risk:** Low

**Quality:**
- Maintainability: High
- Test Coverage: High
- Architectural Alignment: Strong

**Effort:** Medium (2-3 days)

### APPROACH B: EBU R128 LUFS

**Risk Assessment:**
- **Failure Risk:** Low-Medium
- **Failure Modes:**
  1. Algorithm complexity introduces bugs - Probability: Medium - Impact: Medium
  2. Library limited maintenance - Probability: Medium - Impact: Low
  3. Integrated loudness requires full passage analysis - Probability: High - Impact: Medium
- **Mitigation:**
  - Extensive testing against references
  - Fallback to RMS if NaN
- **Residual Risk:** Low-Medium

**Quality:**
- Maintainability: Medium
- Test Coverage: Medium
- Architectural Alignment: Moderate

**Effort:** Medium-High (3-5 days)

### RANKING:
1. **Approach A (RMS)** - Lowest residual risk (Low)
2. Approach B (EBU R128) - Higher residual risk (Low-Medium)

### RECOMMENDATION:
**Choose Approach A (RMS with A-weighting)**
- Lowest failure risk
- Adequate accuracy with user-adjustable thresholds
- Simpler implementation and maintenance

---

## DECISION 2: Parameter Storage Architecture

### APPROACH A: Dedicated `import_parameters` Table

**Risk Assessment:**
- **Failure Risk:** Medium
- **Failure Modes:**
  1. Schema rigidity - Probability: Medium - Impact: High
  2. Complex hierarchy queries - Probability: High - Impact: Medium
  3. Migration required per parameter - Probability: High - Impact: Medium
- **Mitigation:** Well-designed schema, ORM layer
- **Residual Risk:** Low-Medium

**Quality:**
- Maintainability: Medium
- Test Coverage: High
- Architectural Alignment: Strong

**Effort:** Medium (3-4 days)

### APPROACH B: JSON Blob in `settings`

**Risk Assessment:**
- **Failure Risk:** Medium
- **Failure Modes:**
  1. JSON validation bugs - Probability: Medium - Impact: High
  2. No type safety - Probability: High - Impact: Medium
  3. Difficult to query - Probability: Medium - Impact: Low
- **Mitigation:** Strict validation, Rust struct deserialization
- **Residual Risk:** Medium

**Quality:**
- Maintainability: Low
- Test Coverage: Medium
- Architectural Alignment: Moderate

**Effort:** Low (1-2 days)

### APPROACH C: Hybrid (Global JSON + Per-Passage Overrides)

**Risk Assessment:**
- **Failure Risk:** Low
- **Failure Modes:**
  1. Global/per-passage inconsistency - Probability: Low - Impact: Low
  2. JSON validation bugs - Probability: Low - Impact: Medium
- **Mitigation:** Clear precedence rules, Rust `serde` validation
- **Residual Risk:** Low

**Quality:**
- Maintainability: High
- Test Coverage: High
- Architectural Alignment: Strong (matches `musical_flavor_vector` pattern)

**Effort:** Medium (2-3 days)

### RANKING:
1. **Approach C (Hybrid)** - Lowest residual risk (Low)
2. Approach A (Dedicated Table) - Medium residual risk (Low-Medium)
3. Approach B (JSON-only) - Highest residual risk (Medium)

### RECOMMENDATION:
**Choose Approach C (Hybrid)**
- Lowest failure risk
- Matches existing WKMP patterns
- Extensible without migrations

---

## DECISION 3: Essentia Integration

### APPROACH A: Subprocess (Call Binary)

**Risk Assessment:**
- **Failure Risk:** Low
- **Failure Modes:**
  1. Binary not installed - Probability: Medium - Impact: Medium
  2. Subprocess spawn failures - Probability: Low - Impact: Medium
  3. JSON parsing errors - Probability: Low - Impact: Low
- **Mitigation:**
  - Check binary at startup
  - Graceful degradation
  - Include in Full version packaging
- **Residual Risk:** Low

**Quality:**
- Maintainability: High
- Test Coverage: High
- Architectural Alignment: Strong

**Effort:** Low (2-3 days)

### APPROACH B: FFI (Foreign Function Interface)

**Risk Assessment:**
- **Failure Risk:** High
- **Failure Modes:**
  1. Memory safety violations - Probability: Medium - Impact: Critical
  2. Complex build system - Probability: High - Impact: High
  3. Platform-specific issues - Probability: High - Impact: High
- **Mitigation:** Extensive testing, isolated FFI module
- **Residual Risk:** Medium-High

**Quality:**
- Maintainability: Low
- Test Coverage: Low
- Architectural Alignment: Weak

**Effort:** High (1-2 weeks)

### APPROACH C: Python Microservice

**Risk Assessment:**
- **Failure Risk:** Medium
- **Failure Modes:**
  1. Python environment setup - Probability: High - Impact: Medium
  2. HTTP overhead - Probability: Low - Impact: Low
  3. Additional service maintenance - Probability: High - Impact: Medium
- **Mitigation:** Package Python + Essentia, HTTP API
- **Residual Risk:** Medium

**Quality:**
- Maintainability: Medium
- Test Coverage: High
- Architectural Alignment: Strong

**Effort:** Medium-High (1 week)

### RANKING:
1. **Approach A (Subprocess)** - Lowest residual risk (Low)
2. Approach C (Python Microservice) - Medium residual risk (Medium)
3. Approach B (FFI) - Highest residual risk (Medium-High)

### RECOMMENDATION:
**Choose Approach A (Subprocess)**
- Lowest failure risk
- Simplest implementation
- Clean separation

---

## DECISION 4: Import Workflow Model

### APPROACH A: Synchronous (HTTP Blocks)

**Risk Assessment:**
- **Failure Risk:** High
- **Failure Modes:**
  1. HTTP timeout for large libraries - Probability: High - Impact: Critical
  2. Can't cancel in-progress - Probability: High - Impact: High
  3. Server unresponsive - Probability: High - Impact: High
- **Mitigation:** Increase timeout (not practical)
- **Residual Risk:** High

**Quality:**
- Maintainability: High
- Test Coverage: High
- Architectural Alignment: Weak

**Effort:** Low (1-2 days)

### APPROACH B: Asynchronous (Background Jobs + Polling)

**Risk Assessment:**
- **Failure Risk:** Low
- **Failure Modes:**
  1. Job state management bugs - Probability: Low - Impact: Medium
  2. Polling overhead - Probability: Low - Impact: Low
- **Mitigation:** Persistent job state, idempotent operations
- **Residual Risk:** Low

**Quality:**
- Maintainability: Medium
- Test Coverage: High
- Architectural Alignment: Strong

**Effort:** Medium (3-4 days)

### APPROACH C: Hybrid (Async + SSE)

**Risk Assessment:**
- **Failure Risk:** Low
- **Failure Modes:**
  1. SSE connection drops - Probability: Medium - Impact: Low
  2. SSE + polling complexity - Probability: Low - Impact: Low
- **Mitigation:** SSE reconnection, polling fallback
- **Residual Risk:** Low

**Quality:**
- Maintainability: Medium
- Test Coverage: High
- Architectural Alignment: Strong (WKMP already uses SSE)

**Effort:** Medium-High (4-5 days)

### RANKING:
1. **Approach C (Hybrid SSE)** - Lowest residual risk (Low), best UX
2. **Approach B (Async Background)** - Equal residual risk (Low), simpler
3. Approach A (Synchronous) - Highest residual risk (High)

### RECOMMENDATION:
**Choose Approach C (Hybrid with SSE)**
- Equal risk to Approach B
- Better user experience
- Matches WKMP SSE architecture

---

## DECISION 5: UI Complexity Level

### APPROACH A: Simple Wizard (Auto-Import)

**Risk Assessment:**
- **Failure Risk:** Medium
- **Failure Modes:**
  1. Auto-detection fails for certain styles - Probability: High - Impact: High
  2. Users frustrated by lack of control - Probability: Medium - Impact: Medium
- **Mitigation:** Excellent defaults, "Advanced mode" button
- **Residual Risk:** Medium

**Quality:**
- Maintainability: High
- Test Coverage: High
- Architectural Alignment: Strong

**Effort:** Low (2-3 days)

### APPROACH B: Advanced Parameter Tuning (Immediate)

**Risk Assessment:**
- **Failure Risk:** Medium
- **Failure Modes:**
  1. Users overwhelmed - Probability: High - Impact: Medium
  2. Users misconfigure - Probability: Medium - Impact: High
- **Mitigation:** Tooltips, documentation, "Reset to defaults"
- **Residual Risk:** Medium

**Quality:**
- Maintainability: Medium
- Test Coverage: Medium
- Architectural Alignment: Moderate

**Effort:** High (1 week)

### APPROACH C: Progressive Disclosure (Simple → Advanced)

**Risk Assessment:**
- **Failure Risk:** Low
- **Failure Modes:**
  1. UI state management bugs - Probability: Low - Impact: Low
- **Mitigation:** Clear separation of modes, comprehensive testing
- **Residual Risk:** Low

**Quality:**
- Maintainability: Medium
- Test Coverage: High
- Architectural Alignment: Strong

**Effort:** Medium (5-7 days)

### RANKING:
1. **Approach C (Progressive Disclosure)** - Lowest residual risk (Low)
2. Approach A (Simple Wizard) - Medium residual risk (Medium)
3. Approach B (Advanced Immediate) - Medium residual risk (Medium)

### RECOMMENDATION:
**Choose Approach C (Progressive Disclosure)**
- Lowest failure risk
- Matches WKMP philosophy (automatic-first, manual-available)
- Best balance beginner-friendly + power-user

---

## Consolidated Recommendations

| Decision | Recommended Approach | Risk Level | Rationale |
|----------|---------------------|------------|-----------|
| 1. Amplitude Analysis | RMS with A-weighting | Low | Simplest, adequate accuracy |
| 2. Parameter Storage | Hybrid JSON | Low | Matches existing patterns |
| 3. Essentia Integration | Subprocess | Low | Cleanest separation |
| 4. Workflow Model | Async + SSE | Low | Best UX, proven architecture |
| 5. UI Complexity | Progressive Disclosure | Low | Beginner + power-user |

**All choices prioritize:**
- Lowest failure risk (primary criterion)
- Architectural alignment with existing WKMP
- Maintainability over initial speed
- User experience (automatic + manual control)

---

**Navigation:** [← Back to Summary](00_SUMMARY.md) | [Next: Recommendations →](06_recommendations.md)
