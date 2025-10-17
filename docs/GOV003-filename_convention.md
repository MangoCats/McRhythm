# WKMP Documentation Filename Convention

**Status:** APPROVED
**Date Approved:** 2025-10-17
**Tier:** 0 (Governance)
**Purpose:** Establish a consistent filename convention that makes document type and hierarchy immediately obvious

---

## Executive Summary

This document establishes a systematic filename convention for all WKMP documentation that:
- **Maps directly to the 5-tier document hierarchy** (Tiers 0-4 + Special categories)
- **Uses consistent 3-4 letter prefixes** to indicate document type
- **Includes sequential numbering** (001, 002, etc.) within each category
- **Preserves human-readable names** after the prefix
- **Groups naturally in file explorers** by alphabetical sorting
- **Scales easily** for future document additions

### Example Transformations

| Current Filename | Proposed Filename | Rationale |
|------------------|-------------------|-----------|
| `document_hierarchy.md` | `GOV001-document_hierarchy.md` | Tier 0: Governance |
| `requirements.md` | `REQ001-requirements.md` | Tier 1: Authoritative Requirements |
| `entity_definitions.md` | `REQ002-entity_definitions.md` | Tier 1: Authoritative Requirements |
| `architecture.md` | `SPEC001-architecture.md` | Tier 2: System Design |
| `crossfade.md` | `SPEC002-crossfade.md` | Tier 2: Technical Design |
| `database_schema.md` | `IMPL001-database_schema.md` | Tier 3: Implementation Specs |
| `implementation_order.md` | `EXEC001-implementation_order.md` | Tier 4: Execution Plan |
| `review-findings.md` | `REV001-review_findings.md` | Special: Review Document |

---

## Governance Context

This is a **Tier 0 governance document**, which means it governs the documentation system itself. It has the same authority level as [GOV001-document_hierarchy.md](GOV001-document_hierarchy.md) and establishes naming rules that all documentation must follow.

**Relationship to Document Hierarchy:**
- GOV001-document_hierarchy.md defines the tier system and information flow
- GOV003-filename_convention.md defines how documents are named
- Together, they form the complete governance framework for WKMP documentation

---

## Prefix Definitions

### Tier-Based Prefixes

| Prefix | Tier | Meaning | When to Use | Examples |
|--------|------|---------|-------------|----------|
| **GOV** | 0 | **Governance** | Meta-documentation that governs the documentation system itself | GOV001-document_hierarchy.md, GOV003-filename_convention.md |
| **REQ** | 1 | **Requirements** | Authoritative WHAT specifications; product requirements and core definitions | REQ001-requirements.md, REQ002-entity_definitions.md |
| **SPEC** | 2 | **Specifications** | HOW the system is designed; architecture, algorithms, and design decisions | SPEC001-architecture.md, SPEC002-crossfade.md |
| **IMPL** | 3 | **Implementation** | Concrete technical details; database schemas, code conventions, deployment | IMPL001-database_schema.md, IMPL002-coding_conventions.md |
| **EXEC** | 4 | **Execution** | WHEN features are built; implementation plans and task ordering | EXEC001-implementation_order.md |

### Special Category Prefixes

| Prefix | Meaning | When to Use | Examples |
|--------|---------|-------------|----------|
| **ENUM** | Enumeration | Requirement ID schemes and traceability frameworks | ENUM001-requirements_enumeration.md |
| **REV** | Review | Documentation reviews, audits, and findings | REV001-review_findings.md |
| **PLAN** | Plan | Migration plans, proposals, and strategic documents | PLAN001-single_stream_migration_plan.md |
| **STATUS** | Status | Status reports, proof-of-concept outcomes, progress updates | STATUS001-single_stream_poc_status.md |
| **DATA** | Data/Samples | Sample data files, examples, fixtures | DATA001-sample_highlevel.json |
| **INDEX** | Index/README | Directory indexes and navigation documents | INDEX001-README.md |

### Archive Prefix

| Prefix | Meaning | When to Use | Examples |
|--------|---------|-------------|----------|
| **ARCH** | Archived | Superseded documents retained for historical reference | ARCH001-dual_pipeline_design.md |

