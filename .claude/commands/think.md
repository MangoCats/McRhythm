# /think - Multi-Agent Document Analysis Workflow

## Command Signature
```
/think [document_path]
```

## Purpose
Initiate a dynamic multi-agent analytical workflow to thoroughly analyze a document containing questions, problems, desired changes, or action items related to the WKMP project. The workflow produces comprehensive answers, solutions, and option comparisons WITHOUT proceeding to implementation planning.

**Key Features:**
- 8-phase systematic analysis workflow
- Dynamic multi-agent strategy deployment
- Comprehensive research and synthesis
- Detailed option comparisons
- Results permanently recorded in project documentation
- Original input document preserved unchanged

## Input Parameters
- **document_path** (optional): Path to the document to analyze
  - If not provided: Prompt user to specify document path
  - Document should contain: questions, problems, changes, or actions regarding the project

## Workflow Execution Phases

### Phase 1: Document Identification and Initial Assessment
**Objective:** Locate and validate the target document

1. If `document_path` provided:
   - Validate file exists and is readable
   - Confirm with user if document path is correct
2. If `document_path` NOT provided:
   - Prompt user: "Please specify the document path to analyze:"
   - Wait for user input
   - Validate provided path

3. Read and perform initial assessment:
   - Document size and structure
   - Identify distinct topics/questions/problems
   - Catalog action items requiring research
   - Determine analysis complexity level

**Output:** Structured breakdown of document contents and analysis requirements

### Phase 2: Multi-Agent Strategy Design
**Objective:** Dynamically design optimal agent deployment strategy

Based on Phase 1 assessment, design agent roles and responsibilities:

**Core Agent Roles:**
1. **Project State Analyst**
   - Understand current implementation state
   - Map existing codebase structure
   - Identify relevant existing solutions
   - Locate applicable procedures and standards

2. **Documentation Analyst**
   - Review project documentation (CLAUDE.md, docs/, STRUCTURE.md if exists)
   - Extract relevant specifications and requirements
   - Identify established processes and workflows
   - Note documentation gaps or conflicts

3. **Research Agent** (deployed as needed)
   - Conduct internet research for external information
   - Investigate industry standards and best practices
   - Research technical solutions and approaches
   - Validate assumptions against external sources

4. **Integration Analyst**
   - Synthesize findings from other agents
   - Identify connections between problems and solutions
   - Map questions to available information
   - Formulate comprehensive answers

**Strategy Design Considerations:**
- Number of topics requiring parallel analysis
- Depth of codebase investigation required
- Extent of internet research needed
- Dependencies between analysis tasks

**Output:** Agent deployment plan with clear responsibilities and coordination strategy

### Phase 3: Parallel Information Gathering
**Objective:** Execute comprehensive, systematic information collection

**Project State Investigation:**
- Map relevant source code locations
- Identify existing implementations related to topics
- Document current state of features/components
- Note unimplemented or incomplete areas
- Catalog test coverage

**Documentation Review:**
- Extract relevant sections from all documentation
- Map requirements to specifications
- Identify design decisions and rationales
- Note established conventions and procedures
- Document traceability chains

**Process Understanding:**
- Review established workflows (e.g., /commit, /doc-name)
- Understand development methodology
- Note quality standards and practices
- Identify architectural considerations

**Agent Communication Format (Internal):**
```
AGENT: [Role]
TOPIC: [Specific topic/question]
FINDINGS:
- [Concise factual finding 1]
- [Concise factual finding 2]
SOURCE: [File/location references]
GAPS: [Missing information needed]
NEXT: [Suggested next investigation step]
```

**Output:** Comprehensive knowledge base assembled from all sources

### Phase 4: Targeted Internet Research (Conditional)
**Objective:** Obtain external information when project sources insufficient

**Trigger Conditions:**
- Question requires knowledge beyond project scope
- Industry standards or best practices needed
- Technical approach validation required
- Audio/music technology guidance needed
- Technology comparison requested

**Research Protocol:**
1. Formulate specific, targeted search queries
2. Execute searches using web_search tool
3. Evaluate source credibility and relevance
4. Extract key information and references
5. Document findings with citations

**Agent Communication Format (Internal):**
```
RESEARCH: [Specific question/topic]
QUERY: [Search terms used]
FINDINGS:
- [Key finding 1 with source URL]
- [Key finding 2 with source URL]
RELEVANCE: [How this applies to project]
CONFIDENCE: [High/Medium/Low based on source quality]
```

**Output:** Curated research findings with source citations

### Phase 5: Analysis and Synthesis
**Objective:** Transform gathered information into actionable answers and solutions

