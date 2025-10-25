# `/plan` Workflow Training Workshop
## 2-Hour Interactive Training Session

**Duration:** 2 hours
**Target Audience:** All developers working on WKMP
**Prerequisites:** Familiarity with Claude Code custom commands
**Format:** Interactive presentation + hands-on practice

---

## Learning Objectives

By the end of this workshop, participants will be able to:

1. **Explain** the purpose and benefits of the `/plan` workflow
2. **Identify** when to use `/plan` vs. direct implementation
3. **Create** a specification-driven implementation plan using `/plan`
4. **Define** appropriate test cases for verification
5. **Apply** the `/plan` workflow to their next feature implementation

---

## Agenda Overview

| Time | Duration | Section | Format |
|------|----------|---------|--------|
| 0:00 | 45 min | Part 1: `/plan` Overview | Presentation + Demo + Q&A |
| 0:45 | 45 min | Part 2: Hands-On Practice | Group Exercise |
| 1:30 | 30 min | Part 3: Integration & Next Steps | Discussion + Planning |

---

## Part 1: `/plan` Overview (45 minutes)

### Introduction (10 minutes)
**Learning Goal:** Understand the "why" behind `/plan`

- **Problem Statement** (3 min)
  - Challenges with ad-hoc implementation
  - Specification drift and missed requirements
  - Difficulty verifying completeness

- **The `/plan` Solution** (7 min)
  - Specification-driven development approach
  - Automatic test case generation
  - Built-in verification checkpoints
  - Integration with WKMP documentation hierarchy

### Workflow Walkthrough (20 minutes)
**Learning Goal:** Understand the complete `/plan` process

#### Phase 1: Input Specification (5 min)
- What makes a good specification document?
- Where to find specifications in WKMP (docs/ hierarchy)
- How `/plan` extracts requirements from specs

#### Phase 2: Implementation Planning (5 min)
- Multi-agent analysis approach
- Task decomposition and sequencing
- Test-first planning methodology
- Output format: `wip/PLAN###_feature_name.md`

#### Phase 3: Execution and Verification (5 min)
- Step-by-step task execution
- Running verification tests
- Updating specification documents
- Archiving completed plans

#### Integration with Other Workflows (5 min)
- Using `/think` before `/plan` for complex features
- Using `/commit` after `/plan` completion
- Using `/archive-plan` for batch cleanup
- Relationship to change_history.md tracking

### Live Demonstration (10 minutes)
**Learning Goal:** See `/plan` in action

- **Demo Feature:** Add "Clear Queue" button to Audio Player UI
- Walk through complete workflow:
  1. Review simple specification (5 requirements)
  2. Execute `/plan` command with spec path
  3. Review generated plan structure
  4. Show test case examples
  5. Execute first 2-3 tasks
  6. Run verification test

**Key Observations to Highlight:**
- Automatic requirement extraction
- Test-first approach
- Clear task sequencing
- Verification checkpoints

### Q&A Session (5 minutes)
**Learning Goal:** Address concerns and clarify confusion

Common questions to anticipate:
- "How long does `/plan` take compared to direct coding?"
- "What if the specification is incomplete?"
- "Can I modify the plan during execution?"
- "What happens if tests fail?"

---

## Part 2: Hands-On Practice (45 minutes)

### Setup (5 minutes)

**Facilitator Actions:**
- Distribute example_specification.md
- Confirm all participants have Claude Code access
- Review practice objective

**Participant Task:**
- Open provided specification: "User Settings Export/Import Feature"
- Review 8 requirements briefly
- Identify 2-3 key challenges

### Exercise: Create Implementation Plan (25 minutes)

**Objective:** Use `/plan` to create an implementation plan for the example specification

**Instructions:**

1. **Execute `/plan` command** (5 min)
   - Navigate to workshop materials folder
   - Run: `/plan project_management/workshop_materials/plan_workshop/example_specification.md`
   - Wait for plan generation

2. **Review Generated Plan** (10 min)
   - Examine task breakdown
   - Review test cases for each requirement
   - Identify verification checkpoints
   - Note: Look for requirement IDs (REQ-SE-010, etc.)

3. **Critical Analysis** (10 min)
   Answer these questions:
   - Are all 8 requirements covered?
   - Are test cases sufficient?
   - Is task sequencing logical?
   - What would you modify?

