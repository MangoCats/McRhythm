# Document Hierarchy Checker

**Purpose:** Validate WKMP's 5-tier documentation hierarchy for consistency, circular references, and orphaned documents

**Task:** Perform comprehensive validation of documentation governance rules per GOV001.

---

## Instructions

You are validating the WKMP documentation hierarchy against governance rules. Follow these steps systematically to ensure documentation integrity.

---

## Background: 5-Tier Hierarchy

Per GOV001-document_hierarchy.md:

**Tier 0: Governance**
- GOV### documents
- Define documentation framework itself

**Tier 1: Authoritative Requirements**
- REQ### documents
- WHAT the system must do

**Tier 2: Design Specifications**
- SPEC### documents
- HOW requirements are satisfied

**Tier 3: Implementation Specifications**
- IMPL### documents
- Concrete implementation details

**Tier 4: Execution Plans**
- EXEC### documents
- WHEN features are built

**Cross-Cutting:**
- RPT### (Reports & Analysis)
- PLAN### (Implementation Plans)
- GUIDE### (Implementation Guides)
- REV### (Reviews)

**Information Flow Rules:**
- ✅ Downward references: Higher tier → Lower tier (normal)
- ❌ Upward references: Lower tier → Higher tier (controlled, must be justified)
- ❌ Circular references: Doc A → Doc B → Doc A (forbidden)

---

## Execution Steps

### Step 1: Catalog All Documentation

Scan project for all documentation files:

**Locations to scan:**
- `docs/*.md`
- `workflows/*.md`
- `project_management/*.md`
- `wip/*.md` and `wip/*/`

**For each document, extract:**
- File path
- Document prefix (GOV, REQ, SPEC, IMPL, EXEC, etc.)
- Tier level (0-4 or cross-cutting)
- Document number
- Title

**Output:** Complete document inventory with tier classifications

---

### Step 2: Extract Document References

For each document, find all references to other documents:

**Reference patterns to detect:**
- Markdown links: `[text](docs/GOV001-document_hierarchy.md)`
- Inline references: `per GOV001`, `see REQ001:45-78`
- Parenthetical refs: `(REQ001-requirements.md)`
- Section links: `[Requirements](docs/REQ001-requirements.md#section)`

