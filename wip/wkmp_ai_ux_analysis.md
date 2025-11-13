# WKMP-AI User Experience Analysis

**Document Type:** Analysis Report
**Analysis Date:** 2025-01-08
**Analysis Method:** Multi-agent research and synthesis (/think workflow)
**Analyst:** Claude Code
**Scope:** Import UX for both bulk (initial library) and incremental (new albums) scenarios

---

## Executive Summary

### Current State
wkmp-ai implements a well-structured 7-phase import workflow with:
- Strong progress visibility (SSE-based real-time updates)
- Multi-level progress tracking (overall + per-phase + sub-task)
- Cancellable background processing
- Session persistence for crash recovery

### Critical Findings

1. **No differentiation between bulk and incremental imports** - Same heavyweight workflow runs for 2 files or 2,000 files
2. **Attention-demanding single-session model** - User must wait for entire import to complete; cannot add new music during active import
3. **Missing automatic monitoring** - No folder watching; every import requires manual initiation
4. **No smart defaults** - User enters raw folder path; no quick re-import of known locations
5. **Phase visibility may overwhelm casual users** - 7-phase technical breakdown valuable for power users, potentially confusing for first-time users

### Recommendation

**Adopt dual-mode import strategy:**
- **Quick Import Mode** (default) - Optimized for 1-50 files, <5 clicks, ~2 minutes
- **Deep Library Scan Mode** - Full 7-phase analysis for initial bulk import

**Plus optional enhancements:**
- Watched folders with background monitoring
- Import profiles for common locations
- Progressive disclosure UI (simple → detailed)

---

## 1. Current State Assessment

### 1.1 Existing Workflow Strengths

**✓ Excellent Progress Communication**
- Real-time SSE updates (throttled to 10/sec, prevents UI flooding)
- Multi-level progress: overall → phase → sub-task → current file
- Time estimates (elapsed + remaining)
- Error accumulation with file paths

**✓ Robust Architecture**
- Session persistence (survives crashes)
- Cancellable via token mechanism
- Parallel processing (4 workers default)
- Graceful degradation (works without AcoustID key)

**✓ Comprehensive Analysis**
- 7 phases ensure high-quality metadata:
  1. Scanning - File discovery
  2. Extracting - ID3/metadata reading
  3. Fingerprinting - AcoustID → MusicBrainz identification
  4. Segmenting - Passage boundary detection
  5. Analyzing - Crossfade timing calculation
  6. Flavoring - Musical characteristics extraction
  7. Completion - Database commit

**✓ Professional API Design**
- 202 Accepted pattern (async processing)
- RESTful endpoints
- Session-based tracking

### 1.2 Pain Points Identified

#### **Pain Point 1: Bulk vs Incremental Treatment**

**Current Behavior:**
- Adding 2 new albums (20 files) triggers same 7-phase workflow as initial import of 5,000 files
- No "quick add" path for small increments
- Full fingerprinting + MusicBrainz lookup even when metadata is complete

**User Impact:**
- **Bulk scenario:** Appropriate thoroughness, user expects long process
- **Incremental scenario:** Frustration - "Why does adding one album take 10 minutes?"

**Research Context:**
- **MusicBrainz Picard:** Offers "Lookup" (fast, metadata-based) vs "Scan" (slow, fingerprint-based) modes
- **Roon:** Scans incrementally, shows "Identifying..." badge that completes in background
- **Plex:** Criticized for inefficient full-library rescans when adding small amounts of content

#### **Pain Point 2: Single-Session Constraint**

**Current Behavior:**
- Only one import session can run at a time (409 Conflict check)
- User cannot add new music while initial import is running
- User cannot queue multiple folder imports

**User Impact:**
- **Bulk scenario:** User starts 5,000-file import, realizes they forgot another folder → Must wait 2 hours to add it
- **Incremental scenario:** User downloads new album, wants to add it immediately → Blocked if any import is running

**Research Context:**
- **Best practice:** Segmented imports - "import 100 users, then another 100" (Oracle bulk loading guide)
- **File upload UIs:** Allow queueing multiple batches, process sequentially or with limited parallelism

#### **Pain Point 3: Manual-Only Initiation**

**Current Behavior:**
- Every import requires user to:
  1. Open http://localhost:5723
  2. Navigate to import-progress page
  3. Type/paste folder path
  4. Click "Start Import"
  5. Monitor progress page

**User Impact:**
- **Bulk scenario:** Reasonable for one-time initial setup
- **Incremental scenario:** "I just added new music to my usual folder, why do I need 5 clicks to import it?"

**Research Context:**
- **Watch folders:** Industry standard for media servers (Plex, Jellyfin concept, though implementations criticized)
- **Beets:** Command-line import with `beet import /path/to/music` - single command
- **MusicBee/foobar2000:** Third-party tools add folder watching (not built-in, indicating demand)

#### **Pain Point 4: No Location Memory**

**Current Behavior:**
- Text input field for folder path, no autocomplete or history
- User must remember/locate folder path every time
- No "import from last location" button

**User Impact:**
- **Bulk scenario:** Minor inconvenience (one-time use)
- **Incremental scenario:** "Why am I typing `/media/music/incoming` for the 20th time?"

**Research Context:**
- **Standard UX pattern:** File pickers remember recent locations
- **Bulk upload best practices:** "Simplify instructions to 100 words or less" - includes reducing repeated inputs

#### **Pain Point 5: Technical Complexity Exposure**

**Current Behavior:**
- UI shows all 7 phases upfront with sub-tasks (Chromaprint, AcoustID, MusicBrainz)
- Progress displayed as: "Fingerprinting: 45/120 files, Chromaprint: 98% success rate"

**User Impact:**
- **Power users:** Love the detail, helps debug issues
- **Casual users:** "What's a Chromaprint? Why do I need to know this?"
- **First-time users:** Overwhelmed by technical terminology

