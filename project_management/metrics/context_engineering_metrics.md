# Context Engineering Metrics Collection

**Project:** WKMP
**Plan:** PLAN003 - Context Engineering Implementation
**Period:** 4 weeks post-Phase 2 education session
**Owner:** Technical Lead
**Status:** Active Monitoring

---

## Purpose

Track effectiveness of Phase 1 and Phase 2 context engineering interventions to inform Phase 3 decisions. Metrics collection spans three categories:

1. **Document Size Trends** - Are new documents shorter?
2. **Workflow Adoption** - Is `/plan` being used?
3. **Team Satisfaction** - Are standards helpful or burdensome?

---

## Success Criteria (from PLAN003)

**Document Size:**
- Target: 20-40% reduction in new document verbosity
- Minimum: ≥15% average reduction to proceed with Phase 3

**Workflow Adoption:**
- Target: ≥3 features planned with `/plan` workflow in 4-week period
- Minimum: ≥2 features to validate usefulness

**Team Satisfaction:**
- Target: ≥80% positive feedback on standards
- Minimum: ≥70% to avoid rollback

**Specification Issues Detected:**
- Target: Measurable counts of CRITICAL/HIGH issues caught before implementation
- Validation: Issues caught by `/plan` that would have caused rework

---

## Metrics Collection Schedule

### Week 0 (Baseline - Before Education Session)
**Collect:**
- Document sizes for all docs created in past 4 weeks
- Number of features planned vs. ad-hoc implementation
- Historical rework incidents (spec misses)

### Week 1 (Post-Education)
**Collect:**
- Education session attendance (count)
- Exit survey results (ratings + free text)
- Volunteer sign-ups for metrics tracking

### Weeks 2-5 (Monitoring Period)
**Collect Weekly:**
- New document sizes (line counts)
- `/plan` usage (features planned this week)
- Specification issues detected (counts by severity)
- Team feedback (optional pulse checks)

### Week 6 (Analysis)
**Analyze:**
- Compare baseline vs. monitoring period metrics
- Compile trends and insights
- Recommend Phase 3 decision (GO / NO-GO / MODIFY)

---

## Metric 1: Document Size Trends

### Data Collection

**What to Measure:**
- Line count of each new document created
- Document category (GOV, REQ, SPEC, IMPL, RPT, PLAN, etc.)
- Creation date
- Structure type (single file vs. modular)

**How to Collect:**
```bash
# Run weekly to capture new documents
find docs/ wip/ workflows/ -name "*.md" -mtime -7 -exec wc -l {} \; > metrics/doc_sizes_week_N.txt

# For modular docs, count summary size separately
find docs/ wip/ -name "00_SUMMARY.md" -mtime -7 -exec wc -l {} \;
```

**Baseline Period:** 4 weeks before education session (retrospective)

**Monitoring Period:** 4 weeks after education session

### Data Recording

**Format:** CSV or markdown table

| Date | Document | Category | Lines | Structure | Notes |
|------|----------|----------|-------|-----------|-------|
| 2025-10-20 | SPEC018_feature.md | SPEC | 1450 | Single | Pre-standards |
| 2025-10-28 | SPEC019_new_feature/ | SPEC | 480 (summary) | Modular | Post-standards |
| 2025-10-28 | SPEC019_new_feature/ | SPEC | 1620 (full) | Modular | Post-standards |

**Tracking File:** `project_management/metrics/document_size_log.csv`

### Analysis Metrics

**Primary Metric:** Average document size reduction
```
Baseline Average = Sum(baseline doc sizes) / Count(baseline docs)
Monitoring Average = Sum(monitoring doc sizes) / Count(monitoring docs)
Reduction % = (Baseline - Monitoring) / Baseline * 100
```

**For Modular Docs:** Count summary size (what readers actually load), not full size

**Target:** 20-40% reduction

**Secondary Metrics:**
- Percentage of docs >1200 lines using modular structure (target: 90%+)
- Average summary size for modular docs (target: <500 lines)
- Compliance rate with verbosity standards (manual code review)

---

## Metric 2: `/plan` Workflow Adoption

### Data Collection

**What to Measure:**
- Number of features qualifying for `/plan` (>5 req or novel/complex)
- Number of features actually using `/plan`
- Specification issues detected (by severity: CRITICAL, HIGH, MEDIUM, LOW)
- Time from `/plan` completion to implementation start
- Rework incidents avoided (subjective estimate)

**How to Collect:**
```bash
# Count PLAN### folders created
ls -d wip/PLAN*/ | wc -l

# Extract specification issues from plans
grep -r "PRIORITY: CRITICAL" wip/PLAN*/01_specification_issues.md | wc -l
grep -r "PRIORITY: HIGH" wip/PLAN*/01_specification_issues.md | wc -l
```

**Developer Survey (weekly):**
- "Did you start any features this week qualifying for `/plan`?" (Y/N)
- "If yes, did you use `/plan`?" (Y/N)
- "If no, why not?" (free text)

