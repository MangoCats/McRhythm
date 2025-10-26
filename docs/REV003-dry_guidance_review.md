# DRY Guidance Review & Enhancement

**🔍 TIER R - REVIEW & CHANGE CONTROL**

**Document Type:** Design Review (Immutable Snapshot)
**Review Date:** 2025-10-26
**Status:** Complete (Historical Record)
**Reviewer:** Technical Lead (Claude)
**Scope:** Analysis of DRY guidance completeness in IMPL002 and GUIDE002

---

## Executive Summary

Reviewed IMPL002 (Coding Conventions) and GUIDE002 (wkmp-ap Implementation Guide) to ensure DRY (Don't Repeat Yourself) guidance is sufficient for preventing code duplication across WKMP's 5 microservices.

**Findings:**
- ✅ Basic DRY principles exist (CO-070-073)
- ⚠️ **Critical Gap:** CO-007 was a static list, not decision criteria
- ⚠️ **Missing:** No workflow guidance for checking wkmp-common before implementing
- ⚠️ **Missing:** No consolidation triggers (when to move code from module to common)

**Resolution:** Enhanced both documents with actionable decision criteria, workflow guidance, and cross-references to DRY-STRATEGY.md.

---

## Analysis: What Existed

### IMPL002 - Coding Conventions (Before)

**CO-007 (Original):**
```
Shared code shall be implemented in the common/ library:
- Database models and queries
- Event types (WkmpEvent enum)
- API request/response types
- Flavor calculation algorithms
- Cooldown calculation logic
- UUID and timestamp utilities
- Module configuration loading
```

**Problems:**
1. **Static list** - Doesn't explain WHY these go in common
2. **Incomplete** - Missing EventBus, authentication, config resolution (patterns we discovered!)
3. **No decision criteria** - Developer must guess what belongs in common
4. **Becomes outdated** - New patterns emerge but aren't added to list

**CO-070-073 (General DRY):**
- CO-071: Duplicated code blocks (>5 lines) appearing >2 times → extract
- CO-072: Similar patterns with variations → parameterize
- CO-073: Magic numbers/strings → named constants

**Problem:** Addresses within-module duplication but NOT cross-module duplication

### GUIDE002 - wkmp-ap Implementation (Before)

**Mentions wkmp-common but provides no guidance:**
- "wkmp-common library provides shared types (Event, entities, etc.)"
- "Must maintain compatibility with existing wkmp-common Event types"

**Problems:**
1. **Passive reference** - Assumes developer knows what's in wkmp-common
2. **No workflow** - Doesn't say "check wkmp-common first"
3. **No DRY reminder** - Doesn't reinforce CO-007 decision criteria

---

## Real-World Impact

### Patterns NOT Captured by Original CO-007

From our Phase 1-2 DRY consolidation work, we discovered these patterns that would NOT have been caught by the original static list:

| Pattern | Lines Saved | In Original CO-007? |
|---------|-------------|---------------------|
| EventBus infrastructure | ~800 | ❌ No |
| Enhanced WkmpEvent variants | ~800 | ✅ Yes ("Event types") but incomplete |
| RootFolderResolver | ~400 | ❌ No |
| API Authentication | ~1,600 | ❌ No |
| **TOTAL** | **~3,600** | **Only ~800 would have been caught** |

**Key Insight:** Static lists miss ~78% of DRY opportunities because they don't provide decision-making criteria!

---

## Enhancements Made

### IMPL002 - Enhanced CO-007

#### CO-007: Decision Criteria (NEW)

Added 5 criteria - code belongs in wkmp-common if it meets ANY of:

1. **Cross-Module Communication** - Used for communication between modules (Events, API types)
2. **Identical Implementation** - Same logic needed by 2+ modules with no variations
3. **Domain Model** - Core business entities used across modules (Passage, Song, Recording)
4. **Infrastructure Pattern** - Repeated pattern across modules (EventBus, authentication, config)
5. **Security-Critical** - Authentication, authorization, cryptography (must be identical)

**Impact:** Developer can now DECIDE what goes in common, not just follow a static list.

#### CO-007A: Implementation Workflow (NEW)

Before implementing new functionality, developers shall:
1. Check `wkmp-common/src/` for existing implementations
2. Review [DRY-STRATEGY.md](DRY-STRATEGY.md) decision matrix
3. If similar logic exists in another module, consolidate to `wkmp-common` first

**Impact:** Proactive DRY enforcement - check before you code!

#### CO-007B: Consolidation Triggers (NEW)

Cross-module code duplication triggers:
- Same logic appears in 2+ modules → Consolidate immediately
- Similar pattern with minor variations → Parameterize and consolidate
- Module-specific today but planned for other modules → Proactively move to common

**Impact:** Clear thresholds for when to consolidate.

#### CO-007C: Consolidation Process (NEW)

When consolidating code to `wkmp-common`:
1. Ensure existing module tests still pass
2. Add comprehensive tests to `wkmp-common`
3. Update [DRY-STRATEGY.md](DRY-STRATEGY.md) with the new shared component
4. Document in module's re-export (e.g., `pub use wkmp_common::events::EventBus;`)

**Impact:** Quality gate for consolidation work.

### GUIDE002 - Enhanced Implementation Guidance

#### Assumptions Section (Updated)

**Before:**
```
2. wkmp-common library provides shared types (Event, entities, etc.)
```

**After:**
```
2. wkmp-common library provides shared infrastructure:
   - EventBus and WkmpEvent enum (see DRY-STRATEGY.md)
   - API authentication (timestamp/hash validation)
   - Configuration loading (RootFolderResolver, platform defaults)
   - Database models and queries
   - Fade curve definitions
```

**Impact:** Developer knows exactly what's available in wkmp-common.

#### Constraints Section (Updated)

Added:
- Must use wkmp-common shared infrastructure (per CO-007, see DRY-STRATEGY.md)
- Must follow IMPL002 Rust coding conventions (including CO-007A: check wkmp-common first)
- Before implementing new patterns, check if wkmp-common provides the infrastructure

**Impact:** DRY is now a constraint, not just a suggestion.

#### Phase 1 Components (Updated)

**Before:**
```
4. Event integration (events.rs) - EventBus subscription/emission
```

**After:**
```
4. Event integration (events.rs) - Re-export from wkmp_common::events (CO-007)
   - pub use wkmp_common::events::{EventBus, WkmpEvent, PlaybackState, ...};
   - Module-specific types only (PlaybackEvent, MixerStateContext)
```

**Impact:** Clear that we re-export, not re-implement.

---

## Validation: Coverage Analysis

### Test Cases - Would Enhanced Guidance Catch These?

| Scenario | Original CO-007 | Enhanced CO-007 |
|----------|----------------|-----------------|
| EventBus duplication across modules | ❌ Not in list | ✅ Criteria #4 (Infrastructure Pattern) |
| API authentication in wkmp-ap and wkmp-ui | ❌ Not in list | ✅ Criteria #5 (Security-Critical) |
| RootFolderResolver priority logic | ❌ Not in list | ✅ Criteria #4 (Infrastructure Pattern) |
| SSE bridge pattern (pending) | ❌ Not in list | ✅ Criteria #4 (Infrastructure Pattern) |
| CLI argument parsing (pending) | ❌ Not in list | ✅ Criteria #2 (Identical Implementation) |
| Module-specific HTTP routes | ✅ Correctly stays in module | ✅ Correctly stays in module |

**Coverage:** Enhanced criteria would catch 100% of consolidation opportunities from our Phase 1-2 work.

---

## Recommendation: Future Process

### For Developers Implementing New Modules

When implementing wkmp-ui, wkmp-pd, wkmp-ai, or wkmp-le:

1. **Before writing any code:**
   - Read [DRY-STRATEGY.md](DRY-STRATEGY.md) decision matrix
   - Review `wkmp-common/src/` directory structure
   - Check what wkmp-ap already uses from wkmp-common

2. **During implementation:**
   - Apply CO-007A workflow: check wkmp-common first
   - Use CO-007 decision criteria for new patterns
   - Apply CO-007B triggers when you see duplication

3. **After implementation:**
   - Follow CO-007C consolidation process
   - Update DRY-STRATEGY.md with new shared components

### For Code Reviews

Reviewers should verify:
- ✅ CO-007A workflow followed (checked wkmp-common first)
- ✅ No duplication of existing wkmp-common functionality
- ✅ If new pattern appears in 2+ modules, consolidated to wkmp-common
- ✅ DRY-STRATEGY.md updated with new shared components

---

## Benefits of Enhancements

### Quantitative

- **Prevented duplication:** Enhanced criteria would have caught 100% of Phase 1-2 patterns
- **Code savings:** ~3,600 lines from patterns not in original CO-007
- **Maintainability:** Single source of truth for 5 microservices

### Qualitative

- **Proactive vs. Reactive:** CO-007A workflow prevents duplication before it happens
- **Decision Support:** Criteria enable developers to make correct decisions
- **Living Document:** DRY-STRATEGY.md evolves with codebase
- **Consistency:** All 5 modules will use same infrastructure patterns

---

## Conclusion

**Original guidance was incomplete:**
- Static lists become outdated
- No decision-making support
- No workflow integration

**Enhanced guidance is actionable:**
- ✅ Decision criteria (CO-007: 5 criteria)
- ✅ Workflow integration (CO-007A: check wkmp-common first)
- ✅ Consolidation triggers (CO-007B: when to move code)
- ✅ Quality gates (CO-007C: consolidation process)
- ✅ Cross-references to DRY-STRATEGY.md (living documentation)

**Expected outcome:** Remaining 4 modules (wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le) will naturally adopt shared infrastructure, achieving ~4,250 total lines of code savings with minimal duplication risk.

---

## Document Cross-References

**Tier R (Review):**
- [REV003-dry_guidance_review.md](REV003-dry_guidance_review.md) - This document

**Tier 0 (Governance):**
- [GOV001-document_hierarchy.md](GOV001-document_hierarchy.md) - Document tier system

**Tier 3 (Implementation):**
- [IMPL002-coding_conventions.md](IMPL002-coding_conventions.md) - CO-007 through CO-007C (enhanced per this review)

**Tier 4 (Execution):**
- [GUIDE002-wkmp_ap_re_implementation_guide.md](GUIDE002-wkmp_ap_re_implementation_guide.md) - Enhanced Phase 1 guidance (updated per this review)

**Supporting Documentation:**
- [DRY-STRATEGY.md](DRY-STRATEGY.md) - Comprehensive DRY implementation catalog (living document)

---

**Review Status:** Complete (Historical Record)
**Enhancements Applied:** 2025-10-26 (see git history for implementation commits)
**Immutability:** This document is a historical snapshot and will not be updated. Subsequent reviews will create new REV documents.
