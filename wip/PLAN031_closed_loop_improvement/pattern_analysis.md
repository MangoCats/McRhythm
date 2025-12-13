# PLAN031 Architectural Pattern Analysis

## Applicable Patterns

### Structural Patterns

- [x] **Repository Pattern** - Ground truth store abstracts database access
- [x] **Facade Pattern** - TestOrchestrator provides simplified interface to complex subsystem
- [ ] Adapter Pattern - Not needed, all interfaces are internal
- [ ] Composite Pattern - Not applicable

### Behavioral Patterns

- [x] **Observer Pattern** - Pipeline hooks observe/record events (optional evaluator)
- [x] **Strategy Pattern** - Different evaluation strategies for single vs album
- [x] **State Pattern** - Phase progression (Phase 1 → 2 → 3) with different behaviors
- [ ] Command Pattern - Not needed, direct method calls suffice

### Concurrency Patterns (Rust/Tokio)

- [x] **Pipeline Pattern** - File processing flows through stages
- [x] **Fan-out** - Phase 3 parallel processing of multiple files
- [ ] Actor Pattern - Overkill for this use case
- [ ] Circuit Breaker - Not needed, already have rate limiting

### WKMP-Specific Patterns

- [x] **Database-first configuration** - Ground truth stored in DB
- [x] **Zero-config startup** - Test mode enabled via CLI flag
- [x] **Event recording** - Record events at pipeline hooks
- [ ] SSE broadcast - Not needed, CLI only

## Anti-Patterns to Avoid

### Code Smells

- **God Object** - Keep TestOrchestrator focused, delegate to EvaluationEngine, FailureAnalyzer
- **Feature Envy** - Don't reach into existing pipeline internals; use hooks only
- **Magic Numbers** - Define constants for thresholds (FAILURE_THRESHOLD, ACCURACY_TARGET)

### Concurrency Anti-Patterns

- **Blocking in async** - Use `tokio::spawn_blocking` for file hashing
- **Unbounded queues** - Limit parallel workers to avoid memory pressure
- **Missing cancellation** - Respect timeout_secs configuration

### Architecture Anti-Patterns

- **Circular dependencies** - testing/ depends on services/, not vice versa
- **Leaky abstractions** - Pipeline hooks should be simple trait calls
- **Over-engineering** - Start with simple pattern detection, add ML later if needed

## Pattern Recommendations by Hotspot

### Hotspot: Ground Truth Management
**Recommended Pattern:** Repository Pattern
**Rationale:** Isolates database access, enables future caching, simplifies testing
**Confidence:** HIGH - Standard pattern for data access

### Hotspot: Phase Progression
**Recommended Pattern:** State Pattern (lightweight)
**Rationale:** Different batch behaviors per phase, clear state transitions
**Confidence:** HIGH - Clean separation of phase-specific logic

### Hotspot: Failure Pattern Detection
**Recommended Pattern:** Strategy + Aggregation
**Rationale:** Different pattern detectors for different failure types, combine results
**Confidence:** MEDIUM - Pattern detection complexity may evolve

### Hotspot: Pipeline Integration
**Recommended Pattern:** Optional Observer (trait object)
**Rationale:** Non-invasive, services check for evaluator before calling
**Confidence:** HIGH - Minimal impact on existing code
