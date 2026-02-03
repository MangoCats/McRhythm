# Increment Index: am30 Integration

## Overview

| # | Name | Effort | Status | Prerequisites |
|---|------|--------|--------|---------------|
| 1 | [Pipeline Config Extension](increment_01.md) | 1-2h | Pending | None |
| 2 | [Single-Track Check Integration](increment_02.md) | 2-3h | Pending | Inc 1 |
| 3 | [AlbumMatcher Integration](increment_03.md) | 3-4h | Pending | Inc 2 |
| 4 | [Passage Conversion](increment_04.md) | 2-3h | Pending | Inc 3 |
| 5 | [Progress Events](increment_05.md) | 1-2h | Pending | Inc 4 |
| 6 | [Error Handling & Fallback](increment_06.md) | 2-3h | Pending | Inc 4 |
| 7 | [Integration Tests](increment_07.md) | 3-4h | Pending | Inc 6 |

## Dependency Graph

```
┌──────────────┐
│ Increment 1  │ Pipeline Config Extension
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ Increment 2  │ Single-Track Check Integration
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ Increment 3  │ AlbumMatcher Integration
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ Increment 4  │ Passage Conversion
└──────┬───────┘
       │
       ├─────────────────┐
       │                 │
       ▼                 ▼
┌──────────────┐  ┌──────────────┐
│ Increment 5  │  │ Increment 6  │
│ Progress     │  │ Error        │
│ Events       │  │ Handling     │
└──────────────┘  └──────┬───────┘
                         │
                         ▼
                  ┌──────────────┐
                  │ Increment 7  │ Integration Tests
                  └──────────────┘
```

## Critical Path

**I1 → I2 → I3 → I4 → I6 → I7** = 14-19 hours

Increments 5 (Progress Events) can run in parallel with Increment 6.

## Checkpoints

| After | Checkpoint Description |
|-------|------------------------|
| I4 | Core integration complete - verify album routing and passage creation |
| I7 | All tests pass - ready for review |

## Effort Summary

| Range | Hours | Confidence |
|-------|-------|------------|
| Optimistic | 14h | 10% |
| Expected | 18h | 60% |
| Pessimistic | 24h | 25% |

**Recommended Estimate:** 18-20 hours (HIGH confidence ±25%)
