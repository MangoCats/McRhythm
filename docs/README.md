# WKMP Documentation

This directory contains comprehensive documentation for the WKMP (Auto DJ Music Player) project.

## Documentation Structure

WKMP documentation is organized into two major categories:

### 📘 Technical Documentation (docs/ root)

**Audience:** Developers, architects, contributors

Technical documentation follows a strict 5-tier hierarchy (see [GOV001-document_hierarchy.md](GOV001-document_hierarchy.md)):

- **Tier 0 (Governance):** Documentation framework and standards
- **Tier R (Review):** Design reviews, architectural changes, change logs
- **Tier 1 (Requirements):** What the system must do
- **Tier 2 (Design):** How requirements are satisfied
- **Tier 3 (Implementation):** Concrete implementation specifications
- **Tier 4 (Execution):** When features are built

**Key Documents:**
- [Document Hierarchy](GOV001-document_hierarchy.md) - START HERE for understanding doc structure
- [Requirements](REQ001-requirements.md) - Complete feature specifications
- [Architecture](SPEC001-architecture.md) - System structure and components
- [API Design](SPEC007-api_design.md) - REST API and SSE specifications
- [Database Schema](IMPL001-database_schema.md) - Data model and migrations
- [Implementation Order](EXEC001-implementation_order.md) - Development roadmap

### 📗 User Documentation (docs/user/)

**Audience:** End users, system administrators

User documentation provides practical guides for installation, operation, and troubleshooting:

- **[QUICKSTART.md](user/QUICKSTART.md)** - Get WKMP running in under 5 minutes
- **[TROUBLESHOOTING.md](user/TROUBLESHOOTING.md)** - Diagnose and resolve common issues

See [docs/user/README.md](user/README.md) for complete user documentation index.

---

## Quick Start for Developers

**👉 NEW DEVELOPERS: Start with [DEV_QUICKSTART.md](DEV_QUICKSTART.md)** (15-30 min)

This curated guide provides:
- Essential reading list (5 documents)
- Clear learning path
- Setup instructions
- Reference document index
- Required Reading vs Reference classification

---

## Document Classification

### 📕 Required Reading
**Read these first** - Essential for all contributors (15-30 min total):
- **[DEV_QUICKSTART.md](DEV_QUICKSTART.md)** - Start here!
- **[PCH001: Project Charter](PCH001_project_charter.md)** - Project goals and decision framework
- **[GOV001: Document Hierarchy](GOV001-document_hierarchy.md)** - Documentation structure
- **[REQ002: Entity Definitions](REQ002-entity_definitions.md)** - Core concepts
- **[SPEC001: Architecture](SPEC001-architecture.md)** - Microservices design and audio pipeline
- **[DWI001: Workflow Quickstart](../workflows/DWI001_workflow_quickstart.md)** - Development workflows

### 📘 Core Reference
**Read when implementing features** - Frequently referenced:
- **[REQ001: Requirements](REQ001-requirements.md)** - Complete specifications (read sections as needed)
- **[SPEC007: API Design](SPEC007-api_design.md)** - REST API and SSE specs
- **[SPEC028: Playback Orchestration](SPEC028-playback_orchestration.md)** - Playback loop, mixer thread, event system
- **[IMPL001: Database Schema](IMPL001-database_schema.md)** - Data model
- **[IMPL002: Coding Conventions](IMPL002-coding_conventions.md)** - Style guide
- **[IMPL003: Project Structure](IMPL003-project_structure.md)** - Workspace layout

### 📙 Detailed Reference
**Read when working on specific features** - Domain-specific:
- All other SPEC### documents (design specifications)
- All other IMPL### documents (implementation details)
- ADR documents (architecture decision records)
- EXEC001 (implementation roadmap)

### 📗 Archived Documents
**Historical context** - Completed work (see [REG002_archive_index.md](../workflows/REG002_archive_index.md)):
- Implementation plans for completed features
- Analysis documents
- Retrieved via archive branch

---

## Document Types

Technical documents use systematic naming conventions (see [GOV003-filename_convention.md](GOV003-filename_convention.md)):

---

## Architecture Overview

### Playback Architecture

 **[single-stream-design.md](SPEC013-single_stream_playback.md)** - **Current Design**
   - Manual buffer management with sample-accurate crossfading
   - Pure Rust implementation using symphonia, rubato, and cpal
   - Detailed component design and implementation phases

### 📜 Reading Guide

#### For Developers (Implementing Single Stream)
1. Read: `single-stream-design.md`
2. Review: Component Structure and Data Flow
3. Follow: Implementation Phases (Week 1-4 plan)
4. Reference: Code examples and algorithm pseudocode

#### For Audio Engineers
1. Review: Fade curve algorithms in `single-stream-design.md`
2. Compare: Timing precision (sample-accurate vs property-based)

### ðŸ¤ Contributing

When updating these documents:
1. Keep version numbers in sync
2. Update "Last Updated" dates
3. Cross-reference related documents
4. Maintain code examples with actual implementation
5. Update this README if adding new documents

### 📝 Questions?

For technical questions about:
- **Single Stream**: See `single-stream-design.md` "Challenges and Solutions"

---

**Documentation Set Version:** 1.0
**Created:** 2025-10-16
**Maintained By:** WKMP Development Team
