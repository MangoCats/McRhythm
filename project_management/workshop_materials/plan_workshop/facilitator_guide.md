# `/plan` Workflow Training - Facilitator Guide

**Purpose:** Step-by-step instructions for delivering the 2-hour `/plan` workflow training workshop

**Target Audience:** Workshop facilitator (technical lead or senior developer)

---

## Pre-Workshop Preparation (1-2 days before)

### Materials Checklist

- [ ] Review all workshop materials:
  - [ ] workshop_agenda.md
  - [ ] example_specification.md
  - [ ] attendance_sheet.md (print 2 copies)
  - [ ] This facilitator guide

- [ ] Technical setup:
  - [ ] Test `/plan` command on your system
  - [ ] Generate plan for example_specification.md (save as reference)
  - [ ] Prepare demo feature spec (Clear Queue button - see below)
  - [ ] Test screen sharing (presentation + terminal)

- [ ] Room setup (if in-person):
  - [ ] Projector/screen working
  - [ ] Whiteboard + markers available
  - [ ] Seating for 15-20 people
  - [ ] Power outlets accessible

- [ ] Communication:
  - [ ] Send calendar invite (2 hours, mandatory)
  - [ ] Attach workshop_agenda.md
  - [ ] Include "Prerequisites: Familiarity with Claude Code"
  - [ ] Reminder email 24 hours before

### Demo Preparation: "Clear Queue" Feature

**Create this simple spec for live demo:**

Save as `wip/demo_clear_queue_spec.md`:

```markdown
# Clear Queue Button - UI Feature Specification

## Purpose
Add a button to the Audio Player UI that clears all queued passages (except currently playing).

## Requirements

### REQ-CQ-010: Button Placement
The "Clear Queue" button must appear in the queue management section, adjacent to existing queue controls.

### REQ-CQ-020: Button Behavior
When clicked, the button must:
1. Send DELETE request to `/api/queue` endpoint
2. Preserve currently playing passage
3. Remove all other queued passages
4. Update UI to reflect empty queue

### REQ-CQ-030: User Confirmation
The button must prompt for confirmation before clearing (prevent accidental clicks).

### REQ-CQ-040: Disabled State
The button must be disabled when queue is empty or has only one passage.

### REQ-CQ-050: API Integration
Audio Player module must implement `DELETE /api/queue` endpoint that:
- Returns 200 OK on success
- Returns current queue state after clear
- Logs clear action to audit trail

## Success Criteria
- User can clear queue in 2 clicks (button + confirm)
- No way to accidentally clear queue
- UI updates immediately after clear
```

**Practice the demo:**
1. Show the spec (30 seconds)
2. Run `/plan wip/demo_clear_queue_spec.md` (1 minute)
3. Review generated plan (3 minutes)
4. Execute first 2 tasks (5 minutes)
5. Show verification approach (30 seconds)

---

## Workshop Delivery Guide

### Opening (0:00 - 0:05) - 5 minutes

**Welcome and Context**

1. **Welcome participants** (1 min)
   - Thank everyone for attending
   - State importance: "This workflow will become mandatory for all non-trivial features"
   - Set expectation: Interactive session with hands-on practice

2. **Review agenda** (2 min)
   - Point to workshop_agenda.md
   - Highlight three parts: Overview, Practice, Integration
   - Emphasize hands-on component: "You'll create a real plan in Part 2"

3. **Pass attendance sheet** (1 min)
   - Distribute attendance_sheet.md
   - "Please sign in and we'll complete the survey at the end"

4. **Learning objectives** (1 min)
   - Read 5 objectives from agenda
   - "By end of session, you'll be ready to use `/plan` on your next feature"

**Facilitator Tips:**
- Arrive 10 minutes early for setup
- Start on time (respect schedules)
- Energy and enthusiasm set the tone
- Make eye contact, scan for confusion

---

### Part 1: `/plan` Overview (0:05 - 0:50) - 45 minutes

#### Section 1.1: Introduction (0:05 - 0:15) - 10 minutes

**Problem Statement (3 minutes)**

Ask participants: "What challenges have you experienced implementing features without a plan?"

