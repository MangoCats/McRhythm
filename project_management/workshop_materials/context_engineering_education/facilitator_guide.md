# Facilitator Guide: Context Engineering Education Session

**For:** Technical Lead (or designated facilitator)
**Session Duration:** 2 hours
**Preparation Time:** 2-3 hours
**Difficulty:** Intermediate (requires familiarity with `/plan` workflow)

---

## Pre-Session Checklist (1 Week Before)

### Logistics
- [ ] **Room:** Conference room with projector, seats for 15-20
- [ ] **Tech:** Test projector connection, increase terminal font to 18pt+
- [ ] **Materials:** Print handouts (see checklist below)
- [ ] **Timing:** Schedule during work hours, avoid Monday mornings/Friday afternoons
- [ ] **Invites:** Send calendar invites with agenda attached

### Materials Preparation

**Print Quantities** (assume 20 participants):
- [ ] PLAN003 Summary: 20 copies (handout at start)
- [ ] Example `/plan` Output: 20 copies (handout during Part 1)
- [ ] Verbose vs. Concise Reference: 20 copies (handout during Part 2)
- [ ] Scenario Cards: 21 cards (3 types × 7 groups)
- [ ] Blank Worksheets: 30 sheets (extras for mistakes)
- [ ] Quick Reference Cards: 25 copies (handout at end, extras)
- [ ] Exit Surveys: 25 forms (extras)

**Digital Preparation:**
- [ ] Create slide deck (template below, 10-12 slides max)
- [ ] Prepare demo environment:
  - Clean terminal with working `/plan` command
  - Example spec file ready: `workshop_example_spec.md`
  - Pre-run `/plan` once to verify output format
- [ ] Load CLAUDE.md excerpt on laptop (backup if projector fails)

---

## Session Flow Guide (Minute-by-Minute)

### 0:00-0:15 - Introduction: The Context Problem

**Slide 1: Title**
```
Context Engineering Standards
Team Education Session

[WKMP Logo]
Facilitator: [Your Name]
Date: [Date]
```

**Slide 2: Why We're Here**
```
"Most agent failures are context failures"
  — Research from Anthropic, LangChain, LlamaIndex, arXiv, AWS, Google Cloud

Our challenge: WKMP documentation is growing
→ Context window limitations affect AI assistants AND human readers
```

**Facilitator Notes:**
- Start on time even if stragglers arriving
- Distribute PLAN003 Summary handout
- Ask: "Has anyone encountered an AI assistant missing a requirement?" (show of hands, no discussion yet)

---

**Slide 3: Two Problems We're Solving**
```
Problem 1: Implementations Overlook Specifications
  → AI can't fit entire spec in context window
  → Requirements scattered across multiple documents
  → No systematic traceability enforcement

Problem 2: AI-Generated Documents Are Bloated
  → AI defaults to verbose, comprehensive output
  → Documents 20-40% longer than necessary
  → "Lost in the middle" phenomenon
```

**Facilitator Notes:**
- Share anonymized example: "We had a feature where 3 of 7 requirements were missed"
- Don't name names or blame individuals
- Emphasize: This is a systemic problem, not individual performance issue

---

**Slide 4: Four Interventions (Overview)**
```
Phase 1 (Implemented):
  1. Mandate /plan workflow (>5 requirements)
  2. Explicit verbosity constraints (20-40% reduction)
  3. Summary-first reading protocol
  4. Modular structure for large docs (>1200 lines)

Expected Impact:
  ✓ Catch spec issues BEFORE coding
  ✓ 20-40% smaller documents
  ✓ Better context window usage
```

**Facilitator Notes:**
- This is preview - details coming in Parts 1-2
- Point to PLAN003 handout for full details
- Transition: "Let's dive into the first intervention..."

---

### 0:15-0:45 - Part 1: `/plan` Workflow Deep Dive

**Slide 5: When to Use `/plan`**
```
MANDATORY for:
  • Features with >5 requirements
  • Novel/complex features (regardless of count)

OPTIONAL for:
  • Small enhancements
  • Bug fixes
  • Refactoring

Decision Tree:
  >5 requirements? → YES → /plan
  Novel/complex? → YES → /plan
  Otherwise → Optional (but recommended)
```