**For Each Question:**
1. Consolidate all relevant findings
2. Apply software engineering standards
3. Consider architectural implications
4. Formulate complete, accurate answer
5. Identify any assumptions or caveats
6. Document reasoning chain

**For Each Problem:**
1. Analyze root causes based on findings
2. Generate potential solution approaches
3. Evaluate each approach against:
   - Technical feasibility
   - WKMP architecture requirements
   - Existing codebase patterns
   - Development effort required
   - Risk factors
4. Identify solution dependencies
5. Note any specification gaps or conflicts

**For Each Change/Action:**
1. Understand desired outcome
2. Assess current state vs. desired state
3. Identify available approaches
4. Map to existing project capabilities
5. Surface technical considerations

**Output:** Structured analysis for each topic with supporting evidence

---

### CRITICAL DISTINCTION: Analysis vs. Implementation Planning

**This section defines the boundary between /think (analysis) and /plan (implementation planning).**

#### ALLOWED in /think (Analysis):

✓ **Describing WHAT a solution approach does:**
```
"Option A involves renaming files using an automated script that reads
 a mapping file and performs git mv operations to preserve history."
```

✓ **Listing COMPONENTS of an approach:**
```
"This approach requires: (1) category definitions, (2) number registry,
 (3) automation script, (4) reference updating."
```

✓ **Qualitative effort assessment:**
```
"Effort estimate: Medium (2-4 hours) due to need to update references
 across multiple files."
```

✓ **High-level process outline:**
```
"The general workflow would be: gather requirements → design structure →
 implement → test → document."
```

✓ **Conceptual examples:**
```
"For example, a test case might verify that: given a document named
 'analysis.md' in category RPT, the tool assigns 'RPT001_analysis.md'
 and updates the registry."
```

✓ **Testing strategy overview:**
```
"Testing would require unit tests for core functionality, integration
 tests for component interaction, and system tests for end-to-end workflows."
```

---

#### NOT ALLOWED in /think (Implementation Planning):

✗ **Phased implementation plan with tasks:**
```
FORBIDDEN:
Phase 1: Setup (2 hours)
Tasks:
1. Create folder structure
2. Initialize registry file
3. Document workflow
```

✗ **Executable commands or code:**
```
FORBIDDEN:
mkdir -p docs
git mv wip/file.md docs/file.md
cp template.md new_doc.md
```

✗ **Detailed test specifications ready to implement:**
```
FORBIDDEN:
TC-U-001-01: Test configuration parsing
Given: Config file with <enabled="true">
When: ConfigManager::load() called
Then: isEnabled() returns true
Pass Criteria: ASSERT_TRUE(config.isEnabled())
Test Data: [complete TOML file content]
```

✗ **Step-by-step implementation instructions:**
```
FORBIDDEN:
"Step 1: Create REG001_number_registry.md with table headers.
 Step 2: For each existing document, assign next available number.
 Step 3: Use git mv to rename each file.
 Step 4: Update all references."
```

✗ **File structures or code templates:**
```
FORBIDDEN:
struct ConfigManager {
    enabled: bool,
}

impl ConfigManager {
    pub fn load(path: &str) -> Result<Self> {
        // implementation
    }
}
```

✗ **Increment/task breakdowns:**
```
FORBIDDEN:
Increment 1: Parser implementation
  - Task 1.1: Create parser struct
  - Task 1.2: Add unit tests
Increment 2: Registry management
  - Task 2.1: Implement CRUD operations
```

✗ **Specific deliverables per phase:**
```
FORBIDDEN:
Deliverable: File REG001_number_registry.md created with columns:
Number, Document Name, Date, Category, Method
```

✗ **"Next Steps" with implementation tasks:**
```
FORBIDDEN:
Next Steps:
1. Create category_definitions.md file
2. Implement /doc-name command
3. Test with sample documents
4. Update CLAUDE.md

ALLOWED:
Next Steps: Run /plan to create detailed implementation plan
```

---

#### Borderline Cases - How to Handle:

**❓ User asks: "How should we implement X?"**

**CORRECT Response (Analysis):**
```
"Three approaches exist to implement X:

**Approach A: Manual process**
- Description: Users manually perform steps following documented procedure
- Advantages: No tooling needed, full control
- Disadvantages: Error-prone, time-consuming
- Effort: High for initial migration, low for ongoing

**Approach B: Semi-automated**
- Description: Script assists but requires user validation at checkpoints
- Advantages: Reduces errors, maintains user oversight
- Disadvantages: Requires script development
- Effort: Medium upfront, low ongoing

**Approach C: Fully automated**
- Description: Workflow command handles entire process
- Advantages: Consistent, fast, scalable
- Disadvantages: Complex development, less flexibility
- Effort: High upfront, very low ongoing

To develop a detailed implementation plan for your chosen approach,
use the /plan workflow."
```