Expected responses (capture on whiteboard):
- Missed requirements
- Unclear scope
- Difficult to know when "done"
- Specification drift
- Hard to verify completeness

**Your talking points:**
- "These are universal problems in software development"
- "Ad-hoc implementation works for tiny features, fails for complex ones"
- "We need a systematic approach for anything non-trivial"

**The `/plan` Solution (7 minutes)**

Present four key benefits:

1. **Specification-Driven** (2 min)
   - "Start with written requirements"
   - "Plan extracts and organizes them"
   - "Ensures nothing is missed"

2. **Test-First Approach** (2 min)
   - "Generates test cases for each requirement"
   - "Forces clarity: how will we verify this?"
   - "Tests written before code"

3. **Built-in Verification** (2 min)
   - "Checkpoints throughout implementation"
   - "Clear definition of 'done'"
   - "Traceability to requirements"

4. **Integration with WKMP Docs** (1 min)
   - "Works with our 5-tier documentation hierarchy"
   - "Maintains requirement IDs (REQ-XX-###)"
   - "Updates specs after completion"

**Facilitator Tips:**
- Use whiteboard for visual notes
- Relate to WKMP's documentation hierarchy
- Don't go too deep yet (details come in walkthrough)

---

#### Section 1.2: Workflow Walkthrough (0:15 - 0:35) - 20 minutes

**Set context:** "Let me walk you through the complete `/plan` workflow, step by step."

**Phase 1: Input Specification (5 minutes)**

1. **What makes a good specification?** (2 min)

   Write on whiteboard:
   - Clear, enumerated requirements (REQ-XX-010, REQ-XX-020, etc.)
   - Testable statements ("must", "shall", "will")
   - Sufficient detail for implementation
   - No ambiguous language

   Show example from docs/:
   - Open `docs/REQ001-requirements.md` (if available)
   - Or open `example_specification.md`
   - Point out requirement enumeration

2. **Where to find specifications** (2 min)

   Navigate directory structure:
   ```
   docs/
     ├── REQ001-requirements.md (Tier 1)
     ├── SPEC00X-*.md (Tier 2)
     └── IMPL00X-*.md (Tier 3)
   ```

   Explain: "Higher tiers inform lower tiers. Start with most relevant spec."

