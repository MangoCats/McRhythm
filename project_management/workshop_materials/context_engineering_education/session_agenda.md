# Context Engineering Standards - Team Education Session

**Duration:** 2 hours
**Audience:** All WKMP developers and technical writers
**Facilitator:** Technical Lead
**Date:** TBD (Phase 2, Week 4)
**Prerequisites:** None (introductory session)

---

## Session Objectives

By the end of this session, participants will:
1. Understand why context engineering matters for WKMP development
2. Know when and how to use the `/plan` workflow
3. Apply verbosity standards to document generation
4. Use summary-first reading patterns for all documentation
5. Choose appropriate document structures (single file vs. modular)
6. Locate and use templates for modular documents

---

## Agenda Overview

| Time | Topic | Format | Duration |
|------|-------|--------|----------|
| 0:00-0:15 | Introduction: The Context Problem | Presentation | 15 min |
| 0:15-0:45 | Part 1: `/plan` Workflow Deep Dive | Demo + Q&A | 30 min |
| 0:45-1:15 | Part 2: Document Standards | Interactive | 30 min |
| 1:15-1:30 | Break | - | 15 min |
| 1:30-1:50 | Part 3: Hands-On Practice | Group Exercise | 20 min |
| 1:50-2:00 | Wrap-Up: Resources & Next Steps | Discussion | 10 min |

---

## Detailed Agenda

### Introduction: The Context Problem (0:00-0:15)

**Objective:** Establish shared understanding of why we're implementing these standards

**Content:**
- **The Research** (5 min)
  - "Most agent failures are context failures" (6 research sources)
  - Context window limitations affect both AI and human readers
  - WKMP's growing documentation: challenges at scale

- **Our Two Problems** (5 min)
  - Problem 1: AI implementations overlooking specifications
  - Problem 2: AI-generated documents too verbose (bloated)
  - Real examples from WKMP project (anonymized)

- **The Solution: Four Interventions** (5 min)
  - Quick overview of Phase 1 interventions (details in Parts 1-2)
  - Expected impact: 20-40% reduction in doc size, proactive spec verification

**Materials:**
- Slides with research citations
- Before/after examples of verbose vs. concise docs
- PLAN003 summary (handout)

---

### Part 1: `/plan` Workflow Deep Dive (0:15-0:45)

**Objective:** Participants can confidently use `/plan` workflow for their next feature

**Content:**

#### When to Use `/plan` (5 min)
- **MANDATORY for:** Features with >5 requirements
- **MANDATORY for:** Novel/complex features (even if <5 requirements)
- **Optional for:** Small enhancements, bug fixes, refactoring

**Decision Tree:**
```
Does feature have >5 requirements? → YES → Use /plan
Is feature novel/complex? → YES → Use /plan
Otherwise → Optional (but recommended)
```

#### Live Demo: Running `/plan` (15 min)
- **Step 1:** Identify specification document
- **Step 2:** Run `/plan [spec_document]` command
- **Step 3:** Review three outputs:
  - Requirements extraction
  - Specification issue detection (CRITICAL/HIGH/MEDIUM/LOW)
  - Acceptance test generation (with traceability matrix)
- **Step 4:** Resolve CRITICAL issues before implementing
- **Step 5:** Use tests during development

**Demo Example:** Use workshop example specification (User Settings Export/Import)

#### Q&A and Discussion (10 min)
- "What if I find issues during implementation?"
- "How do I update tests when requirements change?"
- "Can I skip tests for prototypes?"

**Materials:**
- Live terminal demo
- Example `/plan` output (printed handout)
- CLAUDE.md excerpt on `/plan` policy

---

### Part 2: Document Standards (0:45-1:15)

**Objective:** Participants know how to write concise, well-structured documents

**Content:**

#### Verbosity Standards (15 min)

**The Target:** 20-40% shorter than comprehensive first draft

**Techniques:**
1. **Bullet points** instead of paragraphs
2. **One concept per sentence** - avoid run-ons
3. **Link to existing docs** instead of repeating
4. **Omit unnecessary details** - focus on essentials
5. **Use tables** for structured data

**Interactive Exercise:** Participants edit a verbose paragraph (3 min individual, 2 min share-out)

**Before Example:**
```
The crossfade feature is designed to provide a smooth transition between
two audio passages by gradually decreasing the volume of the currently
playing passage while simultaneously increasing the volume of the next
passage, creating a seamless listening experience that avoids abrupt
transitions which can be jarring to listeners.
```

**After Example:**
```
Crossfade creates smooth transitions by:
- Fading out current passage volume
- Fading in next passage volume simultaneously
- Avoiding abrupt cuts
```

#### Document Size Thresholds (10 min)

**Three Tiers:**
1. **<300 lines:** Single file, no special requirements
2. **300-1200 lines:** Single file acceptable, executive summary recommended
3. **>1200 lines:** MANDATORY modular folder structure

**Modular Structure:**
```
SPEC018_advanced_feature/
├── 00_SUMMARY.md          # <500 lines - READ THIS FIRST
├── 01_architecture.md     # <300 lines per section
├── 02_api_design.md
├── 03_algorithms.md
└── FULL_DOCUMENT.md       # Consolidated (archival only)
```

