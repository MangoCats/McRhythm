# Document Number Registry

**Purpose:** Track document number assignments and maintain next-available counters
**Maintained by:** /doc-name workflow (automated)
**Last Updated:** 2025-10-30

---

## Next Available Numbers

| Category | Next | Description |
|----------|------|-------------|
| GOV | 004 | Governance (document hierarchy, conventions, requirements enumeration) |
| REQ | 003 | Requirements (authoritative requirements, entity definitions) |
| SPEC | 027 | Specifications (design specs, API design, crossfade, musical flavor, etc.) |
| IMPL | 008 | Implementation (database schema, coding conventions, structure, deployment) |
| EXEC | 002 | Execution (implementation order, schedules) |
| REV | 005 | Reviews (design reviews, architecture updates) |
| GUIDE | 004 | Guides (implementation guides, tutorials) |
| RPT | 001 | Reports & Analysis (/think outputs, research, investigations) |
| DWI | 001 | Developer Work Instructions (workflow procedures, process documentation) |
| TMPL | 001 | Templates (reusable document templates, patterns) |
| PLAN | 001 | Implementation Plans (/plan outputs, project plans) |
| LOG | 001 | Operational Logs (feedback logs, execution logs, metrics) |
| REG | 003 | Registries (this file, archive index, category definitions) |

---

## Category Definitions

### Existing WKMP Categories

**GOV### - Governance**
- Documentation framework and conventions
- Requirements enumeration systems
- File naming conventions
- Location: `docs/`

**REQ### - Requirements**
- Authoritative requirements (WHAT system must do)
- Entity definitions
- Location: `docs/`

**SPEC### - Specifications**
- Design specifications (HOW requirements are satisfied)
- API design, crossfade algorithms, musical flavor, UI specs
- Location: `docs/`

**IMPL### - Implementation**
- Concrete implementation specifications
- Database schema, coding conventions, project structure, deployment
- Location: `docs/`

**EXEC### - Execution**
- Implementation order and schedules
- Aggregates all upstream specs
- Location: `docs/`

**REV### - Reviews**
- Design reviews
- Architecture decision updates
- Location: `docs/`

**GUIDE### - Guides**
- Implementation guides
- Tutorials and walkthroughs
- Location: `docs/`

### New Categories (from Workflow Adoption)

**RPT### - Reports & Analysis**
- /think workflow outputs
- Research reports
- Investigation findings
- Location: `docs/` or `wip/`

**DWI### - Developer Work Instructions**
- Workflow procedures (how we build)
- Process documentation
- Workflow architecture and design
- Location: `workflows/`

**TMPL### - Templates**
- Reusable document templates
- Workflow patterns
- Location: `workflows/`

**PLAN### - Implementation Plans**
- /plan workflow outputs
- Project plans with test specifications
- Implementation breakdown and schedules
- Location: `wip/` (then archived when complete)

**LOG### - Operational Logs**
- Ongoing logs (this change_history file)
- Feedback tracking, metrics
- Location: `project_management/`

**REG### - Registries**
- System registries (this file)
- Archive index, lookup tables
- Location: `workflows/`

---

## Assignment History

| Number | Filename | Date | Category | Method | Notes |
|--------|----------|------|----------|--------|-------|
| REG001 | number_registry.md | 2025-10-25 | REG | Manual | Initial registry creation |
| REG002 | archive_index.md | 2025-10-25 | REG | Manual | Archive retrieval index |
| SPEC021 | SPEC021-error_handling.md | 2025-10-25 | SPEC | Manual | Comprehensive error handling strategy specification |
| SPEC022 | SPEC022-performance_targets.md | 2025-10-25 | SPEC | Manual | Performance targets for wkmp-ap (Pi Zero 2W deployment) |
| SPEC023 | SPEC023-timing_terminology.md | 2025-10-26 | SPEC | Manual | Timing terminology and conventions across WKMP |
| SPEC024 | SPEC024-audio_ingest_architecture.md | 2025-10-26 | SPEC | Manual | Architecture for Audio Ingest module (wkmp-ai) |
| SPEC025 | SPEC025-amplitude_analysis.md | 2025-10-26 | SPEC | Manual | Amplitude analysis for crossfade timing |
| SPEC026 | SPEC026-api_key_configuration.md | 2025-10-30 | SPEC | Manual | Multi-tier API key configuration system (migrated from wip/) |
| GUIDE003 | audio_pipeline_diagrams.md | 2025-10-27 | GUIDE | Auto | Visual reference for audio processing pipeline with DBD-PARAM mapping |
| PLAN010 | workflow_quality_standards | 2025-10-30 | PLAN | Auto | Implementation plan for workflow quality standards enhancement (anti-sycophancy, anti-laziness, anti-hurry, problem transparency) |

<!-- /doc-name workflow will append entries below -->

---

## Document Counts by Category

| Category | Count | Last Updated |
|----------|-------|--------------|
| GOV | 3 | Existing |
| REQ | 2 | Existing |
| SPEC | 26 | 2025-10-30 |
| IMPL | 7 | Existing |
| EXEC | 1 | Existing |
| REV | 4 | Existing |
| GUIDE | 2 | 2025-10-27 |
| RPT | 0 | New |
| DWI | 0 | New |
| TMPL | 0 | New |
| PLAN | 0 | New |
| LOG | 0 | New |
| REG | 2 | 2025-10-25 |

---

## Usage

**Assign document number:**
```bash
/doc-name path/to/document.md
```

The /doc-name workflow will:
1. Analyze document location, name, and content
2. Recommend appropriate category
3. Get next available number from this registry
4. Rename file to CAT###_original_name.md
5. Update this registry (increment next available, add history entry)
6. Stage changes for /commit

---

**Maintained by:** /doc-name workflow
**Format:** Markdown table
**Version:** 1.0