**For each reference found:**
- Source document (where reference appears)
- Target document (what's being referenced)
- Line number of reference
- Context (3 words before/after)

**Output:** Directed graph of document dependencies

---

### Step 3: Validate Information Flow Direction

For each reference, check if it follows hierarchy rules:

**Valid (Downward Flow):**
- REQ → SPEC ✅
- SPEC → IMPL ✅
- IMPL → EXEC ✅
- GOV → Any ✅ (governance is tier 0)
- Any → GUIDE ✅ (guides are supplementary)

**Suspicious (Upward Flow - needs review):**
- SPEC → REQ ⚠️ (design informing requirements)
- IMPL → SPEC ⚠️ (implementation informing design)
- EXEC → IMPL ⚠️ (execution informing implementation)

**Invalid (Cross-tier jumps):**
- EXEC → REQ ❌ (skipping intermediate tiers)
- IMPL → REQ ❌ (skipping SPEC tier)

**Analysis:**
- Count valid vs. suspicious vs. invalid references
- Group suspicious references by tier pair
- Flag all invalid references for correction

---

### Step 4: Detect Circular References

Build directed graph and detect cycles:

**Algorithm:**
1. Start from each document
2. Follow outbound references depth-first
3. Track visited documents in path
4. If revisit a document in current path → cycle detected

**Circular Reference Types:**

**Direct cycles (2 documents):**
```
A → B
B → A
```

**Indirect cycles (3+ documents):**
```
A → B → C → A
```

**Report:**
- All cycles found
- Path showing cycle: A → B → C → A
- Line numbers where circular refs occur
- Severity: Critical (must fix)

---

### Step 5: Find Orphaned Documents

Identify documents with no incoming references:

**Legitimate orphans (exempt):**
- GOV001 (top of hierarchy)
- REQ001 (entry point for requirements)
- README.md files
- CLAUDE.md
- change_history.md
- Workflow guides (DWI###)

**Suspicious orphans (review needed):**
- SPEC### with no references from REQ or other SPEC
- IMPL### with no references from SPEC or IMPL
- GUIDE### with no references from any doc
- RPT### with no references (completed analysis not integrated)

**Report:**
- List of orphaned documents (exclude legitimate)
- Category breakdown (SPEC vs IMPL vs GUIDE)
- Recommendation: Either integrate or archive

---

### Step 6: Validate Document Number Sequences

Check for gaps in numbering:

**Expected sequences:**
- GOV001, GOV002, GOV003... (no gaps)
- REQ001, REQ002, REQ003... (no gaps)
- SPEC001, SPEC002... (gaps acceptable if archived)

**Cross-reference with:**
- `workflows/REG001_number_registry.md` (official number assignments)
- `workflows/REG002_archive_index.md` (archived documents)

**Report:**
- Number gaps found: "Missing REQ005 between REQ004 and REQ006"
- Validate against registry: "REQ005 archived on 2025-10-20"
- Unregistered documents: "SPEC009 not in registry"

---

### Step 7: Check Category Consistency

Validate document prefixes match location and content:

**Location rules (per REG003):**
- `docs/GOV###`, `docs/REQ###`, `docs/SPEC###`, `docs/IMPL###`, `docs/EXEC###` → Product docs
- `docs/RPT###`, `wip/PLAN###` → Analysis & planning
- `workflows/DWI###`, `workflows/REG###`, `workflows/TMPL###` → Workflow docs
- `project_management/LOG###` → Operational logs

**Violations:**
- SPEC### in `workflows/` ❌
- DWI### in `docs/` ❌
- REQ### in `wip/` ⚠️ (work in progress, acceptable temporarily)

**Report:**
- Documents in wrong location
- Recommended moves to fix

---

## Output Format

Generate comprehensive report: `wip/doc_hierarchy_validation_YYYY-MM-DD.md`

### Executive Summary (15 lines max)

```
🗂️  WKMP Document Hierarchy Validation Report

Documents scanned: 47
Tier 0 (GOV): 2
Tier 1 (REQ): 3
Tier 2 (SPEC): 8
Tier 3 (IMPL): 4
Tier 4 (EXEC): 1
Cross-cutting: 29

✅ Valid references: 234 (92%)
⚠️  Upward references: 15 (6%) - requires review
❌ Circular references: 2 (CRITICAL)
❌ Invalid tier jumps: 5 (MUST FIX)
⚠️  Orphaned documents: 8
```

### Critical Issues (Must Fix)

**Circular References:**
```
1. SPEC002-crossfade.md → IMPL001-database_schema.md → SPEC002-crossfade.md
   - SPEC002:145 references IMPL001
   - IMPL001:234 references SPEC002
   - ACTION: Remove upward reference from IMPL001
```

**Invalid Tier Jumps:**
```
1. EXEC001-implementation_order.md → REQ001-requirements.md (line 456)
   - EXEC (Tier 4) should not reference REQ (Tier 1) directly
   - Should reference through SPEC or IMPL
   - ACTION: Update to reference SPEC### instead
```

### Warnings (Review Recommended)

**Upward References (15 found):**
```
Group by tier pair:
- IMPL → SPEC: 8 occurrences
- SPEC → REQ: 5 occurrences
- EXEC → IMPL: 2 occurrences

Top 3 examples:
1. IMPL001:234 → SPEC002:45
2. SPEC007:567 → REQ001:123
3. IMPL003:89 → SPEC001:234
```

**Orphaned Documents (8 found):**
```
- docs/SPEC009-unused.md (no incoming references, candidate for archive)
- docs/GUIDE005-old_guide.md (superseded by GUIDE006)
- wip/RPT007_analysis.md (analysis not integrated into specs)
```

### Information Flow Diagram

```
[ASCII or Mermaid diagram showing document relationships]

GOV001 → REQ001 → SPEC001 → IMPL001 → EXEC001
         REQ001 → SPEC002 → IMPL001
         REQ002 → SPEC003

Circular: SPEC002 ↔ IMPL001 ❌
```

### Document Inventory by Tier

Table format:
```
| Tier | Prefix | Count | Orphaned | Invalid Refs |
|------|--------|-------|----------|--------------|
| 0    | GOV    | 2     | 0        | 0            |
| 1    | REQ    | 3     | 0        | 0            |
| 2    | SPEC   | 8     | 1        | 3            |
| 3    | IMPL   | 4     | 0        | 2            |
| 4    | EXEC   | 1     | 0        | 0            |
```

### Actionable Recommendations

**Immediate (Critical):**
1. Fix circular reference: SPEC002 ↔ IMPL001
2. Remove invalid tier jump: EXEC001 → REQ001
3. Resolve missing GOV002 (referenced but not found)

**Short-term (Important):**
1. Review 15 upward references for justification
2. Archive or integrate 8 orphaned documents
3. Update REG001 registry for 3 unregistered documents

**Long-term (Maintenance):**
1. Establish review process for upward references
2. Create "document integration checklist" for new RPT/PLAN docs
3. Quarterly orphan review

---

## Display to User

Show:
- Executive summary
- Critical issues (full list)
- Warning counts (summary only)
- Link to full report

---

## Performance Optimization

**Efficient scanning:**
- Use Glob to find all .md files first
- Use Grep to extract references in parallel
- Cache document catalog to avoid re-reading
- Build graph incrementally

**Limits:**
- Skip reading files >5000 lines (not documentation)
- Skip binary files, images
- Focus on `.md` files only

---

## Error Handling

**Missing documents:**
- Reference to `SPEC999.md` but file doesn't exist
- Report: "Broken reference: SPEC002:45 → SPEC999 (not found)"

**Malformed references:**
- Log warning: "Possible reference 'see spec 2' doesn't match standard format"
- Continue validation

**Unreadable files:**
- Skip file, report: "Could not read docs/corrupted.md"

---

## Integration with Governance

This check validates enforcement of:
- GOV001: Document hierarchy rules
- REG001: Number registry consistency
- REG002: Archive tracking
- REG003: Category definitions

**Suggested frequency:**
- Before major releases
- Monthly during active development
- After bulk documentation updates
- When governance rules change

---

## Success Criteria

✅ All documentation files scanned
✅ Reference graph built
✅ Information flow validated
✅ Circular references detected
✅ Orphans identified
✅ Number sequences checked
✅ Category consistency validated
✅ Comprehensive report generated
✅ Critical issues flagged with severity
✅ Actionable recommendations provided

---

**Expected runtime:** 30-90 seconds for full documentation scan
**Output file:** `wip/doc_hierarchy_validation_YYYY-MM-DD.md`
