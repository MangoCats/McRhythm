# Technical Debt Analysis Index

**Analysis Date:** 2025-11-10  
**Full Report:** `TECHNICAL_DEBT_ANALYSIS.md` (604 lines, 21KB)

## Quick Navigation

### By Severity
- **Critical (2):** Parallel module hierarchies - `/wkmp-ai/src/extractors` vs `/wkmp-ai/src/fusion/extractors`
- **High (6):** Missing docs, excessive cloning, large files, test gaps, duplicate clients
- **Medium (17):** Organization, configuration, performance, patterns
- **Low (16):** Dead code, minor cleanup

### By Category

| Category | Issues | Files Affected | Priority |
|----------|--------|-----------------|----------|
| **Duplication** | 5 | extractors/, fusion/extractors/, services/ | CRITICAL |
| **Large Modules** | 10 | workflow_orchestrator.rs (2,253 LOC), ui.rs (1,308) | HIGH |
| **Missing Docs** | 15+ | config.rs, ffi/chromaprint.rs, services/ | HIGH |
| **Performance** | 3 | workflow_orchestrator.rs, file_tracker.rs | MEDIUM-HIGH |
| **Tests** | 26 files | api/, db/, models/, validators/ | MEDIUM-HIGH |
| **Architecture** | 4 | PLAN023/024 coexistence, error types | MEDIUM |
| **Naming** | 8 | Module naming, client naming, traits | LOW-MEDIUM |
| **Dead Code** | 6 | 4 allow(dead_code), 2 commented-out | LOW |

### Critical Files Requiring Attention

```
/wkmp-ai/src/
├── services/workflow_orchestrator.rs          [2,253 LOC] HIGHEST PRIORITY
├── api/ui.rs                                  [1,308 LOC] High priority (HTML extraction)
├── config.rs                                  [Missing docs for 4 functions]
├── extractors/                                [DUPLICATE MODULE HIERARCHY]
├── fusion/
│   ├── extractors/                            [DUPLICATE - should remove]
│   ├── fusers/                                [EMPTY - should remove]
│   ├── flavor_synthesizer.rs                  [561 LOC - complex]
│   └── metadata_fuser.rs                      [500 LOC - complex]
├── workflow/
│   ├── storage.rs                             [923 LOC - mixed concerns]
│   ├── pipeline.rs                            [605 LOC - boundary detection could extract]
│   └── boundary_detector.rs                   [304 LOC - silence detection]
└── validators/                                [Missing tests - 3 large files]
    ├── consistency_validator.rs               [658 LOC]
    ├── quality_scorer.rs                      [652 LOC]
    └── completeness_scorer.rs                 [575 LOC]
```

## Top 10 Action Items (by impact/effort ratio)

### Quick Wins (< 2 hours each)
1. Remove empty `/wkmp-ai/src/fusion/fusers/` directory
2. Add doc comments to `/wkmp-ai/src/config.rs` (4 functions)
3. Mark PLAN023 methods as `#[deprecated]`
4. Review/remove 4 `#[allow(dead_code)]` annotations
5. Remove 2 commented-out modules

### Medium Effort (2-6 hours each)
6. Profile and optimize clone() hot paths (workflow_orchestrator.rs:701-702)
7. Consolidate AcoustID client implementations (services → extractors)
8. Add documentation to FFI bindings (ffi/chromaprint.rs)
9. Create basic integration test framework for API/database
10. Document PLAN023 vs PLAN024 architecture decision

### Large Effort (> 6 hours)
- Refactor workflow_orchestrator.rs (split into 7 phase modules) [8-12 hours]
- Add comprehensive test suite [16-24 hours]
- Optimize data structure cloning [4-8 hours after profiling]

## Files to Investigate

### Potential Consolidation
```
AcoustID Implementations:
  /wkmp-ai/src/services/acoustid_client.rs (441 LOC)
  /wkmp-ai/src/extractors/acoustid_client.rs (468 LOC)
  /wkmp-ai/src/fusion/extractors/acoustid_client.rs

MusicBrainz Implementations:
  /wkmp-ai/src/services/musicbrainz_client.rs (245 LOC)
  /wkmp-ai/src/extractors/musicbrainz_client.rs (504 LOC)
  /wkmp-ai/src/fusion/extractors/musicbrainz_client.rs
```

### Tests to Add
```
Database DAOs:
  /wkmp-ai/src/db/*.rs - All lack unit tests (integration tests needed)

API Endpoints:
  /wkmp-ai/src/api/*.rs - No integration tests for HTTP endpoints

Validators:
  /wkmp-ai/src/validators/*.rs - Partial coverage; add property-based tests

FFI Bindings:
  /wkmp-ai/src/ffi/chromaprint.rs - External dependency; add wrapper tests
```

## Code Metrics Summary

| Metric | Value | Status |
|--------|-------|--------|
| Total Files | 100 | - |
| Total LOC | 33,895 | - |
| Average File Size | 339 LOC | Good |
| Largest File | workflow_orchestrator.rs (2,253) | Refactor needed |
| Test Coverage | 67.5% | Adequate |
| Documented APIs | 85% | Good |
| Clone() Calls | 141 (wkmp-ai) | Optimization needed |
| No/Minimal Docs | 15+ functions | Add immediately |

## Effort Estimates

| Phase | Duration | Items |
|-------|----------|-------|
| Phase 1 (Critical) | 1-2 weeks | Documentation, duplicate decisions, profiling |
| Phase 2 (High) | 2-4 weeks | Refactoring, consolidation, tests |
| Phase 3 (Medium) | 4-8 weeks | Code cleanup, optimization, architecture docs |
| Phase 4 (Low) | Ongoing | Minor improvements, logging |
| **TOTAL** | **8-14 weeks** | Full remediation |

## How to Use This Report

1. **For Immediate Action:** Read sections 1-2 of main report (CRITICAL + HIGH priority)
2. **For Planning:** Review "Recommended Priority Order" section
3. **For Detailed Implementation:** Check file-specific recommendations in main report
4. **For Code Review:** Use line number references when examining files

## Key Decision Points

### Must Decide (Blocking other work):
- **PLAN023 vs PLAN024:** Which architecture is authoritative? When to sunset PLAN023?
- **Extractor Location:** Should PLAN024 extractors be in `/extractors/` or `/fusion/extractors/`?
- **Client Consolidation:** Which AcoustID/MusicBrainz implementation is the standard?

### Should Document (But not blocking):
- Clone() optimization strategy (Arc vs Cow vs references)
- Testing strategy for database/API layers
- Configuration management best practices

## References

- **Full Report:** TECHNICAL_DEBT_ANALYSIS.md
- **Project Charter:** docs/PCH001_project_charter.md
- **Architecture:** docs/SPEC001-architecture.md
- **Implementation Plan:** docs/EXEC001-implementation_order.md

---

**Generated:** 2025-11-10  
**Status:** Analysis complete, ready for remediation prioritization