**WRONG Response (Implementation Planning):**
```
"Here's how to implement X:

Phase 1: Create these files...
Tasks:
1. mkdir -p docs
2. Create REG001_number_registry.md
3. Populate with existing documents

Phase 2: Run these commands...
[bash script with git mv commands]

Phase 3: Test using...
[detailed test cases with pass criteria]"
```

---

**❓ User asks: "What tests would be needed for X?"**

**CORRECT Response (Test Strategy):**
```
"Testing strategy for X would require:

**Unit Test Coverage:**
- Core functionality: Verify X performs expected transformation
- Error handling: Validate behavior with invalid inputs
- Edge cases: Test boundary conditions (empty input, max size, etc.)

**Integration Test Coverage:**
- X interacting with component Y
- End-to-end workflow including X

**System Test Coverage:**
- User-facing scenarios exercising X in realistic conditions

Test specifications should be developed during /plan phase using
acceptance test-driven development (Given/When/Then format)."
```

**WRONG Response (Detailed Test Specs):**
```
"Tests needed for X:

### TC-U-001-01: Test X with valid input
**Requirement:** REQ-F-001
**Given:** Input file contains valid data
**When:** X.process() is called
**Then:** Output matches expected result
**Pass Criteria:** assert_eq!(output, expected)
**Test Data:**
[complete test file content]

### TC-U-001-02: Test X with invalid input
[15 more detailed test cases with complete specifications...]"
```

---

#### Self-Check Before Proceeding to Phase 6:

**Ask yourself:**
→ "Could user copy-paste this content and start implementing without additional planning?"
→ If **YES**: It's implementation planning — **REVISE** to be conceptual
→ If **NO**: It's analysis — **ACCEPTABLE**

**Mental validation:**
- ✓ Content describes WHAT solutions are, not HOW to implement them
- ✓ No bash commands, code snippets, or executable instructions
- ✓ No phased implementation plans with tasks/deliverables
- ✓ No detailed test specifications (conceptual test strategy only)
- ✓ Examples remain illustrative, not copy-paste templates

**If any check fails:** Revise violating content before proceeding to Phase 6.

---

### Phase 6: Options Comparison and Presentation
**Objective:** Present detailed comparisons enabling informed decisions

**Comparison Framework:**
For each problem/change with multiple approaches:

```
TOPIC: [Problem/Change description]

APPROACH 1: [Name/Description]
Advantages:
- [Advantage 1 with justification]
- [Advantage 2 with justification]
Disadvantages:
- [Disadvantage 1 with justification]
- [Disadvantage 2 with justification]
Technical Considerations:
- [Consideration 1]
Effort Estimate: [Qualitative: Low/Medium/High]
Risk Level: [Low/Medium/High with explanation]
Architecture Impact: [Description]
Alignment with Project: [How well it fits existing patterns]

APPROACH 2: [Name/Description]
[Same structure as Approach 1]

RECOMMENDATION FACTORS:
- [Factor 1: consideration for decision]
- [Factor 2: consideration for decision]

Note: This analysis provides options; decision authority remains with stakeholders.
```

**Presentation Standards:**
- Use clear, professional formatting
- Include code references where applicable
- Cite specific documentation sections
- Reference relevant standards
- Provide traceability to source material

### Phase 7: Executive Summary
**Objective:** Provide brief, actionable summary of entire analysis

**Summary Structure:**
```
ANALYSIS SUMMARY: [Document Name]

QUESTIONS ADDRESSED: [Count]
[Brief 1-line summary of each question and answer]

PROBLEMS ANALYZED: [Count]
[Brief 1-line summary of each problem and recommended approach]

CHANGES EVALUATED: [Count]
[Brief 1-line summary of each change and key considerations]

RESEARCH CONDUCTED:
- [Research area 1] - [Key insight]
- [Research area 2] - [Key insight]

CRITICAL FINDINGS:
- [Most important finding 1]
- [Most important finding 2]
- [Most important finding 3]

DECISIONS REQUIRED:
- [Decision point 1]
- [Decision point 2]

NEXT STEPS:
This analysis is complete. Implementation planning requires explicit user authorization.

**To proceed with implementation:**
1. Review analysis findings and select preferred approach(es)
2. Make any necessary decisions on identified decision points
3. Run `/plan [specification_file]` to create detailed implementation plan
4. /plan will generate: requirements analysis, test specifications, increment breakdown

**User retains full authority over:**
- Whether to implement any recommendations
- Which approach to adopt
- When to proceed to implementation
- Modifications to suggested approaches

**This section MUST NOT contain:**
- Task lists directing specific implementation work
- Commands to execute
- Step-by-step implementation instructions
- Phased plans with deliverables
```