**Facilitator Notes:**
- Emphasize "MANDATORY" - this is in CLAUDE.md now
- Definition of "novel": No similar feature in codebase
- Definition of "complex": Touches 3+ modules or 5+ files
- Ask: "What counts as one requirement?" (Answer: Each REQ-XXX-### ID)

---

**Slide 6: `/plan` Workflow - What It Does**
```
Input: Specification document (SPEC###, REQ###, etc.)

Phase 1: Extract Requirements (enumerated list)
Phase 2: Find Specification Issues (CRITICAL → LOW)
Phase 3: Generate Acceptance Tests (with traceability)

Output:
  → PLAN###_[feature_name]/ folder
    ├── 00_PLAN_SUMMARY.md
    ├── requirements_index.md
    ├── 01_specification_issues.md
    └── 02_test_specifications/
```

**Facilitator Notes:**
- This is overview - live demo coming next
- Distribute "Example `/plan` Output" handout NOW
- Give participants 2 minutes to skim handout while you prepare demo

---

**LIVE DEMO (15 minutes)**

**Terminal Setup:**
- Font size: 18pt minimum
- Color scheme: High contrast (dark background recommended)
- Working directory: `c:\Users\Mango Cat\Dev\McRhythm`

**Demo Script:**

```bash
# Step 1: Show the specification (briefly)
cat workshop_example_spec.md | head -30

# Facilitator narration: "This is our example spec with 8 requirements for a User Settings Export/Import feature."

# Step 2: Run /plan
/plan workshop_example_spec.md

# Facilitator narration: "Watch what happens - the workflow runs three phases automatically."

# Step 3: Show outputs (after /plan completes)
ls wip/PLAN###_user_settings_export/

# Facilitator narration: "Notice the folder structure - summary, requirements, issues, tests."

# Step 4: Quick peek at each file
cat wip/PLAN###_user_settings_export/00_PLAN_SUMMARY.md | head -50
cat wip/PLAN###_user_settings_export/01_specification_issues.md | grep "PRIORITY:"
cat wip/PLAN###_user_settings_export/02_test_specifications/test_index.md

# Facilitator narration: "The key outputs are: requirements list, issue detection, test specs."
```

**Key Points During Demo:**
1. **Requirements Extraction:** "AI reads spec and enumerates all requirements - you verify completeness"
2. **Issue Detection:** "Catches ambiguities, conflicts, missing info BEFORE you code"
3. **Acceptance Tests:** "Test-first approach - you know what 'done' looks like"
4. **Traceability Matrix:** "Every requirement has ≥1 test - 100% coverage verified"

**Common Questions (prepare answers):**
- Q: "How long does `/plan` take to run?"
  - A: "5-15 minutes depending on spec size, runs in background"
- Q: "What if it finds CRITICAL issues?"
  - A: "MUST resolve before implementing - update spec or clarify requirements"
- Q: "Can I modify the tests?"
  - A: "Yes! They're starting points - refine as needed, maintain traceability"

---

**Q&A and Discussion (10 minutes)**

**Slide 7: Common Questions**
```
"What if I find issues during implementation?"
  → Update spec, re-run /plan, update tests

"How do I update tests when requirements change?"
  → Edit test files manually, update traceability matrix

"Can I skip tests for prototypes?"
  → Prototypes are exempt, but production features: NO
```

**Facilitator Notes:**
- Open floor for questions
- If quiet, seed with: "Who's working on a feature that would benefit from `/plan`?"
- Keep answers concise - defer deep dives to after session
- Capture questions you can't answer immediately - promise follow-up

---

### 0:45-1:15 - Part 2: Document Standards

**Slide 8: Verbosity Standards**
```
Target: 20-40% shorter than comprehensive first draft

5 Techniques:
  1. Bullet points instead of paragraphs
  2. One concept per sentence
  3. Link to existing docs (don't repeat)
  4. Omit unnecessary details
  5. Use tables for structured data

Examples → Interactive Exercise
```

**Facilitator Notes:**
- Show "Before Example" slide (next)
- Participants edit individually (3 min silent)
- Share-out: 2-3 volunteers read their version
- Show "After Example" slide
- Key insight: "We cut 67% of words, kept 100% of information"

---

**Slide 9: Interactive Exercise - Reduce Verbosity**
```
BEFORE (42 words):
"The crossfade feature is designed to provide a smooth transition
between two audio passages by gradually decreasing the volume of
the currently playing passage while simultaneously increasing the
volume of the next passage, creating a seamless listening experience
that avoids abrupt transitions which can be jarring to listeners."

TASK: Rewrite in <15 words using bullet points (3 minutes)
```

**Facilitator Notes:**
- Distribute blank paper
- Set timer for 3 minutes
- Circulate to see participant answers
- Select 2-3 good examples to share aloud

---

**Slide 10: After Example (Reveal After Exercise)**
```
AFTER (14 words):
"Crossfade creates smooth transitions by:
- Fading out current passage volume
- Fading in next passage volume simultaneously
- Avoiding abrupt cuts"

Reduction: 67% fewer words
Information: 100% preserved
Clarity: Improved (scannable bullet points)
```

---

**Slide 11: Document Size Thresholds**
```
<300 lines
  → Single file, no special requirements

300-1200 lines
  → Single file OK
  → Executive summary RECOMMENDED

>1200 lines
  → MANDATORY modular folder structure
  → Templates available: templates/modular_document/
```

**Facilitator Notes:**
- Distribute "Verbose vs. Concise Reference" handout now
- Show modular structure example on screen:
  ```
  SPEC018_advanced_feature/
  ├── 00_SUMMARY.md (<500 lines - READ THIS FIRST)
  ├── 01_architecture.md (<300 lines)
  ├── 02_api_design.md
  └── FULL_DOCUMENT.md (archival only)
  ```
- Key point: `/doc-name` workflow checks size automatically

---

**Slide 12: Summary-First Reading Protocol**
```
MANDATORY Reading Pattern:
  1. Always start with summary
     (00_SUMMARY.md or first 50-100 lines)

  2. Identify relevant sections from summary

  3. Read targeted sections only (not full doc)

  4. Load full spec ONLY if necessary

Benefits:
  • Faster reviews
  • Better context window usage
  • Clearer understanding of scope
```

**Facilitator Notes:**
- This applies to AI assistants AND human reviewers
- Already in CLAUDE.md as global instruction
- Ask: "How many people read entire specs top-to-bottom?" (show of hands)
- Validate both approaches - summary-first is NEW recommended pattern

---

### 1:15-1:30 - Break

**Facilitator Actions During Break:**
- Prepare scenario cards on tables (7 groups, 3 scenarios)
- Arrange seating for group work (clusters of 2-3 chairs)
- Check participant engagement levels (adjust Part 3 timing if needed)

---

### 1:30-1:50 - Part 3: Hands-On Practice

**Slide 13: Group Exercise**
```
Form groups of 2-3 participants
Each group gets one scenario card

15 minutes: Complete your scenario
5 minutes: Share-out (2 min per group)

Scenarios:
  1. Plan or No Plan? (Decision tree practice)
  2. Reduce Verbosity (Rewriting practice)
  3. Structure Decision (Modular vs. single file)
```

**Facilitator Notes:**
- Form groups quickly (count off 1-7)
- Distribute scenario cards (rotate types so each scenario has 2-3 groups)
- Distribute blank worksheets
- Set timer for 15 minutes
- Circulate between groups - answer questions, observe approaches
- 5-minute warning at 10-minute mark

---

**Scenario Card Templates:**

**Scenario 1: Plan or No Plan?**
```
FEATURE: "Playlist Import from External Services"

Requirements:
  REQ-PI-010: Support Spotify playlist imports
  REQ-PI-020: Support Apple Music playlist imports
  REQ-PI-030: OAuth authentication for external services
  REQ-PI-040: Map external track IDs to MusicBrainz IDs
  REQ-PI-050: Handle missing/unavailable tracks gracefully
  REQ-PI-060: Import playlist metadata (name, description, artwork)
  REQ-PI-070: Preserve track order from external service

TASK:
  1. Must you use /plan? Why or why not?
  2. What would /plan outputs include?
  3. Any concerns or questions?
```

**Scenario 2: Reduce Verbosity**
```
BEFORE (Design section, 187 words):
"The Program Director microservice operates on a scheduled basis to select
the next passage for playback. It evaluates all available passages in the
database by calculating their musical flavor distance from the current
timeslot's target flavor using a cosine similarity metric. The selection
algorithm then applies several filters including song cooldown periods which
prevent the same song from being played too frequently, artist cooldown
periods which ensure variety across different artists, and work cooldown
periods for classical music pieces. After filtering, the algorithm computes
a probability distribution based on the inverse of the flavor distance and
the base probability assigned to each passage by the user, normalizes these
probabilities, and selects a passage using weighted random sampling. The
selected passage is then enqueued to the Audio Player microservice via an
HTTP POST request to the /api/queue/add endpoint, which returns a
confirmation including the queue position and estimated playback start time
based on current queue contents and crossfade timings."

TASK: Rewrite to meet verbosity standards (<75 words, 60% reduction)
CONSTRAINTS: Maintain all essential information
```

**Scenario 3: Structure Decision**
```
DOCUMENT: "SPEC019_advanced_flavor_tuning.md"

Estimated Content:
  - Executive Summary: 150 lines
  - Background & Motivation: 200 lines
  - Flavor Vector Specification: 300 lines
  - Tuning Algorithm Design: 400 lines
  - User Interface Mockups: 250 lines
  - API Endpoints: 200 lines
  - Database Schema Changes: 150 lines
  - Testing Strategy: 100 lines

Total Estimated: 1750 lines

TASK:
  1. Single file or modular structure? Why?
  2. If modular, propose folder structure
  3. What goes in 00_SUMMARY.md?
```

**Share-Out Instructions:**
- Call on groups in order
- 2 minutes per group (strict timing)
- Other groups can offer alternative approaches (30 seconds each)
- Facilitator provides brief feedback (30 seconds)

**Model Answers (for facilitator reference):**

**Scenario 1 Answer:**
- YES, must use `/plan` - has 7 requirements (>5 threshold)
- `/plan` outputs: 7 requirements enumerated, issues like "How to handle OAuth token refresh?", tests for each import source
- Concerns: External API changes, MusicBrainz matching accuracy

**Scenario 2 Answer (example):**
```
Program Director selects next passage via:
- Calculate flavor distance (cosine similarity vs. timeslot target)
- Apply filters: song/artist/work cooldowns
- Compute probability: inverse distance × user base probability
- Weighted random sampling
- Enqueue via POST /api/queue/add → returns position & start time

(59 words, 68% reduction)
```

**Scenario 3 Answer:**
- MANDATORY modular structure (1750 lines > 1200 threshold)
- Folder structure:
  ```
  SPEC019_advanced_flavor_tuning/
  ├── 00_SUMMARY.md (~400 lines: motivation + navigation)
  ├── 01_flavor_vector_spec.md (300 lines)
  ├── 02_algorithm_design.md (400 lines)
  ├── 03_ui_mockups.md (250 lines)
  ├── 04_api_database.md (350 lines: combined)
  ├── 05_testing_strategy.md (100 lines)
  └── FULL_DOCUMENT.md (1750 lines, archival)
  ```
- Summary includes: problem statement, key decisions, navigation map

---

### 1:50-2:00 - Wrap-Up: Resources & Next Steps

**Slide 14: Resources Available**
```
Where to Find Everything:

CLAUDE.md
  → Global AI instructions (3 new sections)

docs/GOV001-document_hierarchy.md (v1.6)
  → Document Size and Structure Standards

templates/modular_document/
  → README, 00_SUMMARY template, section template

.claude/commands/
  → All 6 workflows updated with size targets

project_management/workshop_materials/
  → /plan workshop, this session materials
```

**Facilitator Notes:**
- Distribute Quick Reference Cards NOW
- Show file locations on screen (quick `ls` commands)
- Emphasize: "You don't need to memorize - use Quick Reference Card"

---

**Slide 15: Next Steps & Feedback**
```
This Week:
  ✓ Review CLAUDE.md sections (10 minutes)
  ✓ Keep Quick Reference Card handy

Next Feature:
  ✓ Try /plan workflow (even if optional)
  ✓ Apply verbosity standards to docs

Ongoing:
  ✓ Summary-first reading for all docs
  ✓ Use templates for large documents

Questions?
  → Contact [Technical Lead] or post in [Team Channel]

Feedback:
  → Exit survey (2 minutes, anonymous)
  → Optional: Join metrics collection volunteer group
```

**Facilitator Notes:**
- Distribute exit surveys
- Give 2 minutes for survey completion
- While participants complete surveys, show volunteer sign-up sheet
- Explain metrics: "We'll track doc sizes, `/plan` usage, team satisfaction over 4 weeks"
- Thank participants for time and engagement

---

## Post-Session Actions

### Immediate (Same Day):
- [ ] Collect exit surveys
- [ ] Scan Quick Reference Cards (count how many taken vs. left behind)
- [ ] Note attendance count
- [ ] Upload slides to team repository
- [ ] Send thank-you email with links to materials

### Within 48 Hours:
- [ ] Compile feedback summary from exit surveys
- [ ] Calculate satisfaction metrics (% understanding, average rating)
- [ ] Document any questions that need follow-up research
- [ ] Send follow-up email:
  - Session recap
  - Links to all resources
  - Q&A summary
  - Invitation to office hours (optional)

### Within 1 Week:
- [ ] Update PLAN003 metrics tracking with attendance and satisfaction data
- [ ] Schedule 1-on-1 check-ins with any participants who rated session <3/5
- [ ] Post session recording (if recorded) to team knowledge base
- [ ] Begin tracking `/plan` usage (set up monitoring)

---

## Troubleshooting Guide

### Problem: Low Attendance (<50%)
**Solution:**
- Reschedule session for better time (poll team for availability)
- Offer recorded version with async Q&A
- Provide self-study packet with all materials

### Problem: Session Running Over Time
**Cut in Order:**
1. Reduce hands-on practice from 20 to 10 minutes (skip share-out)
2. Shorten break from 15 to 10 minutes
3. Defer advanced Q&A to follow-up email
4. Abbreviate verbose example exercise (show answer without individual work)

### Problem: Demo Technical Failure
**Backup Plan:**
- Use printed `/plan` output handout (already distributed)
- Show slide screenshots of demo instead of live terminal
- Provide demo video link in follow-up email
- Narrate demo using static examples

### Problem: Participants Confused/Resistant
**Responses:**
- Emphasize: "These standards improve OUR productivity, not just AI"
- Show research citations: "This is evidence-based, not arbitrary"
- Validate concerns: "Fair question - let's discuss in office hours"
- Offer pilot program: "Try it on one feature, evaluate effectiveness"

### Problem: Questions You Can't Answer
**Protocol:**
- Acknowledge: "Great question - I don't have definitive answer right now"
- Commit: "I'll research and send email update within 48 hours"
- Document: Write question on whiteboard/notepad
- Follow through: Actually send the answer (builds trust)

---

## Metrics Collection

### During Session:
- Attendance count
- Exit survey responses (4 questions)
- Volunteer sign-ups for metrics tracking

### Exit Survey Questions:
1. "I understand when to use `/plan` workflow" (1-5 scale)
2. "I feel confident applying verbosity standards" (1-5 scale)
3. "I can choose appropriate document structure (single vs. modular)" (1-5 scale)
4. "Overall session rating" (1-5 scale)
5. "Most valuable part of session?" (free text)
6. "What needs clarification?" (free text)

### Post-Session (4 Weeks):
- Number of features planned with `/plan`
- Average document size trend (new docs)
- Percentage of new docs >1200 lines using modular structure
- Team satisfaction follow-up (brief survey)

---

## Tips for Success

### Preparation:
- **Practice demo twice** - know the commands cold
- **Arrive 15 min early** - tech issues happen
- **Have backup materials** - projectors fail

### Delivery:
- **Start on time** - respect participants' calendars
- **Use timer** - keep sections moving
- **Encourage questions** - but defer deep dives
- **Show enthusiasm** - your energy sets the tone

### Follow-Up:
- **Respond within 48 hours** - to surveys and questions
- **Share materials broadly** - team knowledge base
- **Track metrics honestly** - even if results are mixed
- **Iterate for next time** - capture lessons learned

---

**Version:** 1.0
**Date Created:** 2025-10-25
**Next Review:** After first session delivery
