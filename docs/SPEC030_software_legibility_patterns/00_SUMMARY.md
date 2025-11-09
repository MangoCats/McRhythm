# Software Legibility Patterns for WKMP Modules - Summary

**Document ID:** SPEC030
**Status:** DRAFT
**Author:** Development Team
**Created:** 2025-11-09
**Last Updated:** 2025-11-09

---

## Executive Summary

This specification defines architectural patterns for achieving **software legibility** in WKMP microservices—a direct correspondence between code structure and observable runtime behavior. Based on MIT research (Meng & Jackson, 2024), legible architecture enables:

- **Incrementality:** Add features via localized changes without system-wide refactoring
- **Integrity:** Preserve existing functionality when extending systems
- **Transparency:** Trace behavioral changes to specific code modifications

### Key Patterns

**Concepts:** Self-contained functional units with private state and well-defined actions
- No direct inter-concept dependencies
- Complete behavioral protocols
- URI-based global identifiers

**Synchronizations:** Declarative event-driven orchestration between concepts
- Three-clause structure: WHEN → WHERE → THEN
- Free/bound variable binding from events and queries
- Named provenance for traceability

**Action Traces:** Causal chains of operations with full provenance tracking
- Flow tokens group related actions
- Provenance edges record synchronization-mediated causality
- Queryable history for debugging and auditing

**Visible Developer Interface:** HTTP-based runtime introspection
- Dashboard, concept inspector, sync monitor, trace viewer, live events
- Real-time visualization of module structure and activity
- Development builds only (`:port/dev/`)

### Target Application

All WKMP microservices: wkmp-ap, wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le, wkmp-dr

---

## Document Structure

This specification is organized as a modular document with the following sections:

### Core Architecture

- **[01_legibility_principles.md](01_legibility_principles.md)** - Definition, three pillars (incrementality, integrity, transparency), comparison to traditional architecture
- **[02_core_patterns.md](02_core_patterns.md)** - Concepts, synchronizations, action traces structural patterns
- **[03_concept_guidelines.md](03_concept_guidelines.md)** - Identifying concepts, API design, independence rules, URI scheme

### Implementation Patterns

- **[04_synchronization_patterns.md](04_synchronization_patterns.md)** - Event-driven orchestration, catalog for WKMP, implementation patterns, variable binding
- **[05_action_traces.md](05_action_traces.md)** - Trace recording infrastructure, querying, persistent storage, logging integration

### Developer Tools

- **[06_dev_interface.md](06_dev_interface.md)** - Visible developer interface specification (dashboard, concept inspector, sync monitor, trace viewer, event stream)

### Application & Migration

- **[07_wkmp_examples.md](07_wkmp_examples.md)** - Module-specific concept/sync examples (wkmp-ap, wkmp-pd, wkmp-ui)
- **[08_migration_strategy.md](08_migration_strategy.md)** - Incremental adoption, module rollout, backward compatibility, testing

### Reference Materials

- **[09_references_templates.md](09_references_templates.md)** - Citations, concept template, synchronization template

---

## Problem Statement

Modern software suffers from **illegible** design lacking direct correspondence between code and observed behavior:

**Scattered Implementation:** Single features spread across multiple files/modules
**Hidden Dependencies:** Complex interdependencies obscured in code
**Unpredictable Changes:** Difficulty tracing desired behaviors to required code modifications

**LLM Challenges:**
- Incremental patches undermine previous ones
- Maintaining coherence across changes difficult
- "Vibe coding" produces unpredictable results

---

## Solution Approach

### Concept-Based Architecture

Replace technical layering (MVC) with **functional purpose boundaries:**

```
Traditional:                  Legible:
┌─────────────┐              ┌─────────────┐
│  Controller │              │ AudioPlayer │
│    Layer    │              │  Concept    │
├─────────────┤              ├─────────────┤
│   Model     │    vs.       │ Cooldown    │
│    Layer    │              │  Concept    │
├─────────────┤              ├─────────────┤
│  Database   │              │  Timeslot   │
│    Layer    │              │  Concept    │
└─────────────┘              └─────────────┘
     ↑                              ↑
  Cross-layer                   Independent,
  dependencies                  orchestrated
                                by syncs
```

### Event-Driven Orchestration

Replace direct calls with **declarative synchronizations:**

```rust
// Illegible (direct coupling):
let passage = selection.select_next().await?;
player.play(passage).await?;

// Legible (sync-mediated):
Synchronization {
    when: Event::QueueLow,
    where: Query::AutoPlayEnabled,
    then: vec![
        Action::Invoke("PassageSelection", "select_next"),
        Action::Invoke("AudioPlayer", "play"),
    ]
}
```

### Observable Provenance

Replace scattered logs with **structured action traces:**

```
[Web::request] ──(AutoSelect)──> [Timeslot::get_target]
                                         │
                                         ├──> [Selection::select]
                                         │
                                         └──> [Player::play]
```

Every action records:
- Inputs/outputs
- Flow token (groups related actions)
- Producing synchronization (provenance)

---

## Benefits