**Research Context:**
- **Progressive disclosure:** Show simple summary by default, expand details on click
- **MusicBrainz Picard:** Default view shows file tree + match confidence; technical details in separate pane
- **Roon:** Shows simple "Identifying..." badge; detailed analysis hidden in settings

---

## 2. Research Findings: Industry Patterns

### 2.1 Dual-Mode Import Strategies

**MusicBrainz Picard: Lookup vs Scan**
- **Lookup mode:** Uses existing metadata, queries MusicBrainz API, fast (<1 sec/album)
- **Scan mode:** Generates fingerprints, slow but accurate (~5-10 sec/file)
- **User choice:** Defaults to Lookup, recommends Scan for untagged/poorly-tagged files
- **UX lesson:** Users appreciate speed option when metadata is trustworthy

**Roon: Background Identification**
- **Initial scan:** Fast file discovery + basic metadata extraction
- **Analysis phase:** Runs in background, shows "Identifying..." badge
- **Playable immediately:** User can play music before full analysis completes
- **UX lesson:** Don't block user access to content while enriching metadata

**Beets: Incremental by Default**
- **Command:** `beet import <folder>` - processes only new files
- **Detection:** Tracks imported files, skips duplicates automatically
- **Confirmation:** Asks user to confirm matches (interactive mode)
- **UX lesson:** System should remember what's already imported

### 2.2 Folder Monitoring Approaches

**Plex Watch Folders**
- **Configuration:** User designates library folders in settings
- **Monitoring:** Periodic scanning (configurable interval, e.g., every 15 minutes)
- **Criticism:** Full-library rescans are resource-intensive, slow
- **UX lesson:** Folder watching is desirable but must be efficient

**Beets + Third-Party Watchers**
- **Integration:** User sets up external folder watcher (e.g., inotify) to trigger `beet import`
- **Manual setup:** Requires technical knowledge
- **Demand signal:** Community created tools because built-in feature missing
- **UX lesson:** Users want automatic import, will hack it if necessary

**MusicBee/foobar2000**
- **Third-party plugins:** foo_tunes adds folder watching to foobar2000
- **Not built-in:** Core apps don't include this feature
- **UX lesson:** Automatic monitoring is valuable but complex to implement well

### 2.3 Progress Feedback Patterns

**CLI UX Best Practices (Evil Martians, 2024)**

Three essential patterns:
1. **Spinner:** For indefinite tasks ("Connecting to server...")
2. **X of Y:** For countable items ("Processing file 45 of 120")
3. **Progress bar:** For percentage-based tasks ("45% complete")

**Best practices:**
- Show elapsed time after 3 seconds
- Show ETA after sufficient data (5-10% progress)
- Update frequently enough (10-30 FPS for smooth perception) but not excessively (avoid CPU waste)
- Provide cancellation option for long tasks

**wkmp-ai current implementation:** ✓ Follows all best practices (X of Y, progress bar, elapsed time, ETA, cancellation)

### 2.4 Bulk Import UX Patterns (Smashing Magazine, 2020)

**Data Importer Design Principles:**

1. **Clear instructions** (≤100 words)
   - File formats allowed
   - File size limitations
   - Data format requirements

2. **Upfront validation**
   - Check file type/size before upload
   - Validate header columns/structure
   - Show errors before proceeding

3. **Column mapping** (for structured data)
   - Auto-detect common patterns
   - Allow user correction
   - Preview results before import

4. **Segmented processing**
   - "Import 1 file to test, then 100, then full batch"
   - Reduces risk of bulk failures

5. **Results summary**
   - "120 files imported, 5 skipped (duplicates), 2 failed (see log)"
   - Actionable error messages

**wkmp-ai application:**
- ✓ Clear phase descriptions
- ✓ Results summary (completion notification)
- ✗ No segmented import option
- ✗ No validation before scan starts
- ✗ No preview/confirmation step

---

## 3. Pain Point Analysis: Bulk vs Incremental

### 3.1 Bulk Import (Initial Library: 1,000-10,000 files)

**User Goals:**
- Import entire music collection once
- High-quality metadata and analysis
- Willing to wait hours if necessary
- Wants visibility into progress

**Current wkmp-ai Fit:**
- ✓ Comprehensive 7-phase analysis appropriate
- ✓ Excellent progress visibility
- ✓ Parallel processing (4 workers)
- ✓ Cancellable if needed
- ✓ Session persistence (crash recovery)

**Minor Improvements Possible:**
- Pre-scan validation (folder exists, contains audio files, check disk space)
- Estimated total time after scanning phase (based on file count + historical rates)
- Pause/resume capability (in addition to cancel)

**Conclusion:** Current implementation is well-suited for bulk import scenario.

### 3.2 Incremental Import (New Albums: 1-50 files)

**User Goals:**
- Add new music quickly (<5 minutes total)
- Minimal clicks/attention required
- Confidence that music is ready to play
- Don't care about deep analysis if metadata is good

**Current wkmp-ai Fit:**
- ✗ 7-phase workflow overkill for 10 files with complete metadata
- ✗ Manual initiation (5 clicks/inputs) feels tedious for frequent task
- ✗ Blocked if any import running
- ✗ No memory of common import locations

**Specific User Frustrations:**

| User Action | Current UX | User Expectation |
|-------------|------------|------------------|
| Download new album with complete tags | 7-phase import, 5-10 minutes | "Click 'Quick Add', done in 30 seconds" |
| Add 3 albums from usual folder | Type path 3 times or run 1 import with all 3 | "System remembers my music folder" |
| Import 1 album while initial scan running | 409 Conflict, must wait | "Queue it, process when ready" |
| Check if import needed | Must manually initiate | "System watches folder, auto-imports" |

### 3.3 Comparison Matrix