**Note:** "ARCH" for "Archived" is distinct from "architecture" (which uses "SPEC" prefix for Tier 2).

---

## Complete Mapping Table

### Current Documentation → Proposed Filenames

#### Tier 0: Governance (GOV)

| Current | Proposed | Notes |
|---------|----------|-------|
| `document_hierarchy.md` | `GOV001-document_hierarchy.md` | Meta-documentation governance |
| `filename_convention_proposal.md` | `GOV003-filename_convention.md` | This document (Tier 0 governance) |

#### Tier 1: Requirements (REQ)

| Current | Proposed | Notes |
|---------|----------|-------|
| `requirements.md` | `REQ001-requirements.md` | Primary authoritative requirements |
| `entity_definitions.md` | `REQ002-entity_definitions.md` | Core entity terminology (authoritative) |

#### Tier 2: Specifications (SPEC)

| Current | Proposed | Notes |
|---------|----------|-------|
| `architecture.md` | `SPEC001-architecture.md` | Overall system design |
| `crossfade.md` | `SPEC002-crossfade.md` | Crossfade algorithm design |
| `musical_flavor.md` | `SPEC003-musical_flavor.md` | Musical flavor characterization |
| `musical_taste.md` | `SPEC004-musical_taste.md` | Musical taste calculation |
| `program_director.md` | `SPEC005-program_director.md` | Selection algorithm design |
| `like_dislike.md` | `SPEC006-like_dislike.md` | User preference design |
| `api_design.md` | `SPEC007-api_design.md` | REST API and SSE interface |
| `library_management.md` | `SPEC008-library_management.md` | File scanning and metadata workflows |
| `ui_specification.md` | `SPEC009-ui_specification.md` | Web UI design |
| `user_identity.md` | `SPEC010-user_identity.md` | Authentication design |
| `event_system.md` | `SPEC011-event_system.md` | Event-driven communication |
| `multi_user_coordination.md` | `SPEC012-multi_user_coordination.md` | Multi-user coordination mechanisms |
| `single-stream-playback.md` | `SPEC013-single_stream_playback.md` | Single-stream audio playback design |
| `single-stream-design.md` | `SPEC014-single_stream_design.md` | Single-stream detailed design |

#### Tier 3: Implementation (IMPL)

| Current | Proposed | Notes |
|---------|----------|-------|
| `database_schema.md` | `IMPL001-database_schema.md` | Database implementation specs |
| `coding_conventions.md` | `IMPL002-coding_conventions.md` | Code quality standards |
| `project_structure.md` | `IMPL003-project_structure.md` | Workspace organization |
| `deployment.md` | `IMPL004-deployment.md` | Deployment and operations |
| `audio_file_segmentation.md` | `IMPL005-audio_file_segmentation.md` | Segmentation workflow implementation |

#### Tier 4: Execution (EXEC)

| Current | Proposed | Notes |
|---------|----------|-------|
| `implementation_order.md` | `EXEC001-implementation_order.md` | Development sequence and phases |

#### Special Categories

| Current | Proposed | Category | Notes |
|---------|----------|----------|-------|
| `requirements_enumeration.md` | `ENUM001-requirements_enumeration.md` | Enumeration | Requirement ID scheme |
| `review-findings.md` | `REV001-review_findings.md` | Review | Current review findings |
| `review-findings-backup-20251017.md` | `REV001-review_findings-backup-20251017.md` | Review (backup) | Keep date suffix for backups |
| `single-stream-poc-status.md` | `STATUS001-single_stream_poc_status.md` | Status | POC outcome report |
| `sample_highlevel.json` | `DATA001-sample_highlevel.json` | Data | Sample AcousticBrainz data |
| `README.md` | `INDEX001-README.md` | Index | Main documentation index |

#### Archive Directory (docs/archive/)

| Current | Proposed | Notes |
|---------|----------|-------|
| `README.md` | `INDEX001-README.md` | Archive directory index |
| `dual-pipeline-design_archived.md` | `ARCH001-dual_pipeline_design.md` | Remove "_archived" suffix (ARCH prefix makes it clear) |
| `gstreamer_design_archived.md` | `ARCH002-gstreamer_design.md` | Remove "_archived" suffix |
| `architecture-comparison_archived.md` | `ARCH003-architecture_comparison.md` | Remove "_archived" suffix |
| `single-stream-migration-proposal.md` | `ARCH004-single_stream_migration_proposal.md` | Completed migration proposal |