### Data Recording

| Week | Qualifying Features | Used /plan | Issues Found (C/H/M/L) | Rework Avoided | Notes |
|------|---------------------|------------|------------------------|----------------|-------|
| Baseline 1 | 2 | 0 | N/A | - | Pre-standards |
| Baseline 2 | 1 | 0 | N/A | - | Pre-standards |
| Monitor 1 | 1 | 1 | 0/2/3/1 | Est. 4 hours | User Settings Export |
| Monitor 2 | 0 | 0 | - | - | No qualifying features |

**Tracking File:** `project_management/metrics/plan_usage_log.md`

### Analysis Metrics

**Primary Metric:** `/plan` adoption rate
```
Adoption Rate = Features Using /plan / Qualifying Features * 100
```

**Target:** ≥75% adoption rate

**Secondary Metrics:**
- Average issues detected per plan (by severity)
- Percentage of plans revealing CRITICAL issues (indicates value)
- Developer feedback on `/plan` usefulness (qualitative)

---

## Metric 3: Team Satisfaction

### Data Collection

**Education Session Feedback (Week 1):**
- Exit survey responses (4 rating questions, 1-5 scale)
- Free-text feedback compilation
- Attendance percentage

**Ongoing Feedback (Weeks 2-5):**
- Optional pulse surveys (1 question, weekly):
  - Week 2: "Verbosity standards helped (1-5) or hindered (1-5) my work this week"
  - Week 3: "Summary-first reading saved time this week (Y/N)"
  - Week 4: "I would recommend `/plan` to teammates (Y/N, why?)"
  - Week 5: "Overall satisfaction with context engineering standards (1-5)"

**Qualitative Data:**
- Slack/Teams channel discussions
- 1-on-1 feedback during sprint reviews
- Questions/concerns raised to technical lead

### Data Recording

**Exit Survey Summary:**
```
Total Attendees: 18 / 20 (90% attendance)

Q1: Understand when to use /plan
  - 1: 0  | 2: 1  | 3: 2  | 4: 10 | 5: 5
  - Average: 4.06 / 5.0

Q2: Confident applying verbosity standards
  - 1: 0  | 2: 0  | 3: 3  | 4: 9  | 5: 6
  - Average: 4.17 / 5.0

Q3: Can choose document structure
  - 1: 0  | 2: 1  | 3: 4  | 4: 8  | 5: 5
  - Average: 3.94 / 5.0

Q4: Overall session rating
  - 1: 0  | 2: 0  | 3: 1  | 4: 11 | 5: 6
  - Average: 4.28 / 5.0
```

**Tracking File:** `project_management/metrics/team_satisfaction_log.md`

### Analysis Metrics

**Primary Metric:** Overall satisfaction
```
Positive Feedback % = Count(ratings ≥4) / Total Responses * 100
```

**Target:** ≥80% positive feedback

**Secondary Metrics:**
- Education session attendance (target: ≥75%)
- Free-text theme analysis (e.g., "too complex", "very helpful", "needs examples")
- Adoption blockers (why developers avoid standards)

---

## Data Collection Tools

### Automated Scripts

**Script 1: Document Size Tracker**
```bash
#!/bin/bash
# File: scripts/metrics/track_doc_sizes.sh

WEEK_NUM=$1
OUTPUT_FILE="project_management/metrics/doc_sizes_week_${WEEK_NUM}.txt"

echo "Document Size Tracking - Week ${WEEK_NUM}" > $OUTPUT_FILE
echo "Generated: $(date)" >> $OUTPUT_FILE
echo "---" >> $OUTPUT_FILE

# Find all .md files modified in last 7 days
find docs/ wip/ workflows/ -name "*.md" -mtime -7 -exec wc -l {} \; >> $OUTPUT_FILE

echo "Summary sizes (modular docs only):" >> $OUTPUT_FILE
find docs/ wip/ -name "00_SUMMARY.md" -mtime -7 -exec wc -l {} \; >> $OUTPUT_FILE
```

**Script 2: Plan Usage Tracker**
```bash
#!/bin/bash
# File: scripts/metrics/track_plan_usage.sh

OUTPUT_FILE="project_management/metrics/plan_usage_snapshot.txt"

echo "Plan Usage Tracking - $(date)" > $OUTPUT_FILE
echo "---" >> $OUTPUT_FILE

echo "Total PLAN folders:" >> $OUTPUT_FILE
ls -d wip/PLAN*/ 2>/dev/null | wc -l >> $OUTPUT_FILE

echo "" >> $OUTPUT_FILE
echo "Issues by severity:" >> $OUTPUT_FILE
echo "CRITICAL:" >> $OUTPUT_FILE
grep -r "PRIORITY: CRITICAL" wip/PLAN*/01_specification_issues.md 2>/dev/null | wc -l >> $OUTPUT_FILE
echo "HIGH:" >> $OUTPUT_FILE
grep -r "PRIORITY: HIGH" wip/PLAN*/01_specification_issues.md 2>/dev/null | wc -l >> $OUTPUT_FILE
```