| Aspect | Bulk Import (1st time) | Incremental Import (ongoing) |
|--------|------------------------|------------------------------|
| **Frequency** | Once | Weekly/monthly |
| **File count** | 1,000-10,000 | 1-50 |
| **User patience** | High (expecting hours) | Low (expecting minutes) |
| **Metadata quality** | Variable (old rips, various sources) | Usually good (new releases) |
| **Analysis depth needed** | Full (optimize library) | Basic (just make playable) |
| **Attention available** | Willing to monitor | Wants fire-and-forget |
| **Current fit** | ✓ Excellent | ✗ Poor |

---

## 4. Solution Approaches

### Approach A: Status Quo (No Changes)

**Description:**
Maintain current single-workflow design.

**Risk Assessment:**
- **Failure Risk:** Low (system works correctly)
- **Failure Modes:**
  1. User frustration with incremental imports → Low probability (users adapt), Medium impact (poor UX)
  2. Competitive disadvantage vs other players → Medium probability (Roon/Plex offer better incremental UX), Medium impact
- **Mitigation:** Documentation explaining why thoroughness is valuable
- **Residual Risk:** Low-Medium (technical success, UX suboptimal)

**Quality Characteristics:**
- Maintainability: High (no changes)
- Test Coverage: High (existing)
- Architectural Alignment: Strong (no changes)

**Implementation Considerations:**
- Effort: Zero
- Dependencies: None
- Complexity: Low

**Pros:**
- No development effort
- No new bugs
- Consistent user experience

**Cons:**
- Incremental import UX remains frustrating
- Users may resort to workarounds (e.g., manual database edits)
- Missed opportunity to match industry standards

---

### Approach B: Quick Import Mode (Dual-Mode Strategy)

**Description:**
Add "Quick Import" option that skips expensive phases when metadata is sufficient.

**Quick Import Workflow:**
1. Scanning - File discovery ✓
2. Extracting - Read ID3/metadata ✓
3. **Skip Fingerprinting** - Only if metadata includes MusicBrainz IDs or high-confidence artist/album/title
4. **Skip Segmenting** - Create single passage spanning entire file (default crossfade timing)
5. **Skip Analyzing** - Use default lead-in/lead-out values (e.g., 2 seconds)
6. **Skip Flavoring** - Mark for background analysis or use genre-based defaults
7. Completion - Write to database ✓

**Quick Import Criteria (auto-detected):**
- Files have ID3v2.3+ tags with artist/album/title
- OR Files have MusicBrainz Recording ID tag
- File count <100
- User selects "Quick Import" (or system suggests based on criteria)

**Fallback:**
- If Quick Import encounters issues (missing metadata, duplicates), offer to switch to Deep Scan

**UI Changes:**
- Import initiation page: Two buttons - "Quick Import" (default) vs "Deep Library Scan"
- Quick Import progress: Simplified 3-phase view (Scanning → Importing → Complete)
- Results summary: "120 files added in 2 minutes. Deep analysis scheduled for background."

**Risk Assessment:**
- **Failure Risk:** Medium (until proven)
- **Failure Modes:**
  1. Quick Import skips necessary analysis → Probability: Low (metadata validation), Impact: Medium (suboptimal crossfades)
  2. Users always choose Quick Import, defeat purpose → Probability: Medium, Impact: Medium (library quality degrades)
  3. Implementation bugs in mode selection → Probability: Medium, Impact: Low (fallback to Deep Scan)
- **Mitigation:**
  1. Validate metadata quality before Quick Import
  2. System recommends appropriate mode based on file analysis
  3. Allow post-import "deep analyze" for selected passages
  4. Comprehensive testing of both workflows
- **Residual Risk:** Low-Medium

**Quality Characteristics:**
- Maintainability: Medium (two code paths to maintain)
- Test Coverage: Medium (must test both modes + fallback logic)
- Architectural Alignment: Strong (extends existing workflow)

**Implementation Considerations:**
- Effort: Medium (2-4 days - add mode selection, implement shortcuts, test)
- Dependencies: Existing workflow phases (refactor to allow skipping)
- Complexity: Medium (conditional phase execution)

**Pros:**
- Dramatically faster incremental imports (2 min vs 10 min for 20 files)
- Maintains full analysis option for bulk imports
- User choice respects different use cases
- Background analysis can upgrade Quick Imports later

**Cons:**
- Two code paths increase complexity
- Risk of users choosing wrong mode
- Passages created via Quick Import have inferior crossfade timing initially
- Requires user decision (cognitive load)

---

### Approach C: Watched Folders (Automatic Monitoring)

**Description:**
Allow user to designate "watched folders" that are automatically scanned for new files.

**Configuration:**
- Settings page: Add/remove watched folders
- Per-folder settings:
  - Scan interval (e.g., every 15 minutes, hourly, on-demand)
  - Import mode (Quick vs Deep)
  - Notifications (desktop/email when import completes)

**Background Monitoring:**
- Periodic scan for new files (compare against database)
- Auto-import new files using configured mode
- Show notification: "5 new files imported from /home/user/Music/Incoming"

