# Context Engineering Quick Reference Card

**WKMP Development Team** | Version 1.0 | 2025-10-25

---

## `/plan` Workflow Decision Tree

```
┌─────────────────────────────────────┐
│ Starting new feature/enhancement?   │
└─────────────┬───────────────────────┘
              │
              ▼
      ┌───────────────┐
      │ >5 requirements? │────YES────┐
      └───────┬──────────┘            │
              │                        │
             NO                        │
              │                        │
              ▼                        ▼
      ┌──────────────┐          ┌──────────────┐
      │ Novel/complex? │──YES──▶│ USE /plan    │
      └───────┬────────┘         │   MANDATORY  │
              │                   └──────────────┘
             NO
              │
              ▼
      ┌──────────────┐
      │ OPTIONAL     │
      │ (recommended)│
      └──────────────┘
```

**Novel:** No similar feature in codebase
**Complex:** Touches 3+ modules OR 5+ files

---

## Verbosity: 5 Key Techniques

1. **Bullet points** instead of paragraphs
2. **One concept per sentence** - avoid run-ons
3. **Link to existing docs** - don't repeat content
4. **Omit unnecessary details** - focus on essentials
5. **Use tables** for structured data

**Target:** 20-40% shorter than first draft

---

## Document Size Thresholds

| Lines | Structure | Requirements |
|-------|-----------|--------------|
| <300 | Single file | None |
| 300-1200 | Single file | Executive summary recommended |
| >1200 | **Modular folder** | **MANDATORY** |

**Modular Structure:**
```
DOC_NAME/
├── 00_SUMMARY.md       # <500 lines - READ THIS FIRST
├── 01_section.md       # <300 lines each
├── 02_section.md
└── FULL_DOCUMENT.md    # Archival only
```

**Tools:** `/doc-name` checks size automatically

---

## Summary-First Reading (4 Steps)

1. **Start with summary**
   - 00_SUMMARY.md OR first 50-100 lines

2. **Identify relevant sections**
   - Use navigation map in summary

3. **Read targeted sections only**
   - NOT entire document

4. **Load full spec ONLY if necessary**
   - Archival/comprehensive review only

**Applies to:** AI assistants AND human reviewers

---

## Templates & Resources

**Templates:**
- `templates/modular_document/README.md`
- `templates/modular_document/00_SUMMARY.md`
- `templates/modular_document/01_section_template.md`

**Standards:**
- `CLAUDE.md` - Global AI instructions (3 new sections)
- `docs/GOV001-document_hierarchy.md` (v1.6) - Size/structure standards

**Workflows:**
- `.claude/commands/plan.md` - `/plan` workflow
- `.claude/commands/doc-name.md` - Document naming/sizing
- `.claude/commands/commit.md` - Commit with size targets

**Workshop Materials:**
- `project_management/workshop_materials/plan_workshop/`
- `project_management/workshop_materials/context_engineering_education/`

---

## Workflow Commands

| Command | When to Use |
|---------|-------------|
| `/plan [spec_file]` | Before implementing features (>5 req or novel/complex) |
| `/doc-name [file]` | When creating new documentation |
| `/commit` | When committing changes (enforces size targets) |
| `/think [topic]` | Before major architectural decisions |
| `/archive [doc]` | When completing work (clean context) |

---

## Common Scenarios

### Scenario: "Should I use `/plan` for this 3-requirement bug fix?"
**Answer:** Optional (but recommended for complex bugs touching multiple modules)

### Scenario: "My document is 800 lines. Do I need modular structure?"
**Answer:** Optional - single file OK, but add executive summary

### Scenario: "How do I decide what goes in the summary?"
**Answer:** Problem statement + key decisions + navigation map (~300-500 lines)

### Scenario: "Can I edit the tests `/plan` generates?"
**Answer:** YES - they're starting points, refine as needed, maintain traceability

### Scenario: "`/plan` found CRITICAL issue - now what?"
**Answer:** MUST resolve before implementing - update spec or clarify requirements

---

## Quick Checklist: Creating New Documentation

- [ ] Run `/doc-name [file]` to assign category and check size
- [ ] If >1200 lines: Use modular folder structure
- [ ] If 300-1200 lines: Add executive summary section
- [ ] Apply verbosity techniques (20-40% reduction target)
- [ ] Include navigation map (for modular docs)
- [ ] Use templates from `templates/modular_document/`
- [ ] Link to existing docs (don't repeat content)
- [ ] Commit with `/commit` (enforces standards)

---

## Questions or Feedback?

**Contact:** [Technical Lead Name/Email]
**Team Channel:** [Slack/Teams Channel]
**Office Hours:** [If available]

**Metrics Tracking:** Volunteer to help measure effectiveness:
- Document size trends
- `/plan` usage adoption
- Team satisfaction feedback

---

**Keep this card handy!** Use it every time you create documentation or plan a feature.

**Version:** 1.0 | **Date:** 2025-10-25 | **Related:** PLAN003, GOV001 v1.6