**When to Use:** `/doc-name` workflow checks size and recommends structure

#### Summary-First Reading Protocol (5 min)

**MANDATORY Reading Pattern:**
1. **Always start with summary** (00_SUMMARY.md or first 50-100 lines)
2. **Identify relevant sections** from summary
3. **Read targeted sections** only (not entire document)
4. **Never load full specifications** into context unless necessary

**For AI Assistants:** This is now in CLAUDE.md as global instruction

**For Human Reviewers:** Same pattern improves review efficiency

**Materials:**
- Handout: "Verbose vs. Concise Quick Reference"
- Printed examples of modular document structures
- Templates folder walkthrough

---

### Break (1:15-1:30)

---

### Part 3: Hands-On Practice (1:30-1:50)

**Objective:** Apply learned skills in realistic scenarios

**Exercise Setup:**
- Form groups of 2-3 participants
- Each group gets one scenario card
- 15 minutes to complete, 5 minutes share-out

**Scenario 1: Plan or No Plan?**
- Given: Feature description with 7 requirements
- Task: Decide whether `/plan` is needed, explain reasoning
- Bonus: What would `/plan` outputs look like?

**Scenario 2: Reduce Verbosity**
- Given: Verbose design section (~200 words)
- Task: Rewrite to meet verbosity standards (<80 words, 60% reduction)
- Constraint: Maintain all essential information

**Scenario 3: Structure Decision**
- Given: Document outline with estimated line counts per section
- Task: Choose single file vs. modular structure
- Output: Recommend folder structure if modular

**Share-Out:**
- Each group presents their solution (2 min each)
- Facilitator provides feedback and corrections

**Materials:**
- Scenario cards (printed)
- Blank worksheets for answers

---

### Wrap-Up: Resources & Next Steps (1:50-2:00)

**Resources Available:**
- **CLAUDE.md** - Global AI instructions (3 new sections)
- **GOV001** - Document Size and Structure Standards
- **Templates** - [templates/modular_document/](../../templates/modular_document/)
- **Workflows** - All 6 workflows updated with size targets
- **Workshop Materials** - `/plan` workshop agenda and facilitator guide

**Quick Reference Card:**
Participants receive 1-page reference card with:
- `/plan` decision tree
- Verbosity techniques (5 key points)
- Size thresholds (3 tiers)
- Summary-first reading steps
- Template locations

**Next Steps:**
1. **This Week:** Review updated CLAUDE.md sections
2. **Next Feature:** Try `/plan` workflow (even if optional)
3. **Ongoing:** Apply verbosity standards to all new documents
4. **Questions:** Contact technical lead or post in team channel

**Feedback Collection:**
- Exit survey (2 minutes, paper forms)
- Optional: Join metrics collection volunteer group

---

## Facilitator Preparation

### Before Session (1 week prior):
- [ ] Reserve conference room with projector
- [ ] Print handouts (20 copies each):
  - PLAN003 summary
  - Example `/plan` output
  - Verbose vs. Concise reference
  - Scenario cards (3 types x 7 groups = 21 cards)
  - Quick reference cards (1 per participant)
- [ ] Set up demo environment with example specification
- [ ] Prepare slides (10-12 slides max)
- [ ] Test `/plan` workflow run-through

### During Session:
- [ ] Arrive 15 minutes early for setup
- [ ] Check projector and terminal font size
- [ ] Distribute handouts at appropriate times (not all at start)
- [ ] Monitor time closely (use timer for each section)
- [ ] Encourage questions but defer deep dives to end

### After Session:
- [ ] Collect exit surveys
- [ ] Compile feedback summary (within 48 hours)
- [ ] Share slides and materials in team repository
- [ ] Send follow-up email with resources and Q&A summary
- [ ] Update metrics tracking with attendance and satisfaction scores

---

## Success Metrics

**Attendance:** ≥75% of development team (target from PLAN003)

**Exit Survey Targets:**
- ≥80% understand when to use `/plan`
- ≥80% feel confident applying verbosity standards
- ≥70% can choose appropriate document structure
- ≥4.0/5.0 average session rating

**Post-Session Metrics** (tracked over 4 weeks):
- ≥3 features planned with `/plan` workflow
- ≥20% reduction in average new document size
- ≥90% of new docs >1200 lines use modular structure

---

## Backup Plans

**If attendance is low (<50%):**
- Reschedule for better time
- Offer recorded session option
- Provide self-study materials

**If session runs long:**
- Abbreviate hands-on practice (10 min instead of 20)
- Skip break or shorten to 10 minutes
- Defer advanced Q&A to follow-up session

**If technical issues:**
- Have printed `/plan` output as backup for live demo
- Use slide screenshots instead of live terminal
- Provide demo video link for later viewing

---

**Version:** 1.0
**Date Created:** 2025-10-25
**Related:** PLAN003, GOV001 v1.6, project_management/workshop_materials/plan_workshop/