**MANDATORY:** The NEXT STEPS section must follow this exact format. Do not add implementation tasks or action items.

### Phase 8: Results Recording and Documentation
**Objective:** Ensure analysis results are permanently recorded in project documentation

**Recording Decision Logic:**

1. **Assess Analysis Complexity:**
   - **Trivial:** Single short answer, <100 words total
   - **Non-Trivial:** Everything else (multiple questions, detailed comparisons, >100 words)

2. **For Trivial Analysis:**
   - Results may remain only in conversation
   - Optionally add brief note to input document
   - User discretion on recording

3. **For Non-Trivial Analysis (REQUIRED):**
   - Results MUST be permanently recorded
   - Follow two-tier output structure (MANDATORY - see below)
   - Apply size-based recording approach

**MANDATORY Two-Tier Output Structure:**

**REQ-002 from PLAN002: All analyses MUST follow this structure**

**Executive Summary (<300 lines) - ALWAYS REQUIRED:**
- Problems addressed (2-3 sentences each)
- Critical findings (5 bullet points max)
- Recommendation (1 paragraph)
- Next steps (5 bullets max)
- Links to detailed sections if modular

**Detailed Content - Size-Based Approach:**

**IF total analysis ≤ 1200 lines:**
- Single file structure acceptable
- Executive summary at top (as above)
- Detailed analysis follows
- File: `[input]_analysis_results.md`

**IF total analysis > 1200 lines (MANDATORY modular structure):**
- Create folder: `[input]_analysis/`
- Structure:
  ```
  00_SUMMARY.md              <500 lines (expanded executive summary)
  01_[topic].md              <300 lines per topic
  02_[topic].md              <300 lines per topic
  ...
  FULL_ANALYSIS.md           (consolidated, for archival only)
  ```
- Each section file: <300 lines
- Summary provides navigation to sections

**Enforcement:**
- Before writing results: Count lines in generated content
- If >1200 lines: DO NOT write as single file, create modular structure
- Inform user if restructuring applied

**Output Location Decision Tree:**

1. **Analysis results go to:** `docs/` directory
   - Use /doc-name workflow to assign RPT### prefix
   - Follow WKMP's 13-category document system
   - Register in workflows/REG001_number_registry.md

2. **Large analysis work-in-progress:**
   - May create temporary folder in `wip/RPT###_*/`
   - Move to docs/ when complete
   - Use /archive workflow for final organization

3. **Input documents can be in:**
   - `wip/` (work in progress)
   - `docs/` (existing documentation)
   - `project_management/` (planning artifacts)
   - Any project location

**Size-Based Recording Approach (Context Window Optimized):**

**For Compact Analysis (<300 lines or <15KB):**
- MAY embed results in input document's "After Analysis" section
- Include complete findings inline
- Use clear section headers
- Single document is manageable for future reference

**For Medium Analysis (300-1500 lines or 15-75KB) - STANDARD APPROACH:**
1. **Create Separate Results Document**
   - Naming convention: `[input_filename]_analysis_results.md`
   - Location: Same directory as input document (or docs/ for formal reports)
   - Content: Complete detailed analysis with all findings
   - Structure: Executive summary → Detailed analysis → Appendices

2. **Update Input Document with Summary**
   - Locate "After Analysis" section (or create if missing)
   - Add analysis metadata (date, method)
   - Add executive summary (1-page maximum, ~300 lines)
   - Add link to detailed results document
   - Include "Next Steps" or "Decision Required"

**For Large Analysis (>1500 lines or >75KB) - MODULAR APPROACH:**

When analysis is very large, use modular folder structure:

```
[input_filename]_analysis/
├── 00_ANALYSIS_SUMMARY.md          # <500 lines - READ THIS FIRST
├── 01_questions_addressed.md       # Detailed answers to questions
├── 02_problems_analyzed.md         # Root causes and solutions
├── 03_approaches_compared.md       # Detailed option comparisons
├── 04_recommendations.md           # Detailed recommendations with rationale
├── 05_implementation_guidance.md   # How to proceed (if applicable)
└── FULL_ANALYSIS.md               # Consolidated (for archival/reference)
```

**Modular Structure Benefits:**
- Read only summary (~500 lines) for quick understanding
- Drill into specific sections as needed
- Total context: ~500 lines (not 2000+)
- Full analysis available when comprehensive view required

**Document Size Targets:**

