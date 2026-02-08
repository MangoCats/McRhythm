# PLAN031 Increment Index

| # | Name | Effort | Requirements | Dependencies | Phase |
|---|------|--------|--------------|--------------|-------|
| 1 | Testing Module Foundation | 2-3h | - | None | Foundation |
| 2 | Ground Truth Schema & Store | 2-3h | GT-010, GT-020 | I1 | Foundation |
| 3 | Evaluation Engine | 3-4h | EV-010, EV-020, EV-030 | I2 | Core |
| 4 | Batch Orchestrator | 3-4h | BM-010 | I3 | Core |
| 5 | Pipeline Integration Hooks | 2-3h | API-010 | I3 | Integration |
| 6 | Failure Analyzer | 3-4h | FA-010, FA-020, FA-030 | I3 | Analysis |
| 7 | Database Reset | 2h | DB-010 | I2 | Utility |
| 8 | Reporting & CLI | 3-4h | RP-010, RP-020, BM-020 | I4, I6 | Interface |
| 9 | Integration Testing | 2-3h | All | I1-I8 | Testing |

**Total Estimated Effort:** 22-30 hours

## Dependency Graph

```
I1 (Foundation)
 ├──> I2 (Ground Truth)
 │     ├──> I3 (Evaluation)
 │     │     ├──> I4 (Batch Orchestrator)
 │     │     ├──> I5 (Pipeline Hooks)
 │     │     └──> I6 (Failure Analyzer)
 │     └──> I7 (DB Reset)
 └──────────────────────────> I8 (Reporting/CLI)
                               └──> I9 (Integration Tests)
```

## Critical Path

**I1 → I2 → I3 → I4 → I8 → I9** (15-21 hours)

## Parallelization Opportunities

After I3 completes:
- I4 (Batch Orchestrator) and I5 (Pipeline Hooks) can run in parallel
- I6 (Failure Analyzer) and I7 (DB Reset) can run in parallel