---

## Numbering Guidelines

### Sequential Numbering Within Prefix

**Rule:** Numbers are sequential within each prefix category (001, 002, 003, ...)

**Benefits:**
- Clear ordering for related documents
- Easy to see at a glance how many documents of each type exist
- Simple to add new documents (use next available number)

### Number Assignment Strategy

**Option A: Creation Order** (RECOMMENDED)
- Assign numbers based on when documents were created
- Preserve historical sequence
- Simple and unambiguous

**Option B: Logical Grouping**
- Group related specs together (e.g., all audio specs, all UI specs)
- Requires subjective decisions about grouping
- More organizational overhead

**Recommendation:** Use **Option A (Creation Order)** for initial migration, then use logical grouping for new documents.

### Renumbering Policy

**When to Renumber:**
- Never renumber to fix "gaps" in sequence
- Only renumber when reorganizing entire category
- Only with explicit approval from technical lead

**Rationale:** Renumbering breaks:
- Git history references
- Cross-references in other documents
- External links and bookmarks
- Developer mental models

### Gaps in Numbering

**Policy:** Gaps are acceptable and expected
- Documents may be removed/archived
- Documents may be split or merged
- Numbers are identifiers, not counts

**Example:**
```
SPEC001-architecture.md
SPEC002-crossfade.md
SPEC005-program_director.md  ← Gap is OK (003 and 004 were archived)
SPEC006-like_dislike.md
```

---

## Sorting Benefits

### Natural Alphabetical Grouping

With this convention, `ls` or file explorer sorting naturally groups by type:

```
docs/
├── DATA001-sample_highlevel.json
├── ENUM001-requirements_enumeration.md
├── EXEC001-implementation_order.md
├── GOV001-document_hierarchy.md
├── GOV003-filename_convention.md
├── IMPL001-database_schema.md
├── IMPL002-coding_conventions.md
├── IMPL003-project_structure.md
├── IMPL004-deployment.md
├── IMPL005-audio_file_segmentation.md
├── INDEX001-README.md
├── REQ001-requirements.md
├── REQ002-entity_definitions.md
├── REV001-review_findings.md
├── SPEC001-architecture.md
├── SPEC002-crossfade.md
├── SPEC003-musical_flavor.md
├── SPEC004-musical_taste.md
├── SPEC005-program_director.md
├── SPEC006-like_dislike.md
├── SPEC007-api_design.md
├── SPEC008-library_management.md
├── SPEC009-ui_specification.md
├── SPEC010-user_identity.md
├── SPEC011-event_system.md
├── SPEC012-multi_user_coordination.md
├── SPEC013-single_stream_playback.md
├── SPEC014-single_stream_design.md
└── STATUS001-single_stream_poc_status.md

archive/
├── ARCH001-dual_pipeline_design.md
├── ARCH002-gstreamer_design.md
├── ARCH003-architecture_comparison.md
├── ARCH004-single_stream_migration_proposal.md
└── INDEX001-README.md
```

### Benefits of This Sorting

1. **Tier visibility:** All Tier 2 specs (SPEC) group together
2. **Category clarity:** See all implementations (IMPL) at a glance
3. **Navigation efficiency:** Find document type without opening files
4. **Onboarding:** New developers immediately understand structure
5. **Tool compatibility:** Works with any file manager or CLI

---

## Special Cases and Edge Cases

### 1. Multi-Part Documents

**Scenario:** A specification becomes too large and needs to be split

**Option A: Use Sub-numbering** (NOT RECOMMENDED)
```
SPEC005-program_director.md
SPEC005A-selection_algorithm.md
SPEC005B-cooldown_system.md
```

**Option B: Use Sequential Numbers** (RECOMMENDED)
```
SPEC005-program_director.md
SPEC015-selection_algorithm.md
SPEC016-cooldown_system.md
```

**Recommendation:** Use Option B (sequential numbers) to avoid complexity