| Document Type | Target Size | When to Read |
|---------------|-------------|--------------|
| Input doc summary | <300 lines | Always start here |
| Analysis summary | <500 lines | First read for any analysis |
| Section detail (modular) | <500 lines | When need specific topic |
| Full analysis (monolithic) | 300-1500 lines | When need complete context |
| Full analysis (modular) | >1500 lines | For archival, reference specific sections only |

**Input Document Editing Constraints - CRITICAL:**

**ALLOWED Edits (ONLY):**
- ✅ Add analysis date/metadata to "After Analysis" section
- ✅ Add executive summary of findings
- ✅ Add link(s) to detailed results document(s)
- ✅ Add "Next Steps" or "Decision Required" statements
- ✅ Update document status field if present

**PROHIBITED Edits:**
- ❌ Modify ANY original content of input document
- ❌ Change questions, problems, or desired changes sections
- ❌ Edit document information section
- ❌ Alter comparison criteria
- ❌ Modify expected output section
- ❌ Change any pre-analysis content

**Rationale for Constraint:**
- Preserves original request for audit trail
- Separates question from answer
- Allows comparison of what was asked vs what was delivered
- Maintains historical accuracy

**Results Document Structure:**

**For Standard Single-File Analysis (300-1500 lines):**

```markdown
# Analysis Results: [Topic from Input Document]

**Analysis Date:** YYYY-MM-DD
**Document Analyzed:** [relative path to input document]
**Analysis Method:** 8-Phase Multi-Agent Workflow (/think command)
**Analyst:** Claude Code (Software Engineering methodology)
**Stakeholders:** [From input document]
**Timeline:** [From input document]

---

## Executive Summary (Target: <500 lines - Read this first)

**Quick Navigation:**
- Questions Addressed: [count]
- Problems Analyzed: [count]
- Approaches Compared: [count]
- Recommendation: [one-line summary]
- Decisions Required: [count]

### Questions Addressed
[Brief summaries with answers - 1-2 paragraphs each]

### Problems Analyzed
[Root causes and recommended solutions - 1-2 paragraphs each]

### Changes Evaluated
[Options and key considerations - 1-2 paragraphs each]

### Critical Findings
1. [Most important finding 1]
2. [Most important finding 2]
3. [Most important finding 3]

### Recommendation
[Primary recommendation if applicable - 1 paragraph]

### Decisions Required
[What stakeholder must decide - bulleted list]

---

## Detailed Analysis (Progressive disclosure - read as needed)

### Current State Assessment
[Comprehensive current state - expand on summary above]

### [Topic 1]
[Detailed findings - complete analysis]

### [Topic 2]
[Detailed findings - complete analysis]

[Continue with all detailed sections...]

---

## Solution Options - Detailed Comparison (Skip if already decided)

### APPROACH 1: [Name]
[Complete details - only read if evaluating options]

### APPROACH 2: [Name]
[Complete details - only read if evaluating options]

[Continue for all approaches...]

---

## Comparison Matrix (Quick reference)
[If multiple approaches compared - table format for quick scanning]

---

## Decision Guidance (When ready to decide)
[Factors to help choose between options]

---

## Recommendation (Detailed rationale)
[Detailed recommendation with complete justification]

---

**Analysis Complete**
**Document Status:** Ready for stakeholder decision
```

**For Modular Large Analysis (>1500 lines):**

**00_ANALYSIS_SUMMARY.md** (Target: <500 lines)
```markdown
# Analysis Summary: [Topic]

**Quick Reference:**
- **Status:** [Complete/In Progress]
- **Questions:** [count] - See [01_questions_addressed.md]
- **Problems:** [count] - See [02_problems_analyzed.md]
- **Approaches:** [count] - See [03_approaches_compared.md]
- **Recommendation:** [one sentence] - See [04_recommendations.md]

## Executive Summary (5-minute read)

[Condensed findings - key points only]

## Critical Findings (1-minute read)

1. [Finding 1 - one sentence]
2. [Finding 2 - one sentence]
3. [Finding 3 - one sentence]

## Recommendation (30-second read)

[One paragraph - what to do]

## Document Map (Navigation guide)

**For Quick Overview:**
- Read this summary only (~400 lines)

**For Specific Topics:**
- Questions: [01_questions_addressed.md] (~400 lines)
- Problems: [02_problems_analyzed.md] (~500 lines)
- Approach comparison: [03_approaches_compared.md] (~600 lines)
- Recommendation detail: [04_recommendations.md] (~300 lines)

**For Complete Context:**
- Full consolidated analysis: [FULL_ANALYSIS.md] (2500 lines)
- Use only when comprehensive view required

## Next Steps

[Immediate actions required]
```

**Per-Section Files** (Target: 300-500 lines each)

Each section file contains:
- Brief context (links back to summary)
- Detailed analysis for that topic
- References to related sections
- Navigation (previous/next section)

