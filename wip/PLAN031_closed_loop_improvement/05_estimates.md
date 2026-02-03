# PLAN031 Effort Estimates

## Per-Increment Estimates

| Increment | Low | Expected | High | Confidence | Basis |
|-----------|-----|----------|------|------------|-------|
| I1: Foundation | 1.5h | 2h | 3h | HIGH (±25%) | Standard module setup |
| I2: Ground Truth | 2h | 2.5h | 3.5h | HIGH (±25%) | Similar to recording_cache |
| I3: Evaluation | 2.5h | 3.5h | 5h | HIGH (±30%) | Core logic, well-defined |
| I4: Batch Orchestrator | 3h | 3.5h | 5h | MEDIUM (±35%) | Async complexity |
| I5: Pipeline Hooks | 1.5h | 2.5h | 3.5h | HIGH (±25%) | Minimal changes |
| I6: Failure Analyzer | 2.5h | 3.5h | 5h | MEDIUM (±35%) | Pattern detection logic |
| I7: DB Reset | 1.5h | 2h | 2.5h | HIGH (±20%) | Straightforward SQL |
| I8: Reporting/CLI | 2.5h | 3.5h | 5h | MEDIUM (±35%) | Formatting, clap |
| I9: Integration Tests | 2h | 2.5h | 4h | MEDIUM (±40%) | Test fixture setup |

## Aggregate Estimation

| Scenario | Hours | Probability |
|----------|-------|-------------|
| Optimistic (all goes well) | 19h | 15% |
| Expected (normal issues) | 26h | 55% |
| Pessimistic (significant issues) | 36h | 25% |
| Worst Case (major problems) | 45h | 5% |

**Recommended Buffer:** 25% contingency on expected
**Planning Estimate:** 26h + 6.5h buffer = **32.5h**

## Risk-Adjusted Estimation

| Risk | Probability | Impact if Occurs | Expected Impact |
|------|-------------|------------------|-----------------|
| R-001: Ground truth data quality | 30% | +5h debugging | +1.5h |
| R-002: Async testing complexity | 20% | +4h | +0.8h |
| R-003: Pipeline hook side effects | 15% | +6h debugging | +0.9h |

**Total Risk Adjustment:** +3.2h
**Risk-Adjusted Estimate:** 32.5h + 3.2h = **35.7h (~36h)**

## Resource Requirements

- Developer time: 36h (~1 week FTE)
- Test audio files: 50+ with known MBIDs
- Environment: Rust stable, SQLite, MusicBrainz API access
- Review time: ~4h
