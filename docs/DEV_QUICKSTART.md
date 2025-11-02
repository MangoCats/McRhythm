# Developer Quick Start Guide

**Audience:** New developers joining the WKMP project
**Time to Complete:** 15-30 minutes
**Last Updated:** 2025-11-02

---

## Welcome to WKMP!

This guide gets you productive quickly. Read these 5 essential documents in order, then reference others as needed.

---

## Step 1: Understand the Project (5 min)

### Required Reading

**[PCH001: Project Charter](PCH001_project_charter.md)** (MUST READ - 10 min)
- What WKMP is and why it exists
- Quality-absolute goals (flawless audio, 1970s FM radio feel)
- Risk-first decision framework

**Key Takeaway:** All design decisions prioritize risk reduction over implementation speed.

---

## Step 2: Learn the Documentation System (5 min)

### Required Reading

**[GOV001: Document Hierarchy](GOV001-document_hierarchy.md)** (MUST READ - 5 min)
- 5-tier documentation framework
- How requirements flow into design and implementation
- Information flow rules (downward normal, upward controlled)

**Quick Reference:**
- Tier 0: Governance (how we document)
- Tier 1: Requirements (WHAT the system must do - REQ###)
- Tier 2: Design (HOW requirements are satisfied - SPEC###)
- Tier 3: Implementation (concrete specs - IMPL###)
- Tier 4: Execution (WHEN features are built - EXEC###)

---

## Step 3: Understand the System (10 min)

### Required Reading

**[REQ001: Requirements](REQ001-requirements.md)** (MUST READ - 15 min, read sections as needed)
- Complete feature specifications
- Zero-configuration startup requirements
- Audio quality requirements (sample-accurate crossfading)

**[REQ002: Entity Definitions](REQ002-entity_definitions.md)** (MUST READ - 5 min)
- Core concepts: Passage, Song, Musical Flavor, Timeslot
- Database model overview

**[SPEC001: Architecture](SPEC001-architecture.md)** (MUST READ - 15 min, scan sections as needed)
- 6 microservices architecture (wkmp-ap, wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le, wkmp-dr)
- HTTP REST + SSE communication patterns
- Module responsibilities and port assignments
- Audio pipeline architecture (Decoder → Fader → Buffer → Mixer → Output)
- Zero-configuration startup pattern

---

## Step 4: Set Up Development Environment (5 min)

### Required Reading

**[IMPL003: Project Structure](IMPL003-project_structure.md)** (QUICK SCAN - 3 min)
- Workspace layout (6 modules + common library)
- Build commands (`cargo build`, `cargo test`)

**[IMPL002: Coding Conventions](IMPL002-coding_conventions.md)** (QUICK SCAN - 2 min)
- Rust style guidelines
- Error handling patterns
- Requirement traceability (use `[REQ-XXX-NNN]` tags in comments)

### Quick Commands

```bash
# Build all modules
cargo build

# Run all tests
cargo test

# Build specific module
cargo build -p wkmp-ap

# Run module in dev mode
cargo run -p wkmp-ui
```

---

## Step 5: Understand Workflows (5 min)

### Required Reading

**[DWI001: Workflow Quickstart](../workflows/DWI001_workflow_quickstart.md)** (MUST READ - 5 min)
- `/commit` - Multi-step commit with automatic change tracking
- `/think` - Multi-agent analysis for complex decisions
- `/plan` - Specification-driven implementation planning
- `/archive` - Move completed documents to archive branch

**Key Principle:** Always use `/commit` for commits (maintains change_history.md automatically)

---

## 🎯 You're Ready!

**You've completed the required reading.** You now understand:
- ✅ Project goals and decision-making framework
- ✅ Documentation structure
- ✅ Core entities and requirements
- ✅ System architecture
- ✅ Development workflows

---

## Reference Documents (Read as Needed)

### When Implementing Features

| Task | Reference Document |
|------|-------------------|
| Database changes | [IMPL001: Database Schema](IMPL001-database_schema.md) |
| API endpoints | [SPEC007: API Design](SPEC007-api_design.md) |
| Playback loop and orchestration | [SPEC028: Playback Orchestration](SPEC028-playback_orchestration.md) |
| Crossfading logic | [SPEC002: Crossfade](SPEC002-crossfade.md) |
| Musical flavor algorithm | [SPEC003: Musical Flavor](SPEC003-musical_flavor.md) |
| Program Director selection | [SPEC005: Program Director](SPEC005-program_director.md) |
| Event system | [SPEC011: Event System](SPEC011-event_system.md) |
| Audio file import | [SPEC024: Audio Ingest Architecture](SPEC024-audio_ingest_architecture.md) |

### When Debugging/Troubleshooting

| Issue | Reference Document |
|-------|-------------------|
| Error handling patterns | [SPEC021: Error Handling](SPEC021-error_handling.md) |
| Performance issues | [SPEC022: Performance Targets](SPEC022-performance_targets.md) |
| Deployment issues | [IMPL004: Deployment](IMPL004-deployment.md) |

### When Making Architecture Decisions

- [ADR-001: Mixer Refactoring](ADR-001-mixer_refactoring.md)
- [ADR-002: Event-Driven Position Tracking](ADR-002-event_driven_position_tracking.md)

---

## Finding Archived Documents

**[REG002: Archive Index](../workflows/REG002_archive_index.md)**
- Completed implementation plans and analysis documents
- Retrieved via `git show archive-branch:<path>` or `/archive` workflow

**When to check archives:**
- Looking for historical context on completed features
- Understanding why certain design decisions were made
- Finding implementation plans for reference

---

## Document Classification

### 📕 Required Reading (Read First)
Essential for all contributors:
- PCH001 (Project Charter)
- GOV001 (Document Hierarchy)
- REQ001 (Requirements - sections as needed)
- REQ002 (Entity Definitions)
- SPEC001 (Architecture)
- DWI001 (Workflow Quickstart)

### 📘 Core Reference (Read When Needed)
Frequently referenced during development:
- SPEC007 (API Design)
- IMPL001 (Database Schema)
- IMPL002 (Coding Conventions)
- IMPL003 (Project Structure)

### 📙 Detailed Reference (Domain-Specific)
Read only when working on specific features:
- All other SPEC### documents
- All other IMPL### documents
- ADR documents (architecture decisions)

---

## Getting Help

1. **Documentation Questions:** Check [GOV001](GOV001-document_hierarchy.md) for navigation
2. **Requirement Clarification:** See [REQ001](REQ001-requirements.md) or ask technical lead
3. **Implementation Guidance:** Check relevant IMPL### or SPEC### document
4. **Workflow Questions:** See [DWI001](../workflows/DWI001_workflow_quickstart.md)

---

## Next Steps

Choose based on your role:

**Implementing a Feature:**
1. Read the relevant SPEC### document
2. Run `/plan <spec_document>` to create implementation plan
3. Follow test-first approach per traceability matrix
4. Use `/commit` when ready to commit changes

**Fixing a Bug:**
1. Understand the relevant SPEC### and IMPL### documents
2. Write a failing test that reproduces the bug
3. Fix the bug
4. Use `/commit` to commit with descriptive message

**Adding a New Module:**
1. Review [SPEC001](SPEC001-architecture.md) (or CLAUDE.md) for architecture patterns
2. Follow [IMPL003](IMPL003-project_structure.md) for structure
3. Implement zero-config startup per [REQ-NF-030] through [REQ-NF-037]
4. Use `/commit` for changes

---

**Version:** 1.0
**Maintained By:** WKMP Development Team