### 2. Temporary/Draft Documents

**Scenario:** Working drafts, proposals, or experiments

**Option A: Use DRAFT prefix**
```
DRAFT001-new_feature_proposal.md
```

**Option B: Use existing prefixes with DRAFT in filename**
```
SPEC999-DRAFT-new_feature_design.md
```

**Recommendation:** Use existing directory structure (e.g., `docs/drafts/`) rather than filename prefixes

### 3. Version-Specific Documentation

**Scenario:** Documentation specific to Full/Lite/Minimal versions

**Solution:** Include version in filename after main name
```
IMPL004-deployment-full_version.md
IMPL004-deployment-lite_version.md
```

**Current Status:** Not needed yet (version differences documented within files)

### 4. Date-Stamped Documents

**Scenario:** Review findings, status reports with dates

**Solution:** Keep date suffix after main name
```
REV001-review_findings-20251017.md
REV001-review_findings-backup-20251017.md
STATUS001-single_stream_poc_status-20251016.md
```

**Current Status:** Only use for historical backups

### 5. External References

**Scenario:** Documents link to external resources or files

**Solution:** External files use DATA prefix
```
DATA001-sample_highlevel.json
DATA002-example_passage_timings.csv
```

### 6. Diagrams and Images

**Scenario:** Visual assets referenced by documentation

**Solution:** Use subdirectory with prefix
```
docs/
├── SPEC001-architecture.md
└── assets/
    ├── SPEC001-architecture-diagram.png
    └── SPEC002-crossfade-curves.svg
```

---

## Migration Considerations

### Impact on Git History

**Issue:** Renaming files breaks `git log --follow` for some operations

**Mitigation:**
- Perform all renames in a single atomic commit
- Use `git mv` to preserve history
- Add comprehensive commit message listing all transformations
- Tag the commit for easy reference (`git tag v1.0-filename-migration`)

**Suggested Commit Message Format:**
```
docs: Migrate to prefix-based filename convention

Implements GOV003 filename convention. All documentation files
renamed to use tier-based prefixes (GOV/REQ/SPEC/IMPL/EXEC) with
sequential numbering.

Mapping preserved in:
- docs/GOV003-filename_convention.md (migration table)
- This commit message

Key transformations:
- document_hierarchy.md → GOV001-document_hierarchy.md
- requirements.md → REQ001-requirements.md
- architecture.md → SPEC001-architecture.md
[... complete list ...]

Ref: docs/GOV001-document_hierarchy.md, docs/GOV003-filename_convention.md
```

### Cross-References in Files

**Issue:** Many documents reference other documents by filename

**Impact:** High - 968+ requirement ID references across 20+ files

**Strategy:**
1. **Find all references:** `grep -r "\.md" docs/`
2. **Update systematically:** Use find-and-replace per file
3. **Verify links:** Run markdown link checker after migration
4. **Test documentation generation:** If using doc tools, verify they still work

**Example Cross-References to Update:**
```markdown
# Before
See [Requirements](requirements.md) for details.

# After
See [Requirements](REQ001-requirements.md) for details.
```

### External Documentation

**Issue:** README.md, CLAUDE.md, and other root-level files reference docs/

**Impact:** Medium - Several references in CLAUDE.md

**Strategy:** Update all references in single pass:
```bash
# Find all markdown files referencing docs/
grep -r "docs/" *.md

# Update references
# Example: docs/requirements.md → docs/REQ001-requirements.md
```

### Documentation Generation Tools

**Impact:** If using tools like mdBook, Docusaurus, or custom generators

**Mitigation:**
- Update SUMMARY.md or equivalent navigation files
- Update any URL rewriting rules
- Test generated output before deploying

---

## Alternative Approaches Considered

### Alternative 1: Tier Number in Prefix

**Format:** `T0-document_hierarchy.md`, `T1-requirements.md`, `T2-architecture.md`

**Pros:**
- Very clear tier mapping
- Extremely short prefix (T0, T1, T2)