3. **How `/plan` extracts requirements** (1 min)

   "The command reads your spec and identifies:
   - Enumerated requirements (REQ-XX-###)
   - Success criteria
   - Constraints and dependencies"

**Phase 2: Implementation Planning (5 minutes)**

1. **Multi-agent analysis** (1 min)

   "Claude Code uses multiple AI agents to analyze:
   - Task decomposition (what needs to be done)
   - Dependency ordering (what sequence)
   - Test requirements (how to verify)"

2. **Task decomposition** (2 min)

   Show typical plan structure:
   ```markdown
   ## Tasks
   1. Create database migration for X
   2. Implement data model in common/
   3. Add API endpoint in module Y
   4. Implement business logic
   5. Add UI components
   6. Write integration tests
   7. Update documentation
   ```

   Point out: "Notice test-first approach - tests are tasks, not afterthoughts"

3. **Output format** (2 min)

   "Plan saved to `wip/PLAN###_feature_name.md`"

   Show example plan structure (open your generated example):
   - Header with spec reference
   - Requirements list
   - Task breakdown
   - Test cases per requirement
   - Verification checklist

**Phase 3: Execution and Verification (5 minutes)**

1. **Step-by-step execution** (2 min)

   "After plan generated:
   1. Review plan for completeness
   2. Execute tasks in order
   3. Check off completed items
   4. Run verification tests as you go"

2. **Verification tests** (2 min)

   Show test case example:
   ```markdown
   ### REQ-XX-010: [Requirement text]
   **Test Case:**
   - Setup: [Preconditions]
   - Action: [What to do]
   - Expected: [What should happen]
   ```

   "Each requirement has explicit test - no ambiguity about 'done'"

3. **After completion** (1 min)

   "When all tasks done:
   - Update original spec with implementation notes
   - Use `/archive-plan` to move to archive branch
   - Use `/commit` to track in change_history.md"

**Integration with Other Workflows (5 minutes)**

Create a workflow diagram on whiteboard:

```
Complex Question?
     ↓
  /think (analysis)
     ↓
Non-trivial Feature?
     ↓
  /plan (this workshop)
     ↓
Implementation
     ↓
  /commit (track changes)
     ↓
  /archive-plan (cleanup)
```

Explain each:
- **/think:** Use before `/plan` for architectural decisions
- **/plan:** Today's focus - specification to implementation
- **/commit:** Maintains change_history.md automatically
- **/archive-plan:** Moves completed plans to archive branch

"These workflows integrate seamlessly - each has specific purpose"

**Facilitator Tips:**
- Use screen share for directory navigation
- Keep pace moving (don't get bogged down)
- Promise "You'll do this yourself in Part 2"
- Check for questions with body language

---

#### Section 1.3: Live Demonstration (0:35 - 0:45) - 10 minutes

**Set expectation:** "Let me show you `/plan` in action with a simple feature."

**Demo: Clear Queue Button (10 minutes)**

1. **Show specification** (1 min)
   - Open `wip/demo_clear_queue_spec.md`
   - Scroll through 5 requirements
   - "Simple feature - perfect for quick demo"

2. **Execute `/plan`** (1 min)
   - In terminal: `/plan wip/demo_clear_queue_spec.md`
   - "This takes 30-60 seconds - Claude analyzes spec"
   - While waiting: "It's reading requirements, identifying dependencies"
   - Plan appears: "Here's our generated plan"

3. **Review generated plan** (3 min)

   Walk through sections:
   - **Requirements list:** "Notice all 5 REQ-CQ-### items captured"
   - **Task breakdown:** "10 tasks - includes implementation AND tests"
   - **Test cases:** "Each requirement has verification test"
   - **Dependencies:** "Tasks ordered logically"

   Point out specific test:
   ```markdown
   REQ-CQ-030: User Confirmation
   Test: Click Clear Queue → Verify confirmation dialog appears
   ```

   "This is test-first - we define verification BEFORE writing code"

4. **Execute first 2-3 tasks** (4 min)

   Actually do the work:
   - Task 1: "Create UI button component"
     - Open relevant file
     - Add button code
     - Check off task in plan

   - Task 2: "Add confirmation dialog"
     - Show confirmation logic
     - Check off task

   "I won't complete all 10 tasks now, but you see the pattern"

5. **Show verification approach** (1 min)

   "After completing implementation tasks, I'd:
   1. Run test cases from plan
   2. Verify each requirement
   3. Update plan with results
   4. Mark plan complete when all pass"

**Facilitator Tips:**
- Practice this demo beforehand (smooth execution)
- If `/plan` fails, have pre-generated plan as backup
- Don't complete full implementation (time constraint)
- Emphasize test-first approach

---

#### Section 1.4: Q&A Session (0:45 - 0:50) - 5 minutes

**Invite questions:** "What questions do you have about `/plan` before we practice?"

**Common Questions and Answers:**

**Q: "How long does `/plan` take vs. direct coding?"**
A: "Planning: 5-10 minutes. But saves hours catching missed requirements. Net time savings for anything non-trivial."

**Q: "What if specification is incomplete?"**
A: "`/plan` will identify gaps! That's a feature. Fix spec, re-run `/plan`. Better to find gaps before coding."

**Q: "Can I modify plan during execution?"**
A: "Yes! Plans are markdown - edit freely. But document why you deviated. Might indicate spec needs update."

**Q: "What if tests fail?"**
A: "Don't mark task complete until tests pass. Failing tests mean implementation incomplete or requirement misunderstood."

**Q: "Is this overkill for small features?"**
A: "Use judgment. 2-3 file change with clear requirements? Maybe skip. 5+ requirements or cross-module? Always use `/plan`."

**Q: "Who reviews the plan?"**
A: "Self-review initially. For major features, consider peer review of plan before implementation."

**Facilitator Tips:**
- Write questions on whiteboard as asked
- Validate concerns ("Great question!")
- Relate answers to WKMP specifics
- If no questions: "Let me pose a common question I hear..."

---

### Part 2: Hands-On Practice (0:50 - 1:35) - 45 minutes

#### Section 2.1: Setup (0:50 - 0:55) - 5 minutes

**Transition:** "Now it's your turn! You'll use `/plan` with a practice specification."

**Distribute materials:**
1. Confirm everyone has `example_specification.md` open
2. Navigate everyone to: `project_management/workshop_materials/plan_workshop/`
3. "This is a realistic feature: User Settings Export/Import"

**Review practice objective:**
"You'll create a complete implementation plan for this specification. Focus on:
- Does `/plan` capture all requirements?
- Are test cases sufficient?
- Is task sequencing logical?
- What would you modify?"

**Quick specification scan (2 minutes):**
- Give everyone 2 minutes to read example_specification.md
- "Notice 8 requirements (REQ-SE-010 through REQ-SE-080)"
- "Think: What are 2-3 key challenges in this feature?"

**Facilitator actions:**
- Set visible timer for 2 minutes
- Watch for people who can't find file (provide help)
- Be ready to show your screen if confusion

---

#### Section 2.2: Exercise - Create Implementation Plan (0:55 - 1:20) - 25 minutes

**Step 1: Execute `/plan` (5 minutes)**

"Let's all execute the `/plan` command together."

**Guide participants:**
1. "Open terminal in project root"
2. "Type: `/plan project_management/workshop_materials/plan_workshop/example_specification.md`"
3. "Press Enter - takes 30-60 seconds"

**While waiting:**
- "Claude is analyzing requirements"
- "Identifying dependencies"
- "Generating test cases"

**When plans generate:**
- "Your plan is saved in `wip/PLAN###_user_settings_export_import.md`"
- "Open that file now"

**Facilitator actions:**
- Execute `/plan` yourself on shared screen
- Help anyone with errors (file path issues, etc.)
- Be patient - some may need technical help

---

**Step 2: Review Generated Plan (10 minutes)**

"Take 10 minutes to carefully review your generated plan."

**What to look for (write on whiteboard):**

1. **Requirements Coverage (3 min)**
   - Are all 8 REQ-SE-### items listed?
   - Any requirements missed?
   - Any extra requirements added?

2. **Test Cases (3 min)**
   - Does each requirement have test case?
   - Are tests specific and actionable?
   - Can you execute these tests?

3. **Task Sequencing (2 min)**
   - Does task order make sense?
   - Are dependencies respected?
   - Would you reorder anything?

4. **Overall Completeness (2 min)**
   - Could you implement from this plan?
   - What's unclear?
   - What would you add/remove?

**Facilitator actions:**
- Set 10-minute timer (visible countdown)
- Circulate if in-person (answer individual questions)
- Monitor chat if virtual (respond to questions)
- Note common issues for group discussion
- Pull up YOUR generated plan as reference

**Facilitator Tips:**
- Expect variation in plans (AI generates different approaches)
- Look for people struggling (offer help)
- Note particularly good observations for sharing

---

**Step 3: Critical Analysis (10 minutes)**

"Now let's analyze what `/plan` generated."

**Individual reflection (5 minutes):**
"Write brief answers to these questions:" (display on screen)

1. Are all 8 requirements covered?
2. Are test cases sufficient?
3. Is task sequencing logical?
4. What would you modify?

**Facilitator actions:**
- Give 5 minutes silent work time
- This is important: forces critical thinking
- Participants should write notes (not just think)

**Partner share (5 minutes):**
"Turn to person next to you - share your answers" (if in-person)
"Post in chat - share one observation" (if virtual)

**Facilitator actions:**
- Pair people if odd number
- Listen to conversations (gather insights for discussion)
- Note interesting observations for next section

---

#### Section 2.3: Group Discussion (1:20 - 1:35) - 15 minutes

**Share Findings (10 minutes)**

"Let's share what you discovered."

**Prompt discussion:**
"Who wants to share an insight about their generated plan?"

**Areas to cover (ensure all discussed):**

1. **What worked well?** (3 min)
   - Requirements captured correctly?
   - Test cases helpful?
   - Task breakdown logical?

   Collect 3-4 positive observations

   Facilitator affirm: "Yes! That's exactly the value - automatic requirement extraction saves time"

2. **Challenges encountered?** (3 min)
   - Confusion about task order?
   - Unclear test cases?
   - Missing context?

   Collect 2-3 challenges

   Facilitator validate: "Good catch - that's where you edit the plan. AI gives starting point, you refine."

3. **Did `/plan` catch missing requirements?** (2 min)
   - Anyone notice gaps in specification?
   - Did plan highlight dependencies?

   Facilitator emphasize: "This is huge win - finding gaps in planning phase vs. mid-implementation"

4. **How did test cases help?** (2 min)
   - Did tests clarify requirements?
   - Any requirements that need better tests?

   Facilitator: "Test-first forces precision - no vague requirements survive contact with 'How do we test this?'"

**Facilitator Tips:**
- Call on quiet participants ("Chris, what did you notice?")
- Capture key points on whiteboard
- Relate observations back to learning objectives
- Keep pace (10 minutes goes fast)

---

**Best Practices (5 minutes)**

"Based on our experience today, here are best practices for `/plan`:"

**1. Writing Clear Specifications (1.5 min)**
- Enumerate requirements (REQ-XX-010, REQ-XX-020, etc.)
- Use testable language ("must", "shall", "will")
- Provide sufficient context
- Include success criteria

**2. When to Break Features into Multiple Plans (1 min)**
- Guideline: 10-15 requirements per plan maximum
- If feature has 30+ requirements, create 2-3 plans
- Each plan should have clear completion criteria

**3. Handling Specification Ambiguities (1 min)**
- If `/plan` generates unclear tasks, spec probably unclear
- Don't start implementation - fix spec first
- Re-run `/plan` with improved spec

**4. Balancing Detail with Flexibility (1.5 min)**
- Plan is guide, not straitjacket
- Edit plan during implementation (document why)
- Discoveries during implementation may require spec updates
- That's OK! Better to find issues early than late

**Facilitator Tips:**
- These points are in workshop_agenda.md (participants can reference)
- Use examples from discussion ("As Sarah noticed...")
- Emphasize: Plans are living documents

---

### Part 3: Integration and Next Steps (1:35 - 2:00) - 25 minutes

#### Section 3.1: Mandatory Usage Policy (1:35 - 1:45) - 10 minutes

**Transition:** "You've seen `/plan` in action. Now let's talk about when and how we'll use it."

**Policy Announcement (3 minutes)**

Display on screen (clear, large text):

```
EFFECTIVE [Workshop Date]:
All non-trivial features MUST use `/plan` before implementation
```

**Explain rationale:**
"Why mandatory? Because we've seen the problems:
- Specification drift costs us days of rework
- Missed requirements discovered late in testing
- Unclear 'done' criteria causing scope creep
- This workflow solves those problems systematically"

**Definition: "Non-Trivial" (5 minutes)**

"When MUST you use `/plan`?" (write on whiteboard)

**Use `/plan` when ANY of these apply:**
- ✓ Feature involves 3+ files
- ✓ Feature implements 5+ requirements
- ✓ Feature adds new API endpoints
- ✓ Feature modifies database schema
- ✓ Feature requires cross-module coordination
- ✓ You're unsure about implementation approach

**Direct implementation OK for:**
- ✗ Bug fixes (single-issue resolution)
- ✗ Documentation-only changes
- ✗ Trivial UI text/styling updates
- ✗ Code cleanup/refactoring (no behavior change)

**Key guidance:**
"When in doubt, use `/plan`. 15 minutes of planning saves hours of rework."

**Take questions (2 minutes):**
- "What counts as a bug fix vs. feature?"
  - Bug: Restoring intended behavior. Feature: New behavior.
- "What if I'm really sure I understand requirements?"
  - Still use `/plan` if non-trivial - catches missed edge cases.

**Facilitator Tips:**
- Be firm on policy (this is mandatory)
- Also be reasonable (trivial things excepted)
- Emphasize benefits, not just compliance

---

#### Section 3.2: Pilot Program (1:45 - 1:55) - 10 minutes

**Transition:** "Before full rollout, we'll run a 2-week pilot to validate the workflow."

**Pilot Goal (1 minute)**
"Validate `/plan` workflow with real work before full mandatory usage."

**Pilot Features (3 minutes)**

"I've selected 2-3 upcoming features for pilot:" (display on screen)

**Example features (adapt to actual project backlog):**
1. **Crossfade Curve Editor UI** (SPEC002-crossfade.md)
   - Complexity: Medium
   - Timeline: Week 1-2
   - Spec status: Complete

2. **Musical Flavor Distance Algorithm** (SPEC003-flavor_distance.md)
   - Complexity: Medium-high
   - Timeline: Week 2-3
   - Spec status: Complete

3. **User Playlist Management** (REQ001-requirements.md § Playlists)
   - Complexity: Medium
   - Timeline: Week 2-3
   - Spec status: Needs minor updates

"These features are:
- Non-trivial (require `/plan`)
- Well-specified (specs exist)
- Scheduled for next 2 weeks"

**Pilot Participants (3 minutes)**

"I need 2-3 volunteers for pilot."

**Commitment required:**
- Use `/plan` for your assigned feature
- Provide feedback (5 min daily check-in)
- Participate in end-of-pilot retrospective (30 min)

**What you'll get:**
- Direct support (I'm available for questions)
- First experience with workflow (before teammates)
- Input on policy refinement

"Who's interested?" (wait for hands/volunteers)

**Assign features:**
- Match volunteers to features
- Consider: skill level, interest, availability
- Confirm specification documents exist
- Set pilot start date (typically next workday)

**Feedback Collection (2 minutes)**

"How we'll collect feedback:"

**Daily:** (1 min)
- Quick Slack/email check-in
- Questions: "Any blockers? Issues? Surprises?"
- Response time: Within 2 hours

**End-of-pilot:** (1 min)
- 30-minute retrospective meeting (all pilots + facilitator)
- Questions:
  - What worked well?
  - What slowed you down?
  - What would improve workflow?
  - Ready for team-wide rollout?

**Timeline (1 minute)**
- Week 1-2: Pilot execution
- Week 3: Retrospective + policy refinement
- Week 4: Full team rollout

**Facilitator Tips:**
- Have pilot features identified BEFORE workshop
- Make volunteering attractive (support, input)
- If no volunteers: Assign thoughtfully (skilled + positive)
- Set clear expectations (this is real work, not just exercise)

---

#### Section 3.3: Action Items and Commitments (1:55 - 2:00) - 5 minutes

**Finalize Logistics (2 minutes)**

**For Pilots:**
1. "I'll send you feature assignments by end of day"
2. "Specifications are in docs/ - I'll confirm paths"
3. "Start Monday - use `/plan` as first step"
4. "Slack me anytime with questions"

**For All Participants:**
1. "Next eligible feature - use `/plan` (post-pilot)"
2. "Help colleagues during adoption (you're all trained)"
3. "Report issues to me (I'll track and address)"

**Support Resources (1 minute)**

Display on screen:

```
SUPPORT RESOURCES:
- Documentation: .claude/commands/plan.md
- Examples: wip/ directory (completed plans)
- Help Channel: #wkmp-workflows [or specify]
- Office Hours: Wednesdays 2-3pm [or specify]
```

"All resources available now - bookmark them"

**Exit Survey (2 minutes)**

"Please complete the exit survey on your attendance sheet."

**Give 2 minutes for completion:**
- 5 questions
- 1-5 scale
- Optional comments

**Quick verbal feedback (optional):**
If time permits: "One-word reactions - how do you feel about `/plan` workflow?"
- Go around room quickly
- Capture sentiment

**Closing (30 seconds)**

"Thank you for your engagement today! Key takeaways:
1. `/plan` is mandatory for non-trivial features
2. Pilot starts [date] with [volunteers]
3. Full rollout in 4 weeks
4. You're all prepared to use this workflow

Questions? Come see me or Slack anytime."

**Collect attendance sheets**

**Facilitator Tips:**
- End exactly on time (2 hours)
- Energy and enthusiasm at close
- Thank participants genuinely
- Be available afterward for questions

---

## Post-Workshop Actions

### Immediate (Same Day)

- [ ] Send thank-you email to all participants
- [ ] Attach workshop materials again
- [ ] Send pilot assignments to volunteers
- [ ] Confirm specification paths with pilots

### Within 48 Hours

- [ ] Compile exit survey results
- [ ] Share summary with participants
- [ ] Address any concerns raised
- [ ] Schedule pilot retrospective (Week 3)

### Week 1-2: Pilot Support

- [ ] Daily check-ins with pilots (Slack/email)
- [ ] Monitor for blockers
- [ ] Collect feedback continuously
- [ ] Refine documentation based on questions

### Week 3: Retrospective

- [ ] Hold 30-min pilot retrospective
- [ ] Gather detailed feedback
- [ ] Identify policy refinements
- [ ] Update documentation if needed
- [ ] Prepare for full rollout

### Week 4: Full Rollout

- [ ] Announce mandatory usage (all team)
- [ ] Share pilot results and learnings
- [ ] Make yourself available for questions
- [ ] Monitor adoption and address issues

---

## Troubleshooting Guide

### Technical Issues During Workshop

**Problem: `/plan` command fails**
- Fallback: Show pre-generated plan
- Debug: Check file path, Claude Code version
- Continue: Use backup plan for discussion

**Problem: Participants can't find example_specification.md**
- Solution: Screen share your file
- Backup: Email file during break
- Workaround: Pair with neighbor

**Problem: Generated plans vary significantly**
- Expected: AI generates different approaches
- Emphasize: That's OK - plans are customizable
- Focus: Common elements (requirements, tests)

### Engagement Issues

**Problem: No questions during Q&A**
- Tactic: Ask participants questions
  - "Does this solve problems you've experienced?"
  - "What concerns do you have?"
- Tactic: Pose common questions yourself
  - "I often hear..."

**Problem: Negative reactions to mandatory policy**
- Listen: Validate concerns
- Explain: Rationale (past problems, measurable benefits)
- Flexibility: Exemptions for trivial work
- Pilot: "Let's validate with pilot before judging"

**Problem: Exercise taking too long**
- Adjust: Shorten review time
- Skip: Partner share (move to group discussion)
- Prioritize: Ensure 10 min for group discussion

### Timing Issues

**Problem: Running behind schedule**
- Cut: Q&A time (offer to follow up individually)
- Shorten: Demo (show plan, skip execution)
- Combine: Some discussion sections

**Problem: Ahead of schedule**
- Extend: Hands-on exercise (deeper analysis)
- Add: More examples from real codebase
- Extra: Q&A time (deeper questions)

---

## Success Indicators

### During Workshop

- ✓ Participants engaged (asking questions, taking notes)
- ✓ Hands-on exercise completed by 80%+ of participants
- ✓ 3+ pilot volunteers
- ✓ Positive body language / chat sentiment

### Exit Survey (Target Scores)

- Overall satisfaction: 4.0+ / 5.0
- Clarity of instruction: 4.2+ / 5.0
- Relevance to work: 4.5+ / 5.0
- Confidence in using `/plan`: 3.5+ / 5.0

### Follow-Up (Weeks 1-4)

- ✓ Pilots complete features using `/plan`
- ✓ Positive pilot feedback
- ✓ No major workflow blockers identified
- ✓ Team adopts workflow post-rollout

---

## Facilitator Self-Reflection

After workshop, complete this checklist:

**What went well?**
- [ ] Timing (on schedule?)
- [ ] Technical demos (smooth?)
- [ ] Engagement (active participation?)
- [ ] Clarity (concepts understood?)

**What could improve?**
- [ ] Pacing (too fast/slow?)
- [ ] Examples (relevant/clear?)
- [ ] Materials (sufficient/clear?)
- [ ] Logistics (room/tech setup?)

**Action items for next time:**
- [ ] [Specific improvement 1]
- [ ] [Specific improvement 2]
- [ ] [Specific improvement 3]

---

**Questions about facilitation?** Review this guide before workshop and practice demo. You've got this!