**Input Document Summary Format:**

```markdown
## After Analysis

**Analysis Date:** YYYY-MM-DD
**Analysis Method:** `/think` Multi-Agent Workflow (8-Phase Analysis)
**Analysis Output:** [link to results document or "See below"]

### Quick Summary

**[Primary Topic]:**
[One-paragraph overview of findings]

**Options Analyzed:** [Count]
1. [Option 1 name] - [One line description]
2. [Option 2 name] - [One line description]
[etc.]

**Recommendation:** [Option name if applicable]
- [Key reason 1]
- [Key reason 2]

**Key Findings:**
- [Finding 1 - one line]
- [Finding 2 - one line]
- [Finding 3 - one line]

**See full analysis document for:**
- [Detail type 1]
- [Detail type 2]
- [Detail type 3]

**Next Step:** [What user should do next]
```

**Execution Steps:**

1. **Assess complexity and size of analysis**
   - Count lines of analysis content
   - Determine if trivial vs non-trivial

2. **For non-trivial analysis:**
   - If ≥300 lines: Create separate results document
   - If <300 lines: May embed or separate (separate recommended)

3. **Create results document (if separate):**
   - Generate filename: `[input_basename]_analysis_results.md`
   - Write complete analysis to file
   - Verify file created successfully

4. **Update input document:**
   - Read input document
   - Locate "After Analysis" section (or equivalent)
   - If section doesn't exist: Append new section at end
   - Use Edit tool to ONLY add summary (never modify original content)
   - Include link to results document if separate
   - Verify edit only added content, didn't modify existing

5. **Confirm with user:**
   - State where analysis recorded
   - Provide file path(s)
   - Confirm next steps

**Quality Checks:**

Before completing Phase 8:
- ✓ Analysis results are permanently recorded (file created)
- ✓ Input document updated with summary (if non-trivial)
- ✓ Original input content unchanged (verified)
- ✓ Links resolve correctly (if separate document)
- ✓ User informed of recording location(s)

**Example Execution:**

```
Analysis complete (estimated 600 lines).
Non-trivial analysis detected - creating separate results document.

Created: docs/RPT001_think_integration_analysis_results.md (600 lines)
Updated: wip/think_integration.md (added summary + link)
Original document content preserved unchanged.

Results include:
- Executive summary with 5 key findings
- Detailed analysis of current state
- 5 solution approaches with comprehensive comparisons
- Recommendation: Approach 5 (Hybrid)
- Implementation roadmap

Next step: Review analysis and decide which approach to implement.
```

---

## Context Window Management During Analysis

### Challenge
Large input documents or complex multi-topic analyses can overwhelm context windows, reducing analysis quality and increasing cognitive load.

### Strategy 1: Chunked Document Analysis

**For Large Input Documents (>1500 lines):**

1. **Initial Pass: Extract Topics Index**
   - Read full document once
   - Create topic index: Topic name, type (question/problem/change), line numbers
   - Output: topics_index.md (~50-200 lines)

2. **Per-Topic Deep Analysis**
   - For each topic in index:
     - Read topic + surrounding context (±30 lines)
     - Perform analysis for that topic
     - Record findings
   - Process 3-5 related topics at a time
   - Batch similar topics together

3. **Synthesis and Integration**
   - Read topics_index.md (compact form)
   - Read all findings (organized by topic)
   - Identify connections and patterns
   - Generate integrated analysis

**Benefits:**
- Never load full 2000-line document repeatedly
- Only topic under analysis + index in context
- Can handle arbitrarily large input documents

### Strategy 2: Progressive Research Integration

**For Complex Topics Requiring Extensive Research:**

Instead of loading all research results into context:

1. **Research Planning Phase**
   - Identify what research is needed
   - Formulate specific queries
   - Create research_index.md (queries + findings summary)

2. **Per-Query Research**
   - Execute one research query
   - Extract key findings (3-5 bullet points)
   - Add to research_index.md
   - Clear detailed results from context

3. **Research Synthesis**
   - Read research_index.md (all findings, compact)
   - Synthesize without re-loading full research details
   - Reference detailed findings only when needed

**Benefits:**
- Compact research summary (~200 lines) vs. full results (~2000+ lines)
- All findings accessible via index
- Focus on synthesis, not drowning in details

### Strategy 3: Modular Output Generation

**Output Size Decision Tree:**

