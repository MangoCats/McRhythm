# Scope Statement - Risk-Primary Decision Framework Implementation

**Plan ID:** PLAN004
**Feature:** Risk-Primary Decision Framework
**Date:** 2025-10-25
**Stakeholder:** Mango Cat (Principal Developer)
**Priority:** CRITICAL

---

## In Scope

### Documentation Updates

1. **CLAUDE.md**
   - Add new "Decision-Making Framework - MANDATORY" section
   - Define risk (primary) → quality (secondary) → effort (tertiary) prioritization
   - Include rationale referencing PCH001 project charter goals
   - Placement: After "Implementation Workflow - MANDATORY" section

2. **.claude/commands/think.md**
   - Restructure comparison framework template (lines 430-450)
   - Replace current template with risk-first structure:
     - Risk Assessment (failure modes, mitigation, residual risk)
     - Quality Characteristics (maintainability, test coverage, architecture fit)
     - Implementation Considerations (effort, dependencies, complexity)
   - Add RISK-BASED RANKING section
   - Update recommendation format to include risk-based justification

3. **.claude/commands/plan.md**
   - Modify Phase 4 objective statement (line 355)
   - Change from "acceptable risk/effort" to "minimal failure risk; acknowledge effort"
   - Add detailed Phase 4 process steps (risk assessment → ranking → selection)
   - Include ADR requirement with risk-based justification

### Template Creation

4. **templates/risk_assessment.md**
   - Create new template file
   - Include Failure Modes table (ID, Failure Mode, Probability, Impact, Severity)
   - Include Mitigation Strategies table (Failure Mode, Mitigation, Residual Probability, Residual Impact)
   - Include Overall Risk Assessment section (Pre/Post-Mitigation Risk, Risk Ranking)

### Example Updates

5. **Command Examples**
   - Update examples in /think command to demonstrate risk-first analysis
   - Update examples in /plan command to show risk-based approach selection

---

## Out of Scope

### NOT Included in This Implementation

1. **Approach 1 (Documentation-Only)** - Insufficient enforcement
2. **Approach 2 (Reordering)** - Partial solution, superseded by Approach 3
3. **Approach 4 (Gated Process)** - Over-engineered, not selected
4. **Approach 5 (Scoring System)** - False precision, not selected

### NOT Changed

1. **Existing quality mechanisms:**
   - Mandatory /plan workflow (CLAUDE.md lines 37-57) - Unchanged
   - Specification completeness verification (/plan Phase 2) - Unchanged
   - Test-driven development (acceptance tests first) - Unchanged
   - 100% test coverage requirement - Unchanged
   - 5-tier documentation hierarchy (GOV001) - Unchanged

2. **Workflow phases:**
   - /think 8-phase workflow - Structure unchanged, only comparison template modified
   - /plan Phases 1-3, 5-8 - Unchanged
   - /plan Phase 4 - Objective and process modified, but phase structure intact

3. **Other commands:**
   - /commit - No changes
   - /archive - No changes
   - /doc-name - No changes
   - /archive-plan - No changes

---

## Assumptions

1. **User has reviewed and approved /think analysis**
   - Approach 3 selected as preferred approach
   - Analysis document: wip/_deprioritize_effort_analysis_results.md

2. **Current documentation is accessible**
   - CLAUDE.md at project root is writable
   - .claude/commands/ directory contains think.md and plan.md
   - templates/ directory exists or can be created

3. **Implementation environment**
   - AI implementation time available (16-24 hours estimated)
   - Human review time available for validation

4. **Project charter goals remain quality-absolute**
   - PCH001 goals: "Flawless audio playback," "1970s FM radio experience"
   - Quality-first decision making aligns with charter

5. **Backward compatibility**
   - Existing /think and /plan analyses remain valid (structure preserved)
   - Changes enhance rather than replace existing mechanisms

---

## Constraints

### Technical Constraints

1. **File format:** Markdown (.md files)
2. **Encoding:** UTF-8
3. **Line endings:** Windows (CRLF) or Unix (LF) - project uses Windows
4. **Maximum line length:** None specified, follow existing style (~100-120 chars where possible)

### Process Constraints

1. **No code changes:** This implementation affects documentation and workflow templates only
   - No Rust source code modified
   - No database schema changes
   - No HTTP API changes

2. **Preserve existing content:**
   - Do not remove existing CLAUDE.md sections
   - Do not delete existing /think or /plan content
   - Add or modify, do not replace wholesale

3. **Maintain consistency:**
   - Follow existing CLAUDE.md style and formatting
   - Match existing /think and /plan command documentation structure
   - Use consistent terminology across all changes

### Resource Constraints

1. **Time:** Target 16-24 hours AI implementation time (per analysis estimate)
2. **Scope creep:** Resist adding features beyond Approach 3 specification
3. **Review bandwidth:** Human review time limited - changes must be clear and well-documented

