# [Topic Name] - Analysis Request

**Date Created:** YYYY-MM-DD
**Author:** [Your Name]
**Related Documents:** [Links to relevant specs, requirements, etc.]
**Priority:** [LOW / MEDIUM / HIGH / CRITICAL]
**Timeline:** [When do you need answers? ASAP / This Week / This Month / No Rush]

---

## Purpose

**What I'm trying to accomplish:**
[Brief description of what you're trying to do, decide, or understand - 1-3 sentences]

**Why this analysis is needed:**
[Context: What prompted this? What decision depends on it? What problem are we solving?]

---

## Questions to Answer

### Question 1: [Brief question title]

**Full Question:**
[Detailed question text. Be specific about what you want to know.]

**Why This Matters:**
[Why is this question important? What decision or action depends on the answer?]

**Constraints/Context:**
- [Any relevant constraints, assumptions, or context]
- [e.g., "Must work with existing microservices architecture"]
- [e.g., "Budget constraint: <40 hours implementation"]

---

### Question 2: [Brief question title]

**Full Question:**
[Detailed question text]

**Why This Matters:**
[Decision/action dependency]

**Constraints/Context:**
- [Constraint 1]
- [Constraint 2]

---

### Question 3: [Add more questions as needed]

[Repeat structure above]

---

## Problems to Solve

### Problem 1: [Brief problem title]

**Problem Description:**
[What's broken, missing, or not working as intended? Be specific.]

**Current State:**
[What happens now? What's the symptom?]

**Desired State:**
[What should happen instead? What's the goal?]

**Impact:**
[Who is affected? How severe? What are the consequences of not solving this?]

**Root Cause Hypotheses (if any):**
- [Your theory about why this is happening]
- [Or: "Unknown - need investigation"]

---

### Problem 2: [Add more problems as needed]

[Repeat structure above]

---

## Options to Compare (if applicable)

**Decision to Make:**
[What choice are you trying to make? e.g., "Which architecture should we use for feature X?"]

### Option A: [Option name]

**Description:**
[What is this approach?]

**Pros (as I see them):**
- [Advantage 1]
- [Advantage 2]

**Cons (as I see them):**
- [Disadvantage 1]
- [Disadvantage 2]

**Open Questions:**
- [What do I not know about this option?]

---

### Option B: [Option name]

[Repeat structure above]

---

### Option C: [Add more options as needed]

[Repeat structure above]

---

## Research Needed

**Internal (Codebase/Docs):**
- [ ] [What existing code should be reviewed?]
- [ ] [What documentation should be consulted?]
- [ ] [What specifications are relevant?]

**External (Internet Research):**
- [ ] [What industry standards should be investigated?]
- [ ] [What best practices should be researched?]
- [ ] [What technical solutions should be explored?]

**Validation:**
- [ ] [What assumptions need verification?]
- [ ] [What claims need evidence?]

---

## Success Criteria

**This analysis is successful if:**
- [ ] [All questions answered with evidence/reasoning]
- [ ] [All options compared with pros/cons quantified]
- [ ] [Recommendations provided with clear rationale]
- [ ] [Any other specific outcomes you need]

---

## Background Information (Optional)

**Relevant History:**
[Any historical context, previous attempts, or related work]

**Related Decisions:**
[Past decisions that inform this analysis]

**Stakeholders:**
[Who cares about this? Who should review the results?]

---

## Notes / Additional Context

[Any other information that might be helpful for the analysis]

---

## Expected Output Format (Optional)

**Preferred Structure:**
- [ ] Executive summary with recommendations
- [ ] Detailed option comparison table
- [ ] Implementation considerations (but NOT implementation plan)
- [ ] Risk analysis
- [ ] Other: ___________________

**Depth of Analysis:**
- [ ] High-level overview (1-2 pages)
- [ ] Moderate depth (5-10 pages)
- [ ] Comprehensive analysis (10+ pages)

---

# Usage Instructions

## When to Use This Template

Use this template when you need to:
- Answer complex questions requiring research
- Compare multiple solution approaches
- Investigate problems and identify root causes
- Make architectural or design decisions
- Understand tradeoffs between options

**DO NOT use for:** Implementation planning (use `/plan` workflow instead)

## How to Use This Template

1. **Copy this template** to `wip/[topic_name]_analysis_request.md`
2. **Fill out all sections** (delete unused sections if not applicable)
3. **Be specific** - vague questions get vague answers
4. **Run `/think`** on your completed document:
   ```
   /think wip/[topic_name]_analysis_request.md
   ```
5. **Review results** in generated `wip/[topic_name]_analysis_results.md`

## Tips for Better Analyses

### Good Question Examples

✅ **Specific and scoped:**
"Should we use PostgreSQL or SQLite for the Program Director's flavor distance calculations, given that we need to query 10,000+ passages and prioritize read performance?"

✅ **Context-rich:**
"The Audio Player currently uses a ring buffer with 2-second capacity. Should we increase this to 5 seconds to handle slower storage devices? What are the memory/latency tradeoffs?"

❌ **Too vague:**
"What database should we use?"

❌ **Too broad:**
"How should we architect the entire system?"

### Good Problem Descriptions

✅ **Observable symptoms:**
"Crossfades sometimes cut off abruptly in the last 200ms. This happens ~10% of the time, seemingly random. Users report jarring transitions."

✅ **Measurable impact:**
"Database queries take 3-5 seconds during peak usage (>1000 passages). This causes UI lag and violates the <100ms response time requirement (REQ-PERF-020)."

❌ **Too general:**
"Performance is bad."

❌ **Solution-focused (not problem-focused):**
"We need to add caching." (Better: "What's causing the slow queries?")

### Good Option Comparisons

✅ **Comparable alternatives:**
- Option A: Event-driven architecture with message queue
- Option B: Direct HTTP calls between microservices
- Option C: Shared database with triggers

✅ **Evaluation criteria stated:**
"Compare based on: latency, fault tolerance, implementation complexity, operational overhead"

❌ **Incomparable options:**
- Option A: Use Rust
- Option B: Implement feature X
(These are different decision dimensions)

## What `/think` Will Produce

**Output Document:** `wip/[topic_name]_analysis_results.md`

**Typical Structure:**
- Executive Summary (<300 lines)
- Problems Analyzed (root causes, impact assessment)
- Research Findings (with citations)
- Option Comparisons (pros/cons, tradeoffs)
- Recommendations (with rationale)
- Open Questions / Risks
- Appendices (detailed research, supporting data)

**What `/think` Will NOT Do:**
- Create implementation plans (use `/plan` instead)
- Write executable code
- Define detailed test specifications
- Provide step-by-step instructions

## Workflow Integration

**Before `/think`:**
- Clarify questions/problems (use this template)
- Identify constraints and success criteria
- Gather relevant document references

**After `/think`:**
- Review analysis results
- Make decisions based on recommendations
- If implementing: Run `/plan` on relevant specification
- Archive analysis when decision is made (use `/archive`)

---

**Version:** 1.0
**Category:** TMPL (Template)
**Related Workflows:** `/think`, `/plan`, `/archive`
**Maintained By:** Technical Lead
**Last Updated:** 2025-10-25