**UI Changes:**
- Dashboard widget: "Watched Folders - Last scan: 2 minutes ago, 0 new files"
- Manual scan button: "Scan Now" (don't wait for interval)
- Import history: List of auto-imports with timestamps

**Risk Assessment:**
- **Failure Risk:** Medium-High
- **Failure Modes:**
  1. Resource consumption (constant scanning) → Probability: High, Impact: Medium (CPU/disk usage)
  2. Import failures go unnoticed → Probability: Medium, Impact: High (silent data loss)
  3. Race conditions (file still being written when scanned) → Probability: Medium, Impact: Medium (corrupted imports)
  4. Platform-specific file watching (inotify on Linux, FSEvents on Mac) → Probability: High, Impact: Medium (complexity)
- **Mitigation:**
  1. Configurable scan intervals, skip scan if system busy
  2. Persistent notifications of failures, email alerts
  3. File stability check (wait for file size to stabilize)
  4. Use cross-platform library (notify-rs) with fallback to polling
- **Residual Risk:** Medium (even with mitigations, edge cases remain)

**Quality Characteristics:**
- Maintainability: Low (complex state management, platform-specific issues)
- Test Coverage: Medium (difficult to test timing-dependent behavior)
- Architectural Alignment: Weak (adds always-on background service, violates current on-demand design)

**Implementation Considerations:**
- Effort: High (5-10 days - file watching, state management, configuration UI, testing)
- Dependencies: notify-rs crate or equivalent, background task management
- Complexity: High (concurrency, platform differences, error handling)

**Pros:**
- Zero-click import for new files
- Industry-standard feature (expected by users from Plex/Jellyfin)
- Enables true "fire and forget" workflow

**Cons:**
- High implementation complexity
- Resource consumption (always-on monitoring)
- Potential for silent failures if notifications missed
- Platform-specific issues (Linux inotify limits, macOS FSEvents quirks)
- Conflicts with wkmp-ai's "on-demand" architectural pattern

---

### Approach D: Import Profiles (Location Memory)

**Description:**
Save frequently-used import configurations as "profiles" for one-click reuse.

**Profile Structure:**
```rust
struct ImportProfile {
    name: String,              // "Incoming Music", "Ripped CDs"
    folder_path: PathBuf,      // /home/user/Music/Incoming
    mode: ImportMode,          // Quick vs Deep
    parameters: ImportParameters, // Parallelism, file extensions, etc.
}
```

**UI Changes:**
- **Settings page:** Manage profiles (add/edit/delete)
- **Import page:** Dropdown of profiles + "Import with [Profile Name]" button
- **Quick access:** "Recent Imports" - last 5 import paths for one-click re-import

**Risk Assessment:**
- **Failure Risk:** Low
- **Failure Modes:**
  1. Profile references deleted folder → Probability: Low, Impact: Low (validation catches)
  2. User forgets which profile for which purpose → Probability: Low, Impact: Low (rename profile)
- **Mitigation:**
  1. Validate folder exists before import
  2. Require descriptive profile names
- **Residual Risk:** Low

**Quality Characteristics:**
- Maintainability: High (simple CRUD on database table)
- Test Coverage: High (straightforward state management)
- Architectural Alignment: Strong (extends existing workflow)

**Implementation Considerations:**
- Effort: Low (1-2 days - database table, CRUD API, UI components)
- Dependencies: None (uses existing import workflow)
- Complexity: Low (basic CRUD)

**Pros:**
- Reduces clicks for repeat imports (5 clicks → 2 clicks)
- No background monitoring complexity
- User retains control (explicit import trigger)
- Simple to implement and maintain

**Cons:**
- Still requires manual import initiation (not automatic)
- Doesn't address bulk vs incremental workflow difference
- Solves "location memory" pain point only

---

### Approach E: Import Queue (Concurrent Sessions)

**Description:**
Remove single-session constraint; allow users to queue multiple imports.

**Queue Behavior:**
- User can submit multiple import requests
- System processes sequentially (to avoid resource contention)
- OR processes with limited parallelism (e.g., 2 concurrent imports)

**UI Changes:**
- **Import Queue page:** List of pending/active/completed imports
- **Per-session card:** Progress, cancel button, view results
- **Submission:** "Add to Queue" button (instead of blocking 409 error)

**Risk Assessment:**
- **Failure Risk:** Low-Medium
- **Failure Modes:**
  1. Resource contention (2 imports thrashing disk) → Probability: Medium, Impact: Medium (slow performance)
  2. Database lock contention → Probability: Low, Impact: Low (SQLite handles well with WAL)
  3. Complex queue state management → Probability: Medium, Impact: Low (testing catches issues)
- **Mitigation:**
  1. Sequential processing by default, parallel opt-in
  2. Use WAL mode in SQLite (already enabled)
  3. Robust queue implementation with persistence
- **Residual Risk:** Low

**Quality Characteristics:**
- Maintainability: Medium (queue state management)
- Test Coverage: Medium (test queue ordering, cancellation, failures)
- Architectural Alignment: Moderate (extends session model)

**Implementation Considerations:**
- Effort: Medium (3-5 days - queue implementation, UI, state management, testing)
- Dependencies: None (extends existing session tracking)
- Complexity: Medium (queue management, concurrency control)

**Pros:**
- Eliminates blocking 409 error
- User can queue multiple imports, walk away
- Fits bulk scenario (queue 5 folders) and incremental (add one album while bulk runs)

**Cons:**
- Doesn't reduce clicks or time per import
- Resource management complexity (must avoid disk thrashing)
- UI complexity (show multiple sessions in progress)

---

### Approach F: Progressive Disclosure UI (Simplified/Advanced Views)

**Description:**
Redesign UI to show simple view by default, expand to technical details on request.

**Simple View (Default):**
- Single progress bar: "Importing your music... 45% complete (54 of 120 files)"
- Elapsed time: "5 minutes elapsed"
- Current file: "Processing: 02 - Bohemian Rhapsody.mp3"
- "Show Details" button (collapsed by default)

**Advanced View (On Click):**
- Expand to show all 7 phases with sub-task breakdowns
- Technical metrics (Chromaprint success rate, MusicBrainz match confidence)
- Error log with full file paths and stack traces

**Risk Assessment:**
- **Failure Risk:** Low
- **Failure Modes:**
  1. Users miss important error details → Probability: Low, Impact: Medium (could surface critical errors in simple view)
  2. UI redesign introduces bugs → Probability: Low, Impact: Low (visual changes)
- **Mitigation:**
  1. Show error count in simple view with "View Errors" button
  2. Thorough UI testing
- **Residual Risk:** Low

**Quality Characteristics:**
- Maintainability: High (CSS/JS changes, no backend logic)
- Test Coverage: High (UI testing)
- Architectural Alignment: Strong (no backend changes)

**Implementation Considerations:**
- Effort: Low (1-2 days - HTML/CSS/JS refactoring, testing)
- Dependencies: None
- Complexity: Low (frontend only)

**Pros:**
- Reduces cognitive load for casual users
- Maintains power user capabilities
- Common UX pattern (progressive disclosure)
- No backend changes

**Cons:**
- Doesn't address bulk vs incremental workflow difference
- Doesn't reduce time or clicks
- Purely cosmetic improvement

---

## 5. Recommendations

### 5.1 Risk-Based Ranking

Based on Risk-First Decision Framework:

| Approach | Residual Risk | Quality | Effort | Rank |
|----------|---------------|---------|--------|------|
| **A: Status Quo** | Low-Medium | High | Zero | 4th |
| **B: Quick Import** | Low-Medium | Medium | Medium | **1st** ⭐ |
| **C: Watched Folders** | Medium | Low | High | 6th |
| **D: Import Profiles** | Low | High | Low | **2nd** ⭐ |
| **E: Import Queue** | Low | Medium | Medium | 3rd |
| **F: Progressive UI** | Low | High | Low | **2nd** ⭐ (tie) |

### 5.2 Recommended Implementation Plan

**Phase 1: Quick Wins (1-2 weeks)**

Implement **Approach D (Import Profiles)** + **Approach F (Progressive UI)**:

**Rationale:**
- Both have Low residual risk
- Both have Low effort (3-4 days total)
- Combined, they address multiple pain points:
  - Location memory (profiles)
  - Cognitive load (progressive UI)
  - Repeat import friction (profiles reduce clicks)

**User Impact:**
- Incremental import: 5 clicks → 2 clicks (60% reduction)
- First-time user: Less overwhelmed by technical details
- Power user: Still has full visibility when needed

---

**Phase 2: Workflow Optimization (2-4 weeks)**

Implement **Approach B (Quick Import Mode)**:

**Rationale:**
- Highest impact on incremental import time (10 min → 2 min)
- Low-Medium residual risk (acceptable with mitigations)
- Addresses core pain point: workflow overkill for small imports

**User Impact:**
- Incremental import: 10 minutes → 2 minutes (80% time reduction)
- Bulk import: Unchanged (users choose Deep Scan for initial library)
- Quality: Background analysis can upgrade Quick Imports later

**Design Decisions:**
- **Auto-detection recommended:** System suggests mode based on:
  - File count (<50 files → suggest Quick Import)
  - Metadata quality (has MusicBrainz IDs → suggest Quick Import)
  - User can override suggestion
- **Fallback strategy:** If Quick Import encounters missing metadata, offer "Switch to Deep Scan" or "Skip this file"
- **Background upgrade:** Option to "Analyze in background" for passages created via Quick Import

---

**Phase 3: Advanced Features (Future/Optional)**

Consider **Approach E (Import Queue)** if user demand exists:

**Rationale:**
- Medium effort, Low residual risk
- Addresses specific user frustration (409 blocking error)
- Not critical for MVP (users can wait for current import to complete)

**Defer/Avoid:**
- **Approach C (Watched Folders):** High complexity, Medium residual risk, architectural misalignment
- **Recommendation:** Provide good manual import UX (Profiles + Quick Import) rather than risky automatic monitoring

---

### 5.3 Combined User Experience (After Phase 1 + 2)

#### Bulk Import Scenario (Initial 5,000-file library)

**User Actions:**
1. Navigate to http://localhost:5723
2. Click "Import"
3. System detects 5,000 files → Recommends "Deep Library Scan" (selected by default)
4. Click "Start Import"

**System Behavior:**
- Full 7-phase analysis
- Simple progress view by default: "Importing... 23% complete (1,150 of 5,000 files)"
- User can expand to see phase details if curious
- 2-3 hours later: "Import complete! 5,000 files ready to play."

**Time:** ~2-3 hours (unchanged, appropriate for bulk)
**Clicks:** 3 (same as current)
**Attention:** Low (progress visible, can walk away)

---

#### Incremental Import Scenario (2 new albums, 20 files)

**User Actions:**
1. Navigate to http://localhost:5723
2. Select "Incoming Music" profile from dropdown (pre-configured: /home/user/Music/Incoming, Quick Import mode)
3. Click "Import with Profile"

**System Behavior:**
- Quick Import: Scanning → Importing (skips fingerprinting, segmenting, analyzing, flavoring)
- Simple progress: "Importing... 20 of 20 files"
- 2 minutes later: "Import complete! 20 files ready to play. Background analysis scheduled."

**Time:** ~2 minutes (vs 10 minutes currently - 80% reduction)
**Clicks:** 2 (vs 5 currently - 60% reduction)
**Attention:** Minimal (fast enough to watch progress bar)

---

#### Incremental Import Scenario (First Time, Unknown Location)

**User Actions:**
1. Navigate to http://localhost:5723
2. Click "Import"
3. Enter folder path
4. System detects 20 files with complete metadata → Recommends "Quick Import" (selected by default)
5. Click "Start Import"

**System Behavior:**
- Quick Import runs
- 2 minutes later: Complete
- System offers: "Save this as a profile for one-click import next time?"

**Time:** ~2 minutes
**Clicks:** 4 (first time), 2 (subsequent with saved profile)
**Attention:** Minimal

---

## 6. Implementation Guidance

### 6.1 Approach D: Import Profiles

**Database Schema:**
```sql
CREATE TABLE import_profiles (
    id TEXT PRIMARY KEY,          -- UUID
    name TEXT NOT NULL UNIQUE,    -- "Incoming Music"
    folder_path TEXT NOT NULL,    -- "/home/user/Music/Incoming"
    import_mode TEXT NOT NULL,    -- "quick" | "deep"
    parameters_json TEXT,         -- JSON: ImportParameters struct
    created_at INTEGER NOT NULL,  -- Unix timestamp
    last_used_at INTEGER          -- Unix timestamp
);

CREATE INDEX idx_profiles_last_used ON import_profiles(last_used_at DESC);
```

**API Endpoints:**
```
GET    /profiles               - List all profiles
POST   /profiles               - Create profile
GET    /profiles/{id}          - Get profile details
PUT    /profiles/{id}          - Update profile
DELETE /profiles/{id}          - Delete profile
POST   /profiles/{id}/import   - Start import with profile
```

**UI Components:**
- **Settings Page:** CRUD interface for profiles
- **Import Page:** Dropdown of profiles + "Import with [Profile]" button
- **Recent Imports Section:** Last 5 import paths for quick re-import (even if not saved as profile)

**Testing:**
- Unit tests: Profile CRUD operations
- Integration tests: Import with profile (validate folder path, parameters applied correctly)
- UI tests: Create profile, import, verify results

---

### 6.2 Approach F: Progressive Disclosure UI

**Simple View (Default):**
```html
<div class="import-progress-simple">
  <h2>Importing your music...</h2>
  <div class="progress-bar">
    <div class="progress-fill" style="width: 45%">45%</div>
  </div>
  <p class="progress-summary">54 of 120 files · 5 minutes elapsed</p>
  <p class="current-file">Processing: 02 - Bohemian Rhapsody.mp3</p>
  <div class="errors-summary" *ngIf="errorCount > 0">
    <span class="error-badge">⚠️ {{errorCount}} errors</span>
    <button (click)="showErrors()">View Errors</button>
  </div>
  <button class="expand-details" (click)="toggleAdvanced()">
    Show Details ▼
  </button>
</div>
```

**Advanced View (Expanded):**
```html
<div class="import-progress-advanced" *ngIf="showAdvanced">
  <h3>Import Phases</h3>
  <div class="phase-checklist">
    <!-- Existing 7-phase checklist UI -->
    <div class="phase" *ngFor="let phase of phases">
      <span class="phase-icon">{{phase.icon}}</span>
      <span class="phase-name">{{phase.name}}</span>
      <span class="phase-progress">{{phase.progress}}</span>
      <div class="sub-tasks">
        <!-- Sub-task details -->
      </div>
    </div>
  </div>
  <button (click)="toggleAdvanced()">Hide Details ▲</button>
</div>
```

**State Management:**
```javascript
// LocalStorage: Remember user preference
const showAdvanced = localStorage.getItem('import_show_advanced') === 'true';

function toggleAdvanced() {
  showAdvanced = !showAdvanced;
  localStorage.setItem('import_show_advanced', showAdvanced.toString());
  renderView();
}
```

**Testing:**
- UI tests: Toggle advanced view, verify content
- Accessibility tests: Keyboard navigation, screen reader compatibility
- Responsive tests: Mobile vs desktop layouts

---

### 6.3 Approach B: Quick Import Mode

**Workflow Changes:**

**File:** `src/services/workflow_orchestrator.rs`

```rust
pub enum ImportMode {
    Quick,      // Skip expensive phases when possible
    Deep,       // Full 7-phase analysis
}

pub async fn run_import_workflow(
    session_id: Uuid,
    folder_path: PathBuf,
    mode: ImportMode,
    parameters: ImportParameters,
    event_bus: Arc<EventBus>,
    cancel_token: CancellationToken,
) -> Result<()> {
    // Phase 1: Scanning (always)
    let files = scan_files(&folder_path, &parameters).await?;

    // Phase 2: Extracting (always)
    let metadata = extract_metadata(&files).await?;

    match mode {
        ImportMode::Quick => {
            // Skip fingerprinting if metadata sufficient
            if metadata_is_sufficient(&metadata) {
                quick_import(&metadata, event_bus).await?;
            } else {
                // Fallback to deep scan
                deep_import(&metadata, event_bus, cancel_token).await?;
            }
        },
        ImportMode::Deep => {
            deep_import(&metadata, event_bus, cancel_token).await?;
        },
    }
}

fn metadata_is_sufficient(metadata: &[FileMetadata]) -> bool {
    metadata.iter().all(|m| {
        m.musicbrainz_recording_id.is_some() ||
        (m.artist.is_some() && m.album.is_some() && m.title.is_some())
    })
}

async fn quick_import(metadata: &[FileMetadata], event_bus: Arc<EventBus>) -> Result<()> {
    // Create passages with default timing
    for file in metadata {
        let passage = Passage {
            id: Uuid::new_v4(),
            file_path: file.path.clone(),
            start_ms: 0,
            end_ms: file.duration_ms,
            crossfade_start_ms: 2000,  // Default 2 sec lead-in
            crossfade_end_ms: file.duration_ms - 2000,  // Default 2 sec lead-out
            // ... other fields from metadata
        };
        db::insert_passage(&passage).await?;
    }

    // Schedule background analysis (optional)
    schedule_background_analysis(metadata).await?;

    Ok(())
}
```

**UI Changes:**

**File:** `static/import-progress.html`

```html
<div class="import-mode-selection">
  <h2>Import Mode</h2>
  <div class="mode-recommendation" *ngIf="recommendedMode">
    ℹ️ Recommended: <strong>{{recommendedMode}}</strong>
    <p>{{recommendationReason}}</p>
  </div>

  <label>
    <input type="radio" name="mode" value="quick" [(ngModel)]="selectedMode">
    <div class="mode-option">
      <strong>Quick Import</strong>
      <p>Fast import using existing metadata. Best for new albums with complete tags.</p>
      <p class="mode-timing">⏱️ ~2 minutes for 20 files</p>
    </div>
  </label>

  <label>
    <input type="radio" name="mode" value="deep" [(ngModel)]="selectedMode">
    <div class="mode-option">
      <strong>Deep Library Scan</strong>
      <p>Full analysis with fingerprinting and passage detection. Best for initial bulk import.</p>
      <p class="mode-timing">⏱️ ~10 minutes for 20 files, ~2 hours for 2,000 files</p>
    </div>
  </label>
</div>
```

**Mode Recommendation Logic:**

```javascript
function recommendImportMode(fileCount, metadata) {
  if (fileCount < 50 && allFilesHaveGoodMetadata(metadata)) {
    return {
      mode: 'quick',
      reason: 'All files have complete metadata. Quick import will be fast and accurate.'
    };
  } else if (fileCount > 500) {
    return {
      mode: 'deep',
      reason: 'Large library detected. Deep scan recommended for initial import.'
    };
  } else if (someFilesMissingMetadata(metadata)) {
    return {
      mode: 'deep',
      reason: 'Some files have incomplete metadata. Deep scan will identify them accurately.'
    };
  } else {
    return {
      mode: 'quick',
      reason: 'Small batch with good metadata. Quick import recommended.'
    };
  }
}
```

**Testing:**

**Unit Tests:**
- `test_quick_import_with_complete_metadata()` - Verify skips fingerprinting
- `test_quick_import_creates_default_passages()` - Verify 2-sec lead-in/out
- `test_quick_import_fallback_to_deep()` - Verify fallback when metadata insufficient
- `test_deep_import_full_workflow()` - Verify all 7 phases execute

**Integration Tests:**
- `test_import_20_files_quick_mode()` - End-to-end quick import
- `test_import_20_files_deep_mode()` - End-to-end deep import
- `test_mode_recommendation_logic()` - Verify system recommends correct mode

**Performance Tests:**
- `benchmark_quick_vs_deep()` - Verify Quick Import is 5-10x faster

---

## 7. Success Metrics

### 7.1 Quantitative Metrics

**Time to Import (Incremental Scenario: 20 files)**
- **Current:** ~10 minutes
- **Target (Phase 1):** ~10 minutes (unchanged)
- **Target (Phase 2):** ~2 minutes (80% reduction)

**Clicks to Import (Repeat Location)**
- **Current:** 5 clicks (Open UI → Import page → Enter path → Start → Monitor)
- **Target (Phase 1):** 2 clicks (Select profile → Import)
- **Target (Phase 2):** 2 clicks (unchanged, but faster)

**User Attention Required**
- **Current:** Moderate (must monitor 10-minute process)
- **Target (Phase 1):** Moderate (process still 10 min)
- **Target (Phase 2):** Low (2-minute process, can watch progress bar)

**Session Blocking Rate**
- **Current:** 100% (single session only)
- **Target (Phase 1):** 100% (unchanged)
- **Target (Phase 3):** 0% (queue allows multiple imports)

### 7.2 Qualitative Metrics

**User Sentiment:**
- Survey question: "How would you rate the import experience?" (1-5 scale)
- **Current baseline:** Unknown (assume 3/5 based on pain points identified)
- **Target (Phase 1):** 3.5/5 (improved convenience)
- **Target (Phase 2):** 4.5/5 (significantly faster, more convenient)

**Cognitive Load:**
- Observation: Do first-time users understand the import process?
- **Current:** High (7-phase checklist, technical terminology)
- **Target (Phase 1):** Medium (progressive disclosure reduces initial complexity)
- **Target (Phase 2):** Medium (mode choice adds decision, but recommendation reduces cognitive load)

**Error Recovery:**
- Metric: % of users who successfully complete import after encountering errors
- **Current:** Unknown (assume 80% - errors displayed but not actionable)
- **Target:** 90% (improved error messages, fallback strategies)

---

## 8. Risk Mitigation

### 8.1 Risk: Quick Import Creates Poor Quality Passages

**Failure Mode:**
Default 2-second lead-in/lead-out timing is incorrect for some songs, resulting in abrupt crossfades.

**Probability:** Medium (depends on music variety - classical has long intros/outros, EDM does not)

**Impact:** Medium (playback quality degraded, user must manually fix or re-import)

**Mitigation Strategies:**

1. **Genre-Based Defaults:**
   - Classical: 5 sec lead-in/out
   - Rock/Pop: 2 sec lead-in/out
   - EDM: 1 sec lead-in/out
   - Read genre from ID3 tags, apply appropriate defaults

2. **Background Analysis Queue:**
   - Quick Import marks passages for "deep analysis later"
   - Background worker processes queue during idle time (e.g., overnight)
   - User never waits, quality improves over time

3. **Manual Upgrade:**
   - UI shows "Quick Import" badge on passages
   - User can select passages and click "Analyze Now" to run full analysis on-demand

4. **Validation:**
   - If Quick Import detects unusual file characteristics (e.g., very long duration, unusual waveform), suggest Deep Scan

**Residual Risk:** Low (multiple fallbacks, user retains control)

### 8.2 Risk: Users Always Choose Quick Import, Defeat Purpose

**Failure Mode:**
Users always click "Quick Import" even when Deep Scan is appropriate, resulting in poor library quality.

**Probability:** Medium (users prefer speed over quality, ignore recommendations)

**Impact:** Medium (library has suboptimal passages, defeats WKMP's core value proposition)

**Mitigation Strategies:**

1. **Smart Defaults:**
   - System pre-selects recommended mode (not just suggests)
   - User must actively choose to override recommendation

2. **Educational Messaging:**
   - Show comparison: "Quick Import: 2 min, basic quality" vs "Deep Scan: 10 min, optimal quality"
   - Explain tradeoff: "Quick Import uses default crossfade timing. Deep Scan analyzes each song for perfect transitions."

3. **First-Time Experience:**
   - First import always uses Deep Scan (no choice offered)
   - After successful first import, unlock Quick Import with explanation of when to use it

4. **Forced Deep Scan for Bulk:**
   - If file count >100, disable Quick Import option
   - Show message: "Deep Scan required for large imports to ensure quality"

5. **Quality Feedback:**
   - If user reports poor crossfades, suggest re-importing with Deep Scan
   - Track passage quality metrics, warn if Quick Import results have high skip rate

**Residual Risk:** Low-Medium (user education is difficult, but smart defaults help)

### 8.3 Risk: Import Profiles Contain Stale Paths

**Failure Mode:**
User deletes/moves folder, profile references invalid path, import fails.

**Probability:** Low (users generally maintain folder structure)

**Impact:** Low (error message displayed, user updates profile)

**Mitigation Strategies:**

1. **Path Validation:**
   - Before import, check if folder exists
   - Show error: "Profile 'Incoming Music' references missing folder: /home/user/Music/Incoming. Update profile?"

2. **Automatic Cleanup:**
   - Periodically (e.g., on app startup), validate all profile paths
   - Mark invalid profiles with warning icon
   - Suggest deletion of stale profiles

3. **Relative Paths (Optional):**
   - Allow profiles to use relative paths from music library root
   - If user moves entire library, profiles remain valid

**Residual Risk:** Low (validation catches issue, user can fix easily)

---

## 9. Future Enhancements (Out of Scope)

### 9.1 Drag-and-Drop Import

**Description:**
Allow users to drag audio files directly onto wkmp-ai UI to import.

**Benefits:**
- Reduces clicks (no folder path entry)
- Familiar UX pattern (used by many file managers)

**Complexity:**
- Frontend: HTML5 drag-and-drop API
- Backend: Multipart file upload, temp storage

**Recommendation:** Consider for future release if user demand exists.

---

### 9.2 Watched Folders (Revisited)

**Description:**
As discussed in Approach C, automatic folder monitoring.

**Recommendation:**
- **Defer** until Profiles + Quick Import are proven successful
- **Rationale:** Quick Import + Profiles may satisfy "easy incremental import" need without complexity of folder watching
- **Revisit** if user feedback indicates strong demand for zero-click import

---

### 9.3 Import Scheduling

**Description:**
Schedule imports to run at specific times (e.g., "Import from /incoming every night at 2 AM").

**Benefits:**
- Runs during low-usage hours (doesn't compete with playback for disk I/O)
- Users wake up to newly imported music

**Complexity:**
- Scheduling system (cron-like)
- Notification system (email/desktop alerts)

**Recommendation:** Consider if watched folders are implemented (natural pairing).

---

### 9.4 Cloud Storage Integration

**Description:**
Import directly from cloud storage (Dropbox, Google Drive, etc.) without downloading to local disk first.

**Benefits:**
- Enables import from remote music collections
- Supports distributed/multi-device setups

**Complexity:**
- OAuth integration with cloud providers
- Streaming audio file analysis (can't assume local file paths)
- Network latency handling

**Recommendation:** Out of scope for core WKMP (local music player focus).

---

## 10. Conclusion

### 10.1 Summary

**Current State:**
wkmp-ai has a robust, well-architected import system with excellent progress visibility. However, it treats all imports equally (7-phase heavyweight workflow), causing friction for incremental imports.

**Key Pain Points:**
1. Same workflow for 2 files or 2,000 files (no fast path)
2. Manual initiation required (no location memory)
3. Technical complexity exposed (overwhelming for casual users)
4. Single-session constraint (cannot queue imports)

**Recommended Solution:**
- **Phase 1 (Quick Wins):** Import Profiles + Progressive Disclosure UI
  - Low effort (3-4 days), Low risk, High user satisfaction
  - Reduces clicks by 60%, reduces cognitive load

- **Phase 2 (High Impact):** Quick Import Mode
  - Medium effort (2-4 days), Low-Medium risk, Very high user satisfaction
  - Reduces time by 80% for incremental imports
  - Maintains quality with smart defaults and background analysis

**Deferred:**
- Watched Folders (high complexity, architectural misalignment)
- Import Queue (moderate value, can revisit if user demand)

### 10.2 Expected Outcomes

**After Phase 1 + 2 Implementation:**

**Bulk Import Experience:**
- Time: Unchanged (~2-3 hours for 5,000 files)
- UX: Improved (simpler UI, less intimidating)
- Quality: Unchanged (full 7-phase analysis)

**Incremental Import Experience:**
- Time: 80% faster (10 min → 2 min for 20 files)
- Clicks: 60% fewer (5 clicks → 2 clicks)
- UX: Dramatically improved (fire-and-forget possible)
- Quality: Initially default, upgrades via background analysis

**User Sentiment:**
- Casual users: "Much easier to add new music!"
- Power users: "Love having both Quick and Deep options"
- First-time users: "Interface is clear and not overwhelming"

---

## Next Steps

This analysis is complete. Implementation planning requires explicit user authorization.

**To proceed with implementation:**

1. **Review analysis findings** and select preferred approach(es)
   - Recommended: Phase 1 (Profiles + Progressive UI) → Phase 2 (Quick Import)
   - Alternative: Pick individual approaches based on priorities

2. **Make any necessary decisions** on identified decision points:
   - Quick Import: Always available vs unlocked after first Deep Scan?
   - Background analysis: Automatic vs user-initiated?
   - Import mode selection: Radio buttons vs single button with recommendation?

3. **Run `/plan [specification_file]`** to create detailed implementation plan
   - /plan will generate: requirements analysis, test specifications, increment breakdown

4. **/plan will generate:**
   - Requirements analysis and traceability
   - Acceptance test specifications (Given/When/Then)
   - Increment breakdown with tasks and deliverables
   - Risk assessment and mitigation steps

**User retains full authority over:**
- Whether to implement any recommendations
- Which approach to adopt
- When to proceed to implementation
- Modifications to suggested approaches

---

**Document Status:** Analysis Complete, Ready for Stakeholder Review
**Analysis Quality:** Comprehensive (current state, research, options, recommendations)
**Implementation Readiness:** Detailed guidance provided, ready for /plan workflow if approved
