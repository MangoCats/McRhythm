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

1. **Understand the documentation framework:** Read [GOV001-document_hierarchy.md](GOV001-document_hierarchy.md)
2. **Learn what WKMP does:** Read [REQ001-requirements.md](REQ001-requirements.md)
3. **Understand the architecture:** Read [SPEC001-architecture.md](SPEC001-architecture.md)
4. **See implementation details:** Browse Tier 3 documents (IMPL###-*.md)
5. **Follow the development plan:** Read [EXEC001-implementation_order.md](EXEC001-implementation_order.md)

---

## Document Types

Technical documents use systematic naming conventions (see [GOV003-filename_convention.md](GOV003-filename_convention.md)):

- **GOV###:** Governance and meta-documentation
- **REQ###:** Requirements and entity definitions
- **SPEC###:** Design specifications
- **IMPL###:** Implementation details
- **EXEC###:** Execution plans and roadmaps
- **REV###:** Design reviews and architectural changes
- **CHANGELOG-*:** Detailed change tracking
- **ADDENDUM-*:** Temporary clarifications
- **MIGRATION-*:** Migration guides

---

## Architecture Overview

### Playback Architecture

1. **[architecture-comparison.md](archive/ARCH003-architecture_comparison.md)** - **Historical Reference**
   - Side-by-side comparison of Dual Pipeline vs Single Stream approaches
   - Detailed analysis of implementation complexity, performance, and deployment
   - Recommendation table with scores
   - Best for: Decision making and understanding trade-offs

2. **[single-stream-design.md](SPEC013-single_stream_playback.md)** - **Current Design**
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
1. Read: Crossfade Quality sections in `architecture-comparison.md`
2. Review: Fade curve algorithms in `single-stream-design.md`
3. Compare: Timing precision (sample-accurate vs property-based)

### ðŸ—ï¸ Architecture Diagram

#### Single Stream (Manual Buffers)
```
Application
├── Decoder Pool (parallel decoding)
│   └── Uses symphonia + rubato
├── Passage Buffer Manager
│   └── 15-second PCM buffers per passage
├── Crossfade Mixer
│   └── Sample-accurate mixing with fade curves
└── Audio Output (cpal)
    └── Ring buffer → audio device
```


### ðŸš€ Getting Started

**To plan Single Stream migration:**
```bash
# Read the detailed design
cat docs/single-stream-design.md

# Review comparison
cat docs/architecture-comparison.md

# Check implementation phases
grep -A 20 "## Implementation Phases" docs/single-stream-design.md
```

### ðŸ¤ Contributing

When updating these documents:
1. Keep version numbers in sync
2. Update "Last Updated" dates
3. Cross-reference related documents
4. Maintain code examples with actual implementation
5. Update this README if adding new documents

### 📝§ Questions?

For technical questions about:
- **Single Stream**: See `single-stream-design.md` "Challenges and Solutions"

---

**Documentation Set Version:** 1.0
**Created:** 2025-10-16
**Maintained By:** WKMP Development Team