```
Is analysis >1500 lines?
├─ NO: Single file (standard approach)
│  └─ Structure: Summary → Details → Appendices
│
└─ YES: Modular folder structure
   ├─ 00_ANALYSIS_SUMMARY.md (<500 lines)
   ├─ 01_questions_addressed.md (<500 lines)
   ├─ 02_problems_analyzed.md (<500 lines)
   ├─ 03_approaches_compared.md (<600 lines)
   ├─ 04_recommendations.md (<300 lines)
   └─ FULL_ANALYSIS.md (consolidated, archival)
```

**When to Use Modular:**
- Analysis exceeds 1500 lines
- Multiple distinct topics (5+ questions/problems)
- Detailed option comparisons (3+ approaches with full analysis)
- Extensive research findings
- Complex architectural considerations

### Strategy 4: Smart Output Messaging

**Provide Context Window Guidance to User:**

```markdown
**For Standard Analysis (<1500 lines):**
"Analysis complete: [filename]_analysis_results.md (850 lines)"
"Start with Executive Summary (lines 1-300)"
"Read full document for complete context"

**For Large Analysis (>1500 lines):**
"Analysis complete in modular structure: [filename]_analysis/"
"**Read ONLY:** 00_ANALYSIS_SUMMARY.md (400 lines)"
"For specific topics, see section files (300-500 lines each)"
"Do NOT read FULL_ANALYSIS.md unless comprehensive view needed"
```

### Context Window Budgets by Activity

| Activity | Documents Needed | Approx Lines | Context Used |
|----------|-----------------|--------------|--------------|
| **Quick Review** | Analysis summary only | 300-500 | Minimal |
| **Decision Making** | Summary + recommendations | 600-800 | Low |
| **Deep Dive on Topic** | Summary + specific section | 800-1000 | Medium |
| **Complete Understanding** | Full analysis (if <1500 lines) | 1000-1500 | Medium-High |
| **Comprehensive Review** | FULL_ANALYSIS.md (modular) | 2000+ | High (but organized) |

### Key Principle

**Context window management is about ensuring the right information is available at the right time in the right amount.**

For /think workflow:
- Input: Process large documents in chunks
- Analysis: Keep research and findings organized compactly
- Output: Modular structure for large analyses
- Consumption: Explicit guidance on what to read

### Adherence to Global Standards

**This workflow MUST follow CLAUDE.md standards:**
- Executive summaries: <300 lines (already specified in Phase 8, line 539)
- Detailed sections: <300 lines each (already specified)
- Modular structure if >1200 lines (already specified, line 558)
- Verbosity targets: 20-40% shorter than comprehensive draft (reinforced from CLAUDE.md)
- Reading protocol: Summary-first, targeted drill-down (reinforced from CLAUDE.md)

**Note:** These standards are already built into `/think` workflow design. This note confirms alignment with project-wide CLAUDE.md standards.

---

## Workflow Constraints

### MUST DO:
- Read and understand complete source document (using chunked analysis if >1500 lines)
- Systematically review all relevant project documentation
- Research ALL agents' findings thoroughly before synthesis
- Present multiple options when alternatives exist
- Include detailed comparisons with justifications
- Cite sources and references
- Consider WKMP architectural context
- Apply rigorous engineering standards
- Record non-trivial analysis results permanently (Phase 8)
- Update input document with summary and link (Phase 8)
- Preserve original input document content unchanged (Phase 8)

### MUST NOT DO:
- Skip or skim source material
- Proceed to implementation planning
- Create implementation-specific task lists
- Generate code or file structures
- Make decisions on behalf of stakeholders
- Provide recommendations without detailed comparison
- Assume specifications without verification
- Modify original content of input document (Phase 8 constraint)
- Edit questions, problems, or comparison criteria in input document (Phase 8 constraint)
- Leave non-trivial analysis results unrecorded (Phase 8 requirement)
- Create monolithic analysis documents >1500 lines without modular structure
- Waste context window by repeatedly loading full large documents

### Agent Communication Rules:
**Internal (Agent-to-Agent):**
- Concise, structured format
- Fact-focused, minimal prose
- Clear source attribution
- Explicit gaps/needs identification
- Machine-readable when possible

**External (Final Output to User):**
- Professional report format
- Clear section organization
- Detailed explanations and justifications
- Appropriate technical depth
- Executive summary for quick understanding

## Quality Standards

### Completeness:
- ALL questions in document addressed
- ALL problems analyzed with multiple solutions where applicable
- ALL changes evaluated with considerations
- NO topics left unaddressed without explicit reason

### Accuracy:
- Findings verified against source material
- No assumptions stated as facts
- Uncertainties clearly identified
- External research properly cited

### Traceability:
- Every answer/solution linked to evidence
- Documentation references provided
- Code locations cited when relevant
- Research sources attributed

### Software Engineering Appropriateness:
- Architectural considerations addressed
- Technical implications considered
- Quality standards applied
- Risk factors identified

