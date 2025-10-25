# /doc-name

### Task Description: Document Prefix Assignment

Assign a CAT###_ prefix to a document following the documentation governance system.

**Execution Style:** Interactive with user confirmation before renaming.

**Prerequisites:**
- Document exists and is ready for prefix assignment
- Document not already prefixed (doesn't match `CAT###_*.md` pattern)
- Document not exempted (README.md, CLAUDE.md, .claude/commands/*, change_history.md, etc.)

---

## Workflow Steps

### 1. **Validate Input Document**

**Given:** User provides document path (e.g., `docs/my_analysis.md`)

a) **Check if file exists:**
   ```bash
   # Verify file exists
   ```
   - If not found: Report error and STOP
   - If found: Continue

b) **Check if already prefixed:**
   - Pattern: `^[A-Z]{3,4}\d{3}_.*\.md$`
   - If already prefixed: Report "Document already has prefix" and STOP
   - If not prefixed: Continue

c) **Check if exempted:**
   - Check against exemption list from `workflows/REG003_category_definitions.md`:
     - `README.md`, `CLAUDE.md`, `STRUCTURE.md` - Standard files
     - `.claude/commands/*.md` - Workflow definitions
     - `change_history.md` - Project management
   - If exempted: Report "Document is exempted from prefixing" and STOP
   - If not exempted: Continue

## Document Size Checking and Structure Recommendation

**Before assigning document number, check size and recommend structure:**

### Size Check Procedure

1. **Count lines in document:**
   ```bash
   LINE_COUNT=$(wc -l < "$DOCUMENT_PATH")
   ```

2. **Recommend structure based on size:**
   - **<300 lines:** Single file acceptable, no special requirements
   - **300-1200 lines:** Recommend adding executive summary section at beginning
   - **>1200 lines:** **STRONGLY recommend** modular folder structure (per GOV001)

3. **Display recommendation:**
   ```
   Document size: [XXX] lines

   RECOMMENDATION:
   [If <300]: Single file structure is fine.
   [If 300-1200]: Consider adding executive summary (<300 lines) at beginning.
                   See templates/modular_document/ for guidance.
   [If >1200]: STRONG RECOMMENDATION: Use modular folder structure.
                This document should be split into:
                - 00_SUMMARY.md (<500 lines)
                - Section files (<300 lines each, estimated [X] sections needed)
                See templates/modular_document/README.md for complete guide.

                Continuing with single file is allowed but discouraged.
                Proceed? [Y/N]
   ```

4. **If user chooses modular for >1200 line document:**
   - Assign number to folder (e.g., SPEC008_library_management/)
   - Do NOT assign number to individual file
   - Register folder in REG001_number_registry.md

5. **If user chooses single file despite >1200 recommendation:**
   - Proceed with normal numbering
   - Add note in registry: "Large file ([XXX] lines), modular structure recommended"

### GOV001 Alignment Verification

**This workflow implements GOV001 "Document Size and Structure Standards" (once Phase 2 approved):**
- 300-line threshold: Executive summary recommended
- 1200-line threshold: Modular structure mandated
- Templates provided: templates/modular_document/

**Note:** Until GOV001 Phase 2 approval complete, modular structure is STRONGLY RECOMMENDED but not enforced.

### 2. **Determine Category**

a) **Analyze document location:**
   - `docs/*.md` → likely GOV, REQ, SPEC, IMPL, EXEC, REV, GUIDE, RPT, or PLAN
   - `workflows/*.md` → likely DWI, TMPL, or REG
   - `project_management/*.md` → likely LOG
   - `wip/*.md` → defer until moved to permanent location

   **Note:** Current folder structure is flat within each major directory

b) **Analyze document name for hints:**
   - Contains "governance", "charter" → GOV
   - Contains "requirements", "requirement" → REQ
   - Contains "specification", "spec", "architecture", "design" → SPEC
   - Contains "implementation", "schema", "coding" → IMPL
   - Contains "execution", "order", "timeline" → EXEC
   - Contains "review", "revision" → REV
   - Contains "guide", "manual", "howto" → GUIDE
   - Contains "report", "analysis", "investigation" → RPT
   - Contains "plan", "implementation_plan" → PLAN
   - Contains "workflow", "procedure", "instruction" → DWI
   - Contains "template" → TMPL
   - Contains "log", "feedback" → LOG
   - Contains "registry", "index" → REG

c) **Read document first 50 lines:**
   - Look for clues in title, headers, purpose statements
   - Look for requirement IDs (REQ-*, ARCH-*, etc.)
   - Governance document → GOV
   - Requirements document → REQ
   - Architecture/design/spec → SPEC
   - Implementation details → IMPL
   - Execution plan → EXEC
   - Review/revision → REV
   - User guide → GUIDE
   - Analysis/report → RPT
   - Implementation plan → PLAN
   - Workflow specification → DWI
   - Template document → TMPL
   - Operational log → LOG
   - Registry/index → REG

   **Key distinction:** Product vs. Workflow vs. Operational
   - Product docs (WHAT we're building) → GOV, REQ, SPEC, IMPL, EXEC, REV, GUIDE, RPT, PLAN
   - Workflow docs (HOW we build) → DWI, TMPL, REG
   - Operational docs (tracking/logs) → LOG

d) **Present recommendation to user:**
   ```
   Recommended category: RPT (Reports & Analysis)

   Reasoning:
   - Document location: docs/ (product documentation area)
   - Document name: investigation_notes.md (contains "investigation")
   - Content analysis: Contains analysis and research findings
   - Domain: Product (not workflow/process documentation)

   Available categories:

   Product Documentation (docs/):
   - GOV: Governance & Policy
   - REQ: Requirements
   - SPEC: Specifications & Design
   - IMPL: Implementation Details
   - EXEC: Execution Plans
   - REV: Reviews & Revisions
   - GUIDE: User & Developer Guides
   - RPT: Reports & Analysis
   - PLAN: Implementation Plans

   Workflow Documentation (workflows/):
   - DWI: Developer Work Instructions
   - TMPL: Templates
   - REG: Registries & Indexes

   Operational Documentation (project_management/):
   - LOG: Operational Logs

   Accept recommendation? [Y/n/category]
   ```
   - Accept: "y", "Y", "" (empty) → Use recommended category
   - Reject: "n", "N" → Ask user to specify category
   - Override: User types category code (e.g., "SPEC") → Use specified category

### 3. **Get Next Available Number**

a) **Read registry:**
   - Open `workflows/REG001_number_registry.md`
   - Parse "Next Available Numbers" table
   - Extract "Next Available" for chosen category
   - Example: RPT → 011

b) **Filesystem conflict check:**
   ```bash
   # Check if CAT###_*.md already exists
   find docs workflows project_management -name "RPT011_*.md"
   ```
   - If found: Increment to next number (012), check again
   - Repeat until available number found
   - Store final number for use

### 4. **Generate New Filename**

a) **Extract original name:**
   - Input: `docs/investigation_notes.md`
   - Extract: `investigation_notes.md`

b) **Generate prefixed name:**
   - Format: `CAT###_original_name.md`
   - Example: `RPT011_investigation_notes.md`

c) **Construct full new path:**
   - Keep same directory
   - Replace filename only
   - Result: `docs/RPT011_investigation_notes.md`

### 5. **Confirm with User**

**Present rename operation:**
```
Rename Operation:
  From: docs/investigation_notes.md
  To:   docs/RPT011_investigation_notes.md

  Category: RPT (Reports & Analysis)
  Number: 011

Proceed with rename? [Y/n]
```

- Accept: "y", "Y", "" → Proceed to rename
- Reject: "n", "N" → STOP workflow

### 6. **Execute Rename**

a) **Use git mv to preserve history:**
   ```bash
   git mv docs/investigation_notes.md docs/RPT011_investigation_notes.md
   ```

b) **Verify rename succeeded:**
   - Check new file exists
   - Check old file gone
   - If verification fails: Report error and provide recovery instructions

### 7. **Update Registry**

a) **Update "Next Available Numbers" table:**
   - Read current value (e.g., RPT: 011)
   - Increment by 1 (becomes 012)
   - Update "Last Used" to assigned number (011)
   - Increment "Documents in Category" count
   - Update "Note" with current date

b) **Add to "Assignment History" table:**
   - Add new row at top of history (newest first):
     ```
     | RPT011 | investigation_notes.md | 2025-10-25 | RPT | Auto | /doc-name workflow |
     ```

c) **Save registry file**

### 8. **Stage Changes**

```bash
# Stage renamed file
git add docs/RPT011_investigation_notes.md

# Stage registry
git add workflows/REG001_number_registry.md
```

### 9. **Report Success**

```
Document renamed successfully!

From: investigation_notes.md
To:   RPT011_investigation_notes.md

Category: RPT (Reports & Analysis)
Number: 011

Registry updated:
- Next Available: RPT 012
- History entry added

Files staged for commit:
- docs/RPT011_investigation_notes.md (renamed)
- workflows/REG001_number_registry.md (updated)

Next steps:
1. Update any references to old filename
2. Commit changes using git commit
```

---

## Error Handling

### File Not Found
```
Error: Document not found

Path: docs/nonexistent.md

Verify:
- File path is correct
- File exists
- Path is relative to workspace root (c:\Users\Mango Cat\Dev\McRhythm)
```

### Already Prefixed
```
Document already has prefix

Current name: RPT011_investigation_notes.md

No action needed. Document already follows naming convention.
```

### Exempted Document
```
Document is exempted from prefixing

File: README.md
Reason: Standard project file

Exempted types:
- README.md, CLAUDE.md, STRUCTURE.md
- .claude/commands/*.md
- change_history.md
- Files starting with underscore in wip/ (temporary state)
```

### Registry Conflict
```
Number conflict detected

Attempted: RPT011
Conflict: docs/RPT011_existing_document.md already exists

Resolution: Auto-incremented to RPT012
```

---

## Category Override

User can override recommended category:

**Example:**
```
Recommended category: SPEC (Specifications & Design)
Accept recommendation? [Y/n/category] IMPL

Category overridden to IMPL (Implementation Details)

Next Available: IMPL003
Proceeding with IMPL003_component_design.md
```

---

## Batch Operation Support (Future)

**Not implemented in this version.**

For batch operations, run /doc-name once per file or use manual procedures from workflow documentation.

---

## Integration with Git

After using /doc-name:

```bash
# Stage and commit
git commit -m "docs: assign prefix RPT011 to investigation_notes.md"
```

The git commit will include:
1. Renamed file (via git mv)
2. Registry update

---

## Quick Reference

### Command Usage
```bash
/doc-name docs/my_document.md
/doc-name workflows/my_workflow.md
```

### Check Current State
```bash
# View registry
cat workflows/REG001_number_registry.md

# List prefixed documents
find docs workflows project_management -name "*[0-9][0-9][0-9]_*.md"
```

---

## Validation Checklist

Before completing workflow, verify:

- [ ] Document size checked and structure recommendation provided
- [ ] File renamed successfully (git mv)
- [ ] Old filename no longer exists
- [ ] New filename follows CAT###_name.md pattern
- [ ] Registry "Next Available" incremented
- [ ] Registry history entry added
- [ ] Both files staged for commit
- [ ] No filesystem conflicts

### Size Awareness

**Always check document size before numbering:**
- Helps authors plan structure early
- Prevents monolithic documents from being created
- Supports context window optimization project-wide
- Aligns with PLAN003 context engineering improvements

---

## Limitations

**Current Version:**
- Interactive only (no batch mode)
- Manual category override via prompt (no --category flag)
- No automatic reference updating (user must update references manually)
- No rollback mechanism (use git restore if needed)

**Future Enhancements:**
- Batch rename mode
- Automatic reference scanning and updating
- Integration with planning workflows for new documents
- Rollback/undo command

---

## See Also

- **Category Guidelines:** `workflows/REG003_category_definitions.md`
- **Number Registry:** `workflows/REG001_number_registry.md`
- **Document Hierarchy:** `docs/GOV001-document_hierarchy.md`
- **Project Instructions:** `CLAUDE.md`

---

**Maintained by:** Development team
**Created:** 2025-10-25
**Status:** Active workflow automation

