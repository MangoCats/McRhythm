# Modular Documentation Templates

**Purpose:** Templates for creating context-window-optimized modular documentation per WKMP documentation standards (CLAUDE.md + GOV001).

---

## When to Use Modular Structure

**Required for documents >1200 lines:**
- Create folder: `[document_name]/`
- Use modular structure (summary + section files)
- Mandatory per GOV001 "Document Size and Structure Standards"

**Recommended for documents 300-1200 lines:**
- Add executive summary section at beginning
- Consider modular structure for better navigability
- Optional but encouraged

**Not needed for documents <300 lines:**
- Single file acceptable
- Quick reference guides, templates, README files

---

## Template Files

1. **00_SUMMARY.md** - Executive summary template (target: <500 lines)
2. **01_section_template.md** - Section file template (target: <300 lines each)

---

## How to Use

### For New Large Documents (>1200 lines)

**Step 1:** Create folder structure
```bash
mkdir -p docs/[DOCUMENT_NAME]
cd docs/[DOCUMENT_NAME]
```

**Step 2:** Copy templates
```bash
cp templates/modular_document/00_SUMMARY.md ./00_SUMMARY.md
cp templates/modular_document/01_section_template.md ./01_topic1.md
cp templates/modular_document/01_section_template.md ./02_topic2.md
# ... add more sections as needed
```

**Step 3:** Customize
- Edit 00_SUMMARY.md: Replace placeholders, add navigation links
- Edit section files: One topic per file, <300 lines each
- Create FULL_DOCUMENT.md (consolidated, optional - for archival/search)

**Step 4:** Link sections
- Summary links to detailed sections
- Each section references back to summary
- Cross-reference related sections

### For Medium Documents (300-1200 lines)

**Option A - Single file with summary:**
- Add executive summary section at top (<300 lines)
- Follow with detailed sections
- Use clear section headers

**Option B - Modular structure (recommended):**
- Same as large documents above
- Better for documents approaching 1200 lines
- Easier to maintain and navigate

---

## Folder Naming Convention

**Format:** `[DOC-ID]_[descriptive_name]/`

**Examples:**
- `SPEC008_library_management/`
- `IMPL005_testing_strategy/`
- `RPT012_performance_analysis/`

**Contents:**
```
SPEC008_library_management/
├── 00_SUMMARY.md              # <500 lines - READ THIS FIRST
├── 01_requirements.md          # <300 lines
├── 02_architecture.md          # <300 lines
├── 03_api_design.md            # <300 lines
├── 04_implementation.md        # <300 lines
├── 05_testing.md               # <300 lines
└── FULL_DOCUMENT.md            # Consolidated (archival only)
```

---

## Section File Naming

**Pattern:** `[NN]_[topic_name].md`

**NN:** Sequential number (01, 02, 03, ...)
**topic_name:** Lowercase with underscores

**Examples:**
- 01_requirements.md
- 02_architecture.md
- 03_implementation_approach.md
- 04_testing_strategy.md

**Special files:**
- 00_SUMMARY.md (always first)
- FULL_DOCUMENT.md (consolidated, optional)

---

## Size Targets

| File Type | Target Size | Purpose |
|-----------|-------------|---------|
| 00_SUMMARY.md | <500 lines | Quick overview, navigation |
| Section files | <300 lines | Focused, self-contained topics |
| FULL_DOCUMENT.md | Any size | Search, archival, comprehensive review |

**Why these targets?**
- **<500 lines:** Fits in single AI context window read
- **<300 lines:** Comfortable reading session (5-10 minutes)
- **Total modular load:** Summary + 1-2 sections = ~600-1100 lines (optimal)

---

## Navigation Guidelines

### In 00_SUMMARY.md

**Provide:**
- Document map (what's in each section)
- Quick reference (key points from all sections)
- Reading guidance (what to read when)

**Example:**
```markdown
## Document Map

**For Quick Overview:**
- Read this summary only (~400 lines)

**For Specific Topics:**
- Requirements: [01_requirements.md](01_requirements.md) (~250 lines)
- Architecture: [02_architecture.md](02_architecture.md) (~280 lines)

**For Complete Context:**
- Full document: [FULL_DOCUMENT.md](FULL_DOCUMENT.md) (1500 lines)
```

### In Section Files

**Provide:**
- Brief context (how this section relates to whole)
- Link back to summary
- Cross-references to related sections
- Navigation (previous/next section)

**Example:**
```markdown
← [00_SUMMARY.md](00_SUMMARY.md) | [02_architecture.md](02_architecture.md) →

**Context:** This section covers requirements (part of SPEC008 Library Management)
**See also:** [03_implementation.md](03_implementation.md) for how requirements are satisfied
```

---

## Best Practices

**Do:**
- ✅ Keep sections focused (one topic per file)
- ✅ Use descriptive section names (not "section1", "section2")
- ✅ Provide navigation in summary and sections
- ✅ Test readability (summary alone should give clear overview)
- ✅ Cross-reference related content (links, not duplication)

**Don't:**
- ❌ Exceed size targets without good reason
- ❌ Duplicate content across sections (link instead)
- ❌ Create tiny sections (<50 lines - combine related topics)
- ❌ Omit executive summary (defeats purpose of modular structure)
- ❌ Make sections depend on reading order (each should be self-contained)

---

## Conversion from Monolithic Documents

If converting existing large document to modular structure:

1. **Analyze:** Identify natural section boundaries
2. **Extract:** Create executive summary (<500 lines) covering all sections
3. **Modularize:** Split into section files (<300 lines each)
4. **Link:** Add navigation between summary and sections
5. **Consolidate:** Create FULL_DOCUMENT.md (optional, for search/archive)
6. **Verify:** Ensure no information lost, all references updated
7. **Test:** Verify AI can navigate new structure
8. **Commit:** Use `/commit` workflow to track changes

**Effort estimate:** 3-4 hours per document

---

## Testing Usability

After creating modular structure:

**AI Test:**
- Can AI navigate to specific information using summary?
- Can AI answer questions using only summary + 1 section?
- Does modular structure reduce context window usage?

**Human Test:**
- Can reader understand document from summary alone?
- Can reader find specific information quickly?
- Is reading experience better than monolithic version?

**Metrics:**
- Time to find specific information (should decrease)
- Context window usage (should be ~500-1000 lines, not 2000+)
- User feedback (should prefer modular over monolithic)

---

## Examples

**Good modular documents:**
- [wip/PLAN003_context_engineering/](../../wip/PLAN003_context_engineering/) - Implementation plan
- [.claude/commands/think.md](.../.claude/commands/think.md) - Analysis workflow (lines 558-625)
- [.claude/commands/plan.md](.../.claude/commands/plan.md) - Planning workflow (lines 454-493)

**Documents to potentially convert:**
- docs/GOV001-document_hierarchy.md (997 lines) → GOV001/ folder
- docs/SPEC001-architecture.md (if >1200 lines) → SPEC001/ folder

---

## Questions?

**See:**
- CLAUDE.md "Documentation Reading Protocol"
- GOV001 "Document Size and Structure Standards" (once Phase 2 complete)
- PLAN003 for rationale and research backing

**Contact:** Technical lead or documentation lead for guidance

---

**Version:** 1.0
**Created:** 2025-10-25
**Part of:** PLAN003 Context Engineering Implementation (REQ-CE-P1-100)