### Manual Collection Forms

**Weekly Developer Check-In** (Google Form / Microsoft Forms):
1. Did you start any features this week qualifying for `/plan` (>5 requirements or novel/complex)? [Y/N]
2. If yes, how many? [Number]
3. Did you use `/plan` for these features? [Y/N]
4. If no, why not? [Free text]
5. Optional: Feedback on standards [Free text]

**Pulse Survey** (1 question, rotating weekly):
- Week 2: "Verbosity standards this week: Helpful (1-5) or Hindered (1-5)?"
- Week 3: "Summary-first reading saved me time this week: Y/N"
- Week 4: "I would recommend `/plan` to teammates: Y/N + Why?"
- Week 5: "Overall satisfaction with context engineering standards (1-5)"

---

## Analysis & Reporting

### Week 6 Analysis Report

**Template:** `project_management/metrics/PLAN003_effectiveness_report.md`

**Sections:**
1. **Executive Summary** (<300 lines per standards!)
   - Key findings (3-5 bullet points)
   - Recommendation (GO / NO-GO / MODIFY for Phase 3)

2. **Document Size Analysis**
   - Baseline vs. monitoring comparison
   - Percentage reduction achieved
   - Modular structure compliance rate
   - Charts/graphs (optional)

3. **Workflow Adoption Analysis**
   - `/plan` adoption rate
   - Specification issues detected (value demonstration)
   - Estimated rework hours saved

4. **Team Satisfaction Analysis**
   - Survey results summary
   - Qualitative feedback themes
   - Adoption blockers identified

5. **Recommendations**
   - Phase 3 decision (with rationale)
   - Adjustments needed (if any)
   - Next steps

### Decision Criteria

**PROCEED WITH PHASE 3** if:
- Document size reduction ≥15% AND
- `/plan` adoption ≥50% AND
- Team satisfaction ≥70% positive

**MODIFY APPROACH** if:
- Any metric below threshold BUT team feedback suggests refinement (not abandonment)

**HALT / ROLLBACK** if:
- Team satisfaction <50% AND low adoption rates
- Standards actively hindering productivity (evidence-based)

---

## Baseline Data Collection (Retrospective)

### Actions This Week

**Document Sizes (Baseline):**
```bash
# Find all docs created in 4 weeks before education session
# Education session date: TBD (estimate 2025-11-01)
# Baseline period: 2025-10-04 to 2025-11-01

# Manual review of git log
git log --since="2025-10-04" --until="2025-11-01" --name-status --diff-filter=A -- "*.md" | grep "^A" | awk '{print $2}' > baseline_docs.txt

# Get line counts
while read file; do
  if [ -f "$file" ]; then
    wc -l "$file"
  fi
done < baseline_docs.txt > project_management/metrics/baseline_doc_sizes.txt
```

**Plan Usage (Baseline):**
- Review: How many features implemented in baseline period?
- Review: Were any planned systematically (predecessor to `/plan`)?
- Estimate: Qualification rate (features that WOULD have qualified for `/plan`)

**Satisfaction (Baseline):**
- No formal baseline (standards didn't exist)
- Proxy: Historical feedback on documentation quality/size issues

---

## Monitoring Period Execution

### Weekly Tasks (Technical Lead)

**Every Monday Morning:**
- [ ] Run automated scripts (doc size tracker, plan usage tracker)
- [ ] Send weekly pulse survey (if scheduled for this week)
- [ ] Review team chat for organic feedback
- [ ] Update tracking files with manual observations

**Every Friday Afternoon:**
- [ ] Compile week's data into summary
- [ ] Note any significant events (e.g., developer praised `/plan` in standup)
- [ ] Check for anomalies (e.g., sudden spike in document sizes)

**Ad-Hoc:**
- [ ] Respond to questions about standards (track common questions)
- [ ] Offer 1-on-1 assistance to developers struggling with standards
- [ ] Document workarounds or refinements suggested by team

---

## Privacy & Ethics

**Anonymous Surveys:**
- Exit surveys and pulse surveys are anonymous
- No individual tracking of compliance (team-level metrics only)

**Transparent Intent:**
- Team knows metrics are being collected
- Purpose clearly communicated: Validate effectiveness, inform Phase 3
- Results will be shared openly (good or bad)

**No Punitive Use:**
- Metrics NOT used for performance reviews
- Low adoption triggers investigation (blockers), not blame
- Standards may be adjusted based on feedback

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-10-25 | Initial metrics system (Phase 2) |

---

**Status:** Active - Begin baseline collection this week
**Next Review:** Week 6 analysis (approx. 2025-12-01)
**Owner:** Technical Lead
**Related:** PLAN003, GOV001 v1.6, REQ-CE-MON-010/020/030