**Facilitator Notes:**
- Circulate to answer questions
- Note common issues for group discussion
- Prepare to show your own generated plan as reference

### Group Discussion (15 minutes)

**Share Findings** (10 min)
- What worked well?
- What challenges did you encounter?
- Did `/plan` catch any missing requirements?
- How did test cases help clarify requirements?

**Best Practices** (5 min)
Facilitator highlights:
- Writing clear, enumerated specifications
- When to break large features into multiple plans
- Handling specification ambiguities
- Balancing plan detail with flexibility

---

## Part 3: Integration and Next Steps (30 minutes)

### Mandatory Usage Policy (10 minutes)

**Effective Date:** [Date of workshop]

**Policy Statement:**
All non-trivial features and enhancements **must** use `/plan` workflow before implementation.

**Definition - "Non-Trivial":**
Use `/plan` when ANY of these conditions apply:
- Feature involves 3+ files
- Feature implements 5+ requirements
- Feature adds new API endpoints
- Feature modifies database schema
- Feature requires cross-module coordination
- You're unsure about implementation approach

**Exemptions:**
Direct implementation allowed for:
- Bug fixes (single-issue resolution)
- Documentation-only changes
- Trivial UI text/styling updates
- Code cleanup/refactoring (no behavior change)

**When in doubt:** Use `/plan`. 15 minutes of planning saves hours of rework.

### Pilot Program (10 minutes)

**Goal:** Validate workflow with real work before full rollout

**Pilot Selection:**
- Facilitator selects 2-3 upcoming features from backlog
- Features should be:
  - Non-trivial (require `/plan`)
  - Well-specified (specs already exist)
  - Scheduled for next 2 weeks

**Pilot Participants:**
- Volunteers (2-3 developers)
- Commitment: Use `/plan` + provide feedback

**Feedback Collection:**
- Daily: Quick check-in (blockers? issues?)
- End-of-pilot: 30-min retrospective
- Metrics: Time spent, issues found, value delivered

**Timeline:**
- Week 1-2: Pilot execution
- Week 3: Retrospective and policy refinement
- Week 4: Full team rollout

### Action Items and Commitments (10 minutes)

**Facilitator Actions:**

1. **Assign Pilot Features** (3 min)
   - Match volunteers to upcoming features
   - Confirm specification documents exist
   - Set pilot start date

2. **Review Support Resources** (3 min)
   - Documentation: `.claude/commands/plan.md`
   - Examples: Completed plans in `wip/` (if any)
   - Help channel: [Specify team communication channel]
   - Office hours: [Schedule weekly 30-min Q&A session]

3. **Collect Exit Survey** (4 min)
   - Distribute survey (see attendance_sheet.md)
   - Allow 2-3 minutes for completion
   - Optional: Quick verbal feedback round-robin

**Participant Commitments:**
- Use `/plan` for next eligible feature
- Provide feedback after first use
- Help colleagues during adoption phase
- Report issues/improvements to facilitator

---

## Materials Provided

1. **This Agenda** - Workshop structure and learning objectives
2. **Facilitator Guide** - Step-by-step facilitation instructions
3. **Example Specification** - Practice exercise specification document
4. **Attendance Sheet** - Sign-in and exit survey

---

## Post-Workshop Resources

**Documentation:**
- `.claude/commands/plan.md` - Complete `/plan` command reference
- `workflows/DWI001_workflow_quickstart.md` - All workflow quick start
- `docs/GOV001-document_hierarchy.md` - Specification hierarchy reference

**Support:**
- Office Hours: [Schedule recurring time slot]
- Help Channel: [Team communication channel]
- Feedback Form: [Link to ongoing feedback collection]

---

## Workshop Success Metrics

**Immediate (Day 1):**
- 100% attendance
- 90%+ positive survey responses
- 3+ pilot volunteers

**Short-term (Week 4):**
- All pilot features completed using `/plan`
- Pilot feedback collected and reviewed
- Policy refinements implemented

**Long-term (Month 3):**
- 80%+ of eligible features use `/plan`
- 50% reduction in specification drift issues
- Measurable improvement in requirement traceability

---

**Questions?** Contact [Facilitator Name/Email]