**Cons:**
- Confusing for special categories (what tier is a review document?)
- Less descriptive (T2 doesn't convey "specification")
- Numbers feel arbitrary without context

**Decision:** REJECTED - Less readable than descriptive prefixes

### Alternative 2: Hierarchical Numbering

**Format:** `001-000-document_hierarchy.md`, `001-010-requirements.md`, `002-010-architecture.md`

**Pros:**
- Can encode both tier and sequence
- Numeric sorting matches logical structure

**Cons:**
- Less human-readable
- Harder to remember
- Requires complex numbering scheme
- Still needs prefix for document type

**Decision:** REJECTED - Over-engineered for this project size

### Alternative 3: Domain-Based Prefixes

**Format:** `AUDIO-crossfade.md`, `DB-schema.md`, `UI-specification.md`

**Pros:**
- Groups by functional domain
- Clear what area of system is documented

**Cons:**
- Doesn't map to document hierarchy
- Some documents span multiple domains
- Loses tier information

**Decision:** REJECTED - Violates document hierarchy governance model

### Alternative 4: Date-Based Prefixes

**Format:** `2025-10-01-requirements.md`

**Pros:**
- Clear chronological ordering
- Version control built into filename

**Cons:**
- Date of creation doesn't convey document purpose
- Doesn't map to tier structure
- Sorting by date is rarely useful for documentation

**Decision:** REJECTED - Dates don't help identify document type

---

## FAQ

### Q1: Why not use subdirectories instead of prefixes?

**A:** Subdirectories add navigation friction:
- More clicks/commands to reach files
- Harder to see all documents at once
- Complicates cross-references (relative paths)
- Reduces discoverability

Prefixes provide organization without hierarchy depth.

### Q2: What happens when we reach 999 documents in a category?

**A:** Use four digits (0001, 0002, ...). However, this is unlikely:
- Current project has approximately 30 docs total
- SPEC (largest category) has 14 documents
- Would take years to reach 999 in any category

### Q3: Can I create a document without following the convention?

**A:** No, with exceptions:
- Draft documents (use `docs/drafts/` directory)
- Personal notes (use `docs/notes/` directory, not committed)
- Generated files (use `docs/generated/` directory)
- All committed documentation MUST follow convention

### Q4: How do I know what number to assign a new document?

**A:** Use next available number in the prefix category:
```bash
# List existing SPEC documents
ls docs/SPEC*.md | tail -1
# Output: SPEC014-single_stream_design.md

# Next number is SPEC015
```

### Q5: What if I want to rename a document?

**A:** Renaming requires approval:
- Minor: Rename descriptive part (e.g., `SPEC005-program_director.md` → `SPEC005-program_director_algorithm.md`) - Seek review
- Major: Change prefix or number - Requires technical lead approval

### Q6: How does this relate to requirement IDs?

**A:** They are separate systems:
- **Requirement IDs (DOC-CAT-NNN):** Track individual requirements/specs within documents
- **Filenames (PREFIX###-name.md):** Organize document files

Example:
- File: `SPEC002-crossfade.md`
- Contains requirement IDs: `[XFD-TP-010]`, `[XFD-CURV-020]`, etc.

### Q7: What about non-markdown files?

**A:** Convention applies to all documentation files:
- Markdown: `SPEC001-architecture.md`
- JSON: `DATA001-sample_highlevel.json`
- CSV: `DATA002-example_timings.csv`
- Images: `assets/SPEC001-diagram.png`

### Q8: Can I use this convention in other directories?

**A:** Convention is specific to `docs/` and `docs/archive/`:
- Source code: Follow language conventions (snake_case, camelCase)
- Tests: Follow test framework conventions
- Config: Follow tool conventions (Cargo.toml, .gitignore)

---

## References

- [GOV001-document_hierarchy.md](GOV001-document_hierarchy.md) - Document tier system and governance
- [ENUM001-requirements_enumeration.md](ENUM001-requirements_enumeration.md) - Requirement ID scheme (DOC-CAT-NNN format)
- [REV001-review_findings.md](REV001-review_findings.md) - Documentation quality review identifying need for this convention

---

**Document Status:** APPROVED
**Version:** 1.0
**Date Approved:** 2025-10-17
**Maintained By:** Technical lead, documentation lead
**Next Review:** After migration completion

---

End of document - WKMP Documentation Filename Convention