## Success Criteria

The `/think` workflow is successful when:
1. ✓ Complete document analyzed systematically
2. ✓ All relevant project state understood
3. ✓ Necessary internet research conducted
4. ✓ Comprehensive answers formulated
5. ✓ Multiple options compared in detail
6. ✓ Professional presentation delivered
7. ✓ Executive summary provided
8. ✓ Non-trivial analysis results permanently recorded
9. ✓ Input document updated with summary (original content preserved)
10. ✗ NO implementation plans created (must not happen - see CRITICAL DISTINCTION section)
11. ✗ NO phased plans with tasks/deliverables (must not happen)
12. ✗ NO executable commands or code templates (must not happen)
13. ✗ NO detailed test specifications with TC-IDs and pass criteria (must not happen)
14. ✗ NO step-by-step implementation instructions (must not happen)
15. ✗ NO modifications to original input document content (must not happen)
16. ✓ User equipped to make informed decisions
17. ✓ NEXT STEPS section follows mandatory boilerplate (refers to /plan)
18. ✓ Context window management: Large inputs (>1500 lines) processed in chunks
19. ✓ Context window management: Large outputs (>1500 lines) in modular structure
20. ✓ Context window management: Analysis summary <500 lines for quick review
21. ✓ Context window management: User guidance on what to read provided

## Workflow Execution Verification Checklist

**Before marking /think workflow complete, verify ALL phases executed:**

**Analysis Phase:**
- [ ] Input document read and understood completely
- [ ] Relevant project state loaded (specs, architecture, prior analyses)
- [ ] Internet research conducted if needed (web_search for current info)
- [ ] Source material analysis complete (chunked if >1500 lines)

**Synthesis Phase:**
- [ ] Questions answered comprehensively
- [ ] Problems analyzed with root causes identified
- [ ] Multiple approaches evaluated with pros/cons
- [ ] Recommendations provided with clear rationale
- [ ] Executive summary created (<500 lines)

**Output Phase:**
- [ ] Analysis results written to file (if non-trivial)
- [ ] Original input document updated with summary (content preserved)
- [ ] Modular structure used if output >1500 lines
- [ ] User guidance provided on what to read

**CRITICAL VERIFICATIONS (Must NOT happen):**
- [ ] ✗ NO implementation plans created (stop if found)
- [ ] ✗ NO phased plans with tasks/deliverables (stop if found)
- [ ] ✗ NO test specifications with TC-IDs (stop if found)
- [ ] ✗ NO step-by-step implementation instructions (stop if found)
- [ ] ✗ NEXT STEPS follows mandatory boilerplate (must refer to /plan)

**Common Violations:**
- ❌ Creating implementation plans - Use /plan instead
- ❌ Modifying original input content - Only add summary
- ❌ Skipping executive summary - Always required
- ❌ Outputting >1500 lines without modular structure

**If workflow contamination detected:**
1. Stop immediately
2. Remove contaminating content
3. Redirect user to appropriate workflow (/plan)
4. Document for pattern recognition

## Error Handling

**Document Not Found:**
- Clearly state file does not exist
- Prompt for correct path
- Offer to list similar filenames in workspace

**Incomplete Information:**
- Explicitly state what information is missing
- Document assumptions made (if any)
- Recommend additional information sources
- Note impact on analysis confidence

**Conflicting Specifications:**
- Identify all conflicts discovered
- Present each conflicting position
- Explain implications of each interpretation
- Recommend specification clarification process

**Research Limitations:**
- State when internet research is inconclusive
- Document best available information
- Note confidence level
- Suggest alternative information sources

## Integration with Project Standards

This command operates within project context:
- Respects established development workflow
- Considers WKMP microservices architecture requirements
- Applies software engineering problem-solving methodology
- Maintains focus on quality and traceability
- Follows DRY principle in documentation review
- Does not modify protected files (e.g., project_management/change_history.md)
- Implements context window management for AI and human consumers
- Supports chunked processing of large documents
- Generates modular output for complex analyses

## Example Invocations

```bash
/think docs/research/investigation_notes.md
```

```bash
/think wip/requirements_questions.md
```

```bash
/think
# System prompts: "Please specify the document path to analyze:"
# User provides: wip/new_feature_questions.md
```

## Related Commands
- `/plan` - For creating implementation plans from analysis results
- `/commit` - For committing changes after implementation
- `/archive` - For organizing completed analysis documents
- `/doc-name` - For assigning document categories and numbers

---

**VERSION:** 1.0-wkmp
**AUTHOR:** WKMP Music Player Development Team
**DATE:** 2025-10-25
**ADAPTED FROM:** Cursor AI /think workflow