---

## Success Criteria

### Functional Success

1. **CLAUDE.md updated:**
   - ✓ New "Decision-Making Framework - MANDATORY" section exists
   - ✓ Framework clearly states risk (primary), quality (secondary), effort (tertiary)
   - ✓ Rationale references PCH001 project charter
   - ✓ Section integrated seamlessly with existing content

2. **/think command updated:**
   - ✓ Comparison framework template restructured
   - ✓ Risk Assessment section appears first
   - ✓ Quality Characteristics section appears second
   - ✓ Implementation Considerations (effort) appears third
   - ✓ RISK-BASED RANKING section added
   - ✓ Recommendation format includes risk-based justification

3. **/plan command updated:**
   - ✓ Phase 4 objective changed to "minimal failure risk; acknowledge effort"
   - ✓ Phase 4 process steps detailed (risk assessment → ranking → selection → ADR)
   - ✓ Integration with existing /plan workflow seamless

4. **Risk assessment template created:**
   - ✓ templates/risk_assessment.md file exists
   - ✓ Includes Failure Modes table
   - ✓ Includes Mitigation Strategies table
   - ✓ Includes Overall Risk Assessment section
   - ✓ Template is usable (clear instructions, well-formatted)

5. **Examples updated:**
   - ✓ /think examples demonstrate risk-first analysis
   - ✓ /plan examples show risk-based approach selection

### Quality Success

1. **Consistency:** All changes use consistent terminology and formatting
2. **Clarity:** Changes are clear and unambiguous
3. **Completeness:** All 25 requirements satisfied
4. **Maintainability:** Changes follow existing documentation patterns
5. **Testability:** Changes can be verified through inspection (acceptance tests will define)

### User Acceptance

1. **Principal Developer approves:** Changes reviewed and accepted
2. **No regression:** Existing workflows still functional
3. **Improved decision-making:** Risk-first framework demonstrably used in subsequent decisions
4. **Reduced effort bias:** Decision language no longer co-weights risk and effort

---

## Dependencies

### Existing Assets

| Asset | Type | Location | Status | Required For |
|-------|------|----------|--------|--------------|
| CLAUDE.md | Documentation | Project root | Exists | REQ-RPF-010, REQ-RPF-020, REQ-RPF-030 |
| .claude/commands/think.md | Workflow | .claude/commands/ | Exists | REQ-RPF-080 through REQ-RPF-130 |
| .claude/commands/plan.md | Workflow | .claude/commands/ | Exists | REQ-RPF-140 through REQ-RPF-200 |
| templates/ directory | Directory | Project root | May need creation | REQ-RPF-210 |
| PCH001_project_charter.md | Reference | Project root | Exists (read-only) | Rationale for framework |

### External Dependencies

**None.** This implementation is self-contained within WKMP project documentation.

### Temporal Dependencies

1. **Prerequisite:** /think analysis completed and Approach 3 selected
   - Status: ✓ Complete (wip/_deprioritize_effort_analysis_results.md)

2. **Sequencing:** Implementation order
   - First: Create risk assessment template (foundation)
   - Second: Update CLAUDE.md (establishes policy)
   - Third: Update /think command (applies to analysis workflow)
   - Fourth: Update /plan command (applies to planning workflow)
   - Fifth: Update examples (demonstrates usage)

3. **Post-implementation:** Validation
   - Run next /think analysis using new framework
   - Run next /plan using new Phase 4 process
   - Verify risk-first language appears in outputs

---

## Risk Factors

| Risk | Probability | Impact | Mitigation | Residual |
|------|------------|--------|------------|----------|
| Framework changes ignored in practice | Medium | High | Include in MANDATORY sections, clear examples | Low-Medium |
| Learning curve delays adoption | Low | Medium | Provide templates, examples, clear documentation | Low |
| Increased verbosity of analyses | Low | Low | Already have verbosity standards (CLAUDE.md) | Very Low |
| Inconsistent application across commands | Medium | Medium | Update all commands simultaneously, use shared template | Low |
| User finds framework too rigid | Low | Medium | Framework allows tiebreakers (quality, effort) for equivalent risk | Low |

**Overall Implementation Risk:** Low

**Rationale:** Documentation-only changes with clear templates and examples. No code changes. Aligns with existing quality mechanisms. Reversible if issues found.

---

## Implementation Approach Preview

**This is NOT the implementation plan (Phase 5 will define increments). This is high-level approach confirmation.**

**General Approach:**
1. Create foundation (risk assessment template)
2. Establish policy (CLAUDE.md framework section)
3. Apply to workflows (/think, /plan command updates)
4. Document with examples
5. Validate through inspection and test usage

**Estimated Effort:** 16-24 hours AI implementation time
**Estimated Increments:** 8-12 increments (2-3 hours each)

---

**Scope Statement Status:** Complete - Ready for Phase 2 (Specification Completeness Verification)