### For Developers

**Localized Changes:** Add features by creating new concepts/syncs, not refactoring system
**Clear Boundaries:** Concept independence prevents accidental coupling
**Traceable Behavior:** Action traces show exactly what happened and why
**Visual Debugging:** Developer interface exposes runtime state without instrumentation

### For AI-Assisted Development

**LLM-Compatible:** Declarative synchronizations match LLM code generation patterns
**Incremental Safety:** Explicit contracts prevent patches from undermining prior work
**Verifiable:** Synchronization rules enable formal analysis of interactions

### For Operations

**Audit Trail:** Complete provenance for security-sensitive operations
**Production Debugging:** Replay action sequences to diagnose issues
**Performance Analysis:** Synchronization timing metrics identify bottlenecks

---

## Architectural Alignment with WKMP

### Microservices Compatibility

Legible patterns apply **within each microservice:**
- Each module (wkmp-ap, wkmp-pd, etc.) contains concepts
- Synchronizations orchestrate intra-module concept interactions
- Inter-module communication remains HTTP/SSE (unchanged)

### Zero-Configuration Startup

Developer interface integrates with existing infrastructure:
- Same port as main API (`:port/dev/`)
- Development builds only (`#[cfg(debug_assertions)]`)
- No additional configuration required

### Documentation Hierarchy

SPEC030 sits in Tier 2 (Design):
- Implements requirements from REQ001 (Tier 1)
- Guides implementation in IMPL00X (Tier 3)
- Informs execution plans in EXEC001 (Tier 4)

---

## Implementation Roadmap

### Phase 1: Concept Identification (Weeks 1-2)
Audit existing code to identify implicit concepts per module

### Phase 2: Concept Extraction (Weeks 3-6)
Refactor mixed concerns into separate concept modules

### Phase 3: Synchronization Engine (Weeks 7-10)
Build event bus and sync engine infrastructure

### Phase 4: Action Tracing (Weeks 11-14)
Add tracer and persistent storage

### Phase 5: Developer Interface (Weeks 15-18)
Build HTTP-based introspection UI

### Module Rollout Priority

1. **wkmp-ap** (Audio Player) - Cleanest boundaries, high tracing value
2. **wkmp-pd** (Program Director) - Clear concepts (selection, cooldown, timeslot)
3. **wkmp-ui** (User Interface) - Benefits from explicit sync (auth, SSE)
4. **wkmp-ai** (Audio Ingest) - Complex workflow, high legibility value
5. **wkmp-le** (Lyric Editor) - Lower priority (specialized tool)
6. **wkmp-dr** (Database Review) - Lowest priority (read-only)

---

## Quick Reference

### Concept Design Checklist

- [ ] User-facing functionality with clear purpose
- [ ] Independent state (no concept-to-concept dependencies)
- [ ] Complete behavioral protocol (actions + queries)
- [ ] Reusable across different domains
- [ ] URI-based naming for global traceability

### Synchronization Design Checklist

- [ ] Event pattern clearly defined (WHEN)
- [ ] State conditions use queries only (WHERE)
- [ ] Actions invoke concepts via well-defined APIs (THEN)
- [ ] Named for provenance tracking
- [ ] Testable in isolation

### Action Trace Verification

- [ ] Flow tokens group causally-related actions
- [ ] Provenance edges record producing synchronizations
- [ ] Inputs/outputs captured for all actions
- [ ] Queryable: "What caused this?" "Which sync produced this?"

### Developer Interface Requirements

- [ ] Dashboard shows module status and recent activity
- [ ] Concept inspector exposes current state
- [ ] Sync monitor tracks rule activations
- [ ] Trace viewer visualizes provenance graphs
- [ ] Event stream provides real-time monitoring

---

## Navigation

**Next:** [01_legibility_principles.md](01_legibility_principles.md) - Core legibility concepts

**See Also:**
- [02_core_patterns.md](02_core_patterns.md) - Structural patterns (concepts, syncs, traces)
- [06_dev_interface.md](06_dev_interface.md) - Visible developer interface specification
- [07_wkmp_examples.md](07_wkmp_examples.md) - Module-specific examples
- [08_migration_strategy.md](08_migration_strategy.md) - Adoption roadmap

---

## Key Definitions

**Legibility:** Direct correspondence between code and observed behavior

**Concept:** Self-contained functional unit with private state and well-defined actions

**Synchronization:** Declarative rule orchestrating concept interactions (WHEN → WHERE → THEN)

**Action Trace:** Directed acyclic graph of causally-related operations with provenance

**Flow Token:** UUID grouping all actions triggered by single root event

**Provenance Edge:** Link between actions labeled with producing synchronization

---

## Document Metadata

**Total Lines:** ~1823 (across all sections)
**Format:** Modular document per GOV001
**Reading Time:** 45-60 minutes (full document)
**Summary Reading Time:** 5-10 minutes (this file)

**Context Window Optimization:**
- Start with this summary (<500 lines)
- Drill down to specific sections as needed
- Each section <300 lines for focused reading

---

**END OF SUMMARY**

