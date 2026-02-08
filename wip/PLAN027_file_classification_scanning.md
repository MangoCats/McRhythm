# PLAN027: File Classification During Scanning Phase

**Category:** Implementation Plan
**Created:** 2025-11-16
**Status:** Ready for Implementation
**Specification References:**
- [SPEC032-audio_ingest_architecture.md](../docs/SPEC032-audio_ingest_architecture.md) v2.2 (File Classification sections)
- [IMPL008-audio_ingest_api.md](../docs/IMPL008-audio_ingest_api.md) v1.1 (File Classification Endpoint)

---

## Executive Summary

Enhance wkmp-ai's SCANNING phase to classify ALL discovered files into three categories (audio, image, other) and provide a dedicated UI report page showing complete file inventory with human-readable sizes and metadata. This feature provides users with comprehensive visibility into root folder contents after the scan completes.

**Key Changes:**
- Extend file scanner to classify all files by extension during directory traversal
- Add `FileClassification` data structure to import session state
- Implement `/file-report` UI page with category tabs and scrollable file lists
- Add `GET /api/import/file-classification` REST endpoint with pagination
- Display link to classification report in progress page after SCANNING completes

**Scope:** wkmp-ai (Audio Ingest microservice) only - no changes to other modules

---

## Specification References

### SPEC032 v2.2 Sections

**File Classification Logic:**
- **[AIA-CLASSIFY-010]:** File category definitions and classification scope
- **[AIA-CLASSIFY-020]:** Extension lists for audio/image categories
- **[AIA-CLASSIFY-030]:** Session state data structures

**UI Implementation:**
- **[AIA-CLASSIFY-UI-010]:** Report page route and layout
- **[AIA-CLASSIFY-UI-020]:** Category tab behavior and column specifications
- **[AIA-CLASSIFY-UI-030]:** Human-readable size formatting algorithm
- **[AIA-CLASSIFY-UI-040]:** REST API endpoint specification
- **[AIA-CLASSIFY-UI-050]:** Workflow integration (non-blocking, optional)

### IMPL008 v1.1 Section

**API Endpoint:**
- `GET /api/import/file-classification` - Complete endpoint specification with pagination

---

## Requirements Analysis

### Functional Requirements

**FR-1: File Classification During Scan**
- **Source:** [AIA-CLASSIFY-010]
- **Description:** During SCANNING phase, classify ALL discovered files into three categories
- **Categories:** Audio (MP3, FLAC, OGG, M4A, AAC, OPUS, WAV), Image (JPG, PNG, GIF, BMP, WEBP, TIFF), Other (all remaining)
- **Classification Method:** Case-insensitive extension matching (no file content inspection)
- **Scope:** Recursive traversal, respect symlink/hidden file settings

**FR-2: File Metadata Collection**
- **Source:** [AIA-CLASSIFY-030]
- **Description:** For each classified file, collect metadata
- **Fields:** Absolute path, file size (bytes), last modified timestamp
- **Storage:** In-memory import session state (persists until session ends)

**FR-3: Classification Report UI**
- **Source:** [AIA-CLASSIFY-UI-010]
- **Description:** Dedicated web page at `/file-report` showing classification results
- **Access:** Link displayed in progress page after SCANNING completes
- **Layout:** Category tabs (audio/image/other), scrollable file list, summary statistics

**FR-4: Category Tab Switching**
- **Source:** [AIA-CLASSIFY-UI-020]
- **Description:** User can switch between categories to view different file lists
- **Tab Display:** Category name, count, total size in human-readable format
- **File Display:** Path, size (human-readable), modified date in three-column table

**FR-5: Pagination Support**
- **Source:** [AIA-CLASSIFY-UI-040]
- **Description:** For categories with >500 files, paginate results
- **Default Page Size:** 500 files
- **Maximum Page Size:** 1000 files
- **Implementation:** Client-side pagination (all data loaded, client renders pages)

**FR-6: Human-Readable Size Formatting**
- **Source:** [AIA-CLASSIFY-UI-030]
- **Description:** Display file sizes in appropriate units
- **Format:**
  - `< 1 KB`: "1,234 B"
  - `< 1 MB`: "42.5 KB"
  - `< 1 GB`: "15.3 MB"
  - `>= 1 GB`: "2.1 GB"
- **Algorithm:** Binary units (1024-based), 1 decimal place for KB/MB/GB

**FR-7: REST API Endpoint**
- **Source:** [AIA-CLASSIFY-UI-040], IMPL008
- **Route:** `GET /api/import/file-classification`
- **Query Parameters:** `category` (audio|image|other), `offset`, `limit`
- **Response:** JSON with session metadata, category statistics, file lists
- **Error Codes:** 404 (session not found), 409 (scan not complete), 400 (invalid category)

**FR-8: Workflow Integration**
- **Source:** [AIA-CLASSIFY-UI-050]
- **Description:** Non-blocking, optional report viewing
- **Behavior:**
  - SCANNING completes → Display "View File Classification Report" link
  - User clicks link → Open `/file-report` (same tab or new tab)
  - User clicks "Continue to Processing" → Return to progress page, start PROCESSING
  - User skips report → PROCESSING starts automatically after SCANNING
- **Persistence:** Classification data available until session ends

### Non-Functional Requirements

**NFR-1: Performance - Minimal Scanning Overhead**
- **Requirement:** File classification must NOT significantly slow down SCANNING phase
- **Strategy:** Extension lookup during existing directory traversal (no additional I/O)
- **Target:** < 5% scan time increase (negligible)

**NFR-2: Memory Efficiency**
- **Requirement:** Classification data must fit in memory for large libraries
- **Estimate:** 1000 files × 300 bytes average path = ~300 KB per 1000 files
- **Target:** Support 50,000 files = ~15 MB classification data (acceptable)

**NFR-3: UI Responsiveness**
- **Requirement:** Report page must render quickly even for large file counts
- **Strategy:** Client-side pagination (render 500 files at a time)
- **Target:** < 500ms page render time for any file count

**NFR-4: API Response Time**
- **Requirement:** `/api/import/file-classification` must respond quickly
- **Strategy:** In-memory session state (no database queries)
- **Target:** < 100ms response time (even for 50,000 files)

---

## Implementation Increments

### Increment 1: Core Classification Logic

**Goal:** Extend file scanner to classify all files during SCANNING phase

**Tasks:**
1. Define extension constants in `wkmp-ai/src/services/file_scanner.rs`
   - `AUDIO_EXTENSIONS: &[&str]`
   - `IMAGE_EXTENSIONS: &[&str]`
2. Add `FileInfo` and `FileClassification` structs to `wkmp-ai/src/models/session.rs`
3. Modify `scan_directory()` function to collect metadata for ALL files
   - For each file, determine category via extension lookup (case-insensitive)
   - Collect `PathBuf`, file size (`std::fs::metadata().len()`), modified time
   - Append to appropriate `Vec<FileInfo>` in classification struct
4. Store `FileClassification` in `ImportSession` state after SCANNING completes

**Acceptance Tests:**
- **AT1.1:** SCANNING phase discovers 100 MP3 files → `audio_files.len() == 100`
- **AT1.2:** SCANNING phase discovers 10 JPG files → `image_files.len() == 10`
- **AT1.3:** SCANNING phase discovers 5 TXT files → `other_files.len() == 5`
- **AT1.4:** File with `.MP3` extension (uppercase) → Classified as audio (case-insensitive)
- **AT1.5:** File with no extension → Classified as other
- **AT1.6:** File size metadata matches `std::fs::metadata().len()` output
- **AT1.7:** Modified timestamp matches file metadata `modified()` value

**Implementation Notes:**
- Use `Path::extension()` to extract extension, convert to lowercase for comparison
- Handle `None` extension case (files without extensions → other category)
- Collect metadata synchronously during sequential directory traversal (no parallelization needed)

---

### Increment 2: REST API Endpoint

**Goal:** Implement `/api/import/file-classification` endpoint with pagination

**Tasks:**
1. Create `wkmp-ai/src/handlers/file_classification.rs` module
2. Implement `get_file_classification()` handler function
   - Extract session from shared state (`Arc<RwLock<Option<ImportSession>>>`)
   - Validate SCANNING phase completed (return 409 if still scanning)
   - Parse query parameters: `category`, `offset`, `limit`
   - If `category` specified: Return single-category response with pagination
   - If no `category`: Return all-categories response (first 500 files per category)
3. Add route to Axum router: `Router::new().route("/api/import/file-classification", get(get_file_classification))`
4. Implement human-readable size formatting function `format_file_size()`

**Acceptance Tests:**
- **AT2.1:** `GET /api/import/file-classification` (no params) → Returns all categories with first 500 files each
- **AT2.2:** `GET /api/import/file-classification?category=audio` → Returns audio files only (first 500)
- **AT2.3:** `GET /api/import/file-classification?category=audio&offset=500&limit=500` → Returns audio files 500-999
- **AT2.4:** `GET /api/import/file-classification?category=invalid` → 400 Bad Request
- **AT2.5:** `GET /api/import/file-classification` before SCANNING completes → 409 Conflict
- **AT2.6:** `GET /api/import/file-classification` with invalid session → 404 Not Found
- **AT2.7:** File size 1023 bytes → "1,023 B" format
- **AT2.8:** File size 1024 bytes → "1.0 KB" format
- **AT2.9:** File size 1048576 bytes (1 MB) → "1.0 MB" format
- **AT2.10:** File size 1073741824 bytes (1 GB) → "1.0 GB" format
- **AT2.11:** Response includes `scan_completed_at` timestamp
- **AT2.12:** Files sorted alphabetically by path (case-insensitive)

**Implementation Notes:**
- Use `serde_json::json!` macro for response construction
- Implement custom serialization for `SystemTime` → ISO 8601 string conversion
- For all-categories response, limit each category to 500 files (first page only)
- For single-category response, include `pagination` object with `offset`, `limit`, `total_count`, `has_more`

---

### Increment 3: File Classification Report UI Page

**Goal:** Create `/file-report` HTML page with category tabs and file lists

**Tasks:**
1. Create `wkmp-ai/static/file-report.html` template
   - Three-tab layout (Audio Files, Image Files, Other Files)
   - Tab labels show count and total size: "Audio Files (1,247 files, 8.2 GB)"
   - Active tab highlighted with CSS styling
   - Scrollable table with columns: Path, Size, Modified
2. Create `wkmp-ai/static/file-report.js` JavaScript module
   - Fetch classification data from `/api/import/file-classification`
   - Render category tabs with counts/sizes
   - Implement tab click handlers to switch displayed file list
   - Render file table with human-readable sizes
   - Implement client-side pagination (if >500 files, show "Load More" button)
3. Create `wkmp-ai/static/file-report.css` styles
   - Tab styling (active/inactive states)
   - Table styling (fixed header, scrollable body)
   - Responsive layout (mobile-friendly)
4. Add route to Axum router: `Router::new().route("/file-report", get(serve_file_report))`

**Acceptance Tests:**
- **AT3.1:** Navigate to `/file-report` → Page loads successfully
- **AT3.2:** Audio Files tab shows correct count and total size
- **AT3.3:** Click "Image Files" tab → File list switches to image files
- **AT3.4:** File table displays path, human-readable size, modified date
- **AT3.5:** Modified date format: "2024-01-15" for recent files, full timestamp for older files
- **AT3.6:** Files sorted alphabetically by path
- **AT3.7:** Scrolling table body (fixed header, scrollable content)
- **AT3.8:** Click "Continue to Processing" button → Return to `/import-progress` page
- **AT3.9:** Click "Cancel Import" button → Cancel session, return to home
- **AT3.10:** Page refresh → Classification data persists (fetched from session state)

**Implementation Notes:**
- Use CSS Grid for tab layout, Flexbox for table columns
- Implement "sticky" table header using `position: sticky; top: 0;`
- For date formatting, use JavaScript `Date.toISOString().split('T')[0]` for YYYY-MM-DD
- For >500 files, implement "Load More" button that fetches next page via API

---

### Increment 4: Progress Page Integration

**Goal:** Display "View File Classification Report" link in progress page after SCANNING

**Tasks:**
1. Modify `wkmp-ai/static/import-progress.js` to detect SCANNING completion
   - Listen for SSE event indicating SCANNING → PROCESSING transition
   - When SCANNING completes, display link: `<a href="/file-report">View File Classification Report</a>`
   - Position link above "Current Phase" section (non-intrusive)
2. Add CSS styling for report link
   - Prominent but not blocking
   - Optional styling (user can ignore and continue)
3. Ensure link persists if user refreshes progress page (check session state via API)

**Acceptance Tests:**
- **AT4.1:** SCANNING completes → "View File Classification Report" link appears
- **AT4.2:** Click link → Navigate to `/file-report` page
- **AT4.3:** Link visible even if user refreshes progress page after SCANNING
- **AT4.4:** Link NOT visible during SCANNING phase
- **AT4.5:** Link NOT visible before SCANNING starts
- **AT4.6:** PROCESSING phase auto-starts even if user does not view report (non-blocking)

**Implementation Notes:**
- Use SSE event: `event: state_changed, data: {"old_state": "SCANNING", "new_state": "PROCESSING"}`
- Add link to DOM via JavaScript: `document.getElementById('report-link-container').innerHTML = '<a>...</a>'`
- Link should open in same tab (not new window) for simplicity

---

### Increment 5: Testing and Documentation

**Goal:** Comprehensive testing and documentation updates

**Tasks:**
1. Write unit tests for extension classification logic
   - Test audio extensions (all 7 formats)
   - Test image extensions (all 8 formats)
   - Test case-insensitive matching
   - Test files without extensions
2. Write integration tests for API endpoint
   - Test all query parameter combinations
   - Test error responses (404, 409, 400)
   - Test pagination logic
   - Test human-readable size formatting
3. Write end-to-end UI test
   - Automate browser test: start import, wait for SCANNING, open report, verify UI
   - Use headless browser (Playwright or Selenium)
4. Update `wkmp-ai/README.md` with file classification feature description
5. Add file classification to wkmp-ai user guide (if exists)

**Acceptance Tests:**
- **AT5.1:** All unit tests pass (>95% code coverage for new functions)
- **AT5.2:** All integration tests pass
- **AT5.3:** End-to-end UI test passes
- **AT5.4:** README.md updated with feature description
- **AT5.5:** User guide updated (if applicable)

**Implementation Notes:**
- Use `cargo test` for Rust unit/integration tests
- For E2E testing, consider Playwright (JavaScript) or headless Chrome
- Document pagination behavior clearly in user guide

---

## Traceability Matrix

| Requirement ID | Specification Reference | Increment | Acceptance Test(s) |
|----------------|------------------------|-----------|-------------------|
| FR-1 | [AIA-CLASSIFY-010] | 1 | AT1.1, AT1.2, AT1.3, AT1.4, AT1.5 |
| FR-2 | [AIA-CLASSIFY-030] | 1 | AT1.6, AT1.7 |
| FR-3 | [AIA-CLASSIFY-UI-010] | 3 | AT3.1, AT3.2, AT3.3, AT3.4 |
| FR-4 | [AIA-CLASSIFY-UI-020] | 3 | AT3.2, AT3.3, AT3.6 |
| FR-5 | [AIA-CLASSIFY-UI-040] | 2, 3 | AT2.2, AT2.3, AT3.8 |
| FR-6 | [AIA-CLASSIFY-UI-030] | 2 | AT2.7, AT2.8, AT2.9, AT2.10 |
| FR-7 | [AIA-CLASSIFY-UI-040], IMPL008 | 2 | AT2.1-AT2.12 |
| FR-8 | [AIA-CLASSIFY-UI-050] | 4 | AT4.1-AT4.6 |
| NFR-1 | Performance | 1 | (Manual verification: scan time increase < 5%) |
| NFR-2 | Memory | 1 | (Manual verification: memory usage for 50K files < 20 MB) |
| NFR-3 | UI Responsiveness | 3 | (Manual verification: page render < 500ms) |
| NFR-4 | API Performance | 2 | (Manual verification: API response < 100ms) |

**Total Requirements:** 12 (8 functional, 4 non-functional)
**Total Acceptance Tests:** 41 automated tests
**Coverage Target:** 100% of functional requirements

---

## Implementation Order

**Recommended sequence:**
1. Increment 1 (Core Classification Logic) - Foundation
2. Increment 2 (REST API Endpoint) - Testable backend
3. Increment 3 (UI Page) - User-facing feature
4. Increment 4 (Progress Page Integration) - Workflow integration
5. Increment 5 (Testing and Documentation) - Quality assurance

**Dependencies:**
- Increment 2 depends on Increment 1 (needs session state with classification data)
- Increment 3 depends on Increment 2 (UI fetches data from API)
- Increment 4 depends on Increment 3 (links to report page)
- Increment 5 depends on all previous (testing complete feature)

**Estimated Effort:**
- Increment 1: 4 hours (classification logic + session state)
- Increment 2: 3 hours (REST endpoint + pagination)
- Increment 3: 6 hours (HTML/CSS/JS UI page)
- Increment 4: 2 hours (progress page link integration)
- Increment 5: 4 hours (testing + documentation)
- **Total:** 19 hours (~2.5 development days)

---

## Risk Assessment

### Risk 1: Large File Count Performance

**Scenario:** User scans folder with 100,000 files → Classification data ~30 MB in memory, API response slow

**Probability:** Low-Medium (power users with massive libraries)

**Impact:** Medium (slow UI, high memory usage)

**Mitigation:**
- Implement lazy loading: Only fetch first 500 files per category initially
- Add "Load More" button for pagination (fetch on demand)
- Consider streaming response for large datasets (future enhancement)

**Residual Risk:** Low (pagination mitigates most performance issues)

---

### Risk 2: Extension-Based Classification Inaccuracy

**Scenario:** File named `image.mp3` contains image data → Misclassified as audio

**Probability:** Low (uncommon in practice)

**Impact:** Low (classification is for informational purposes, does not affect import)

**Mitigation:**
- Document that classification is extension-based (not content-based)
- Note in UI: "Classification based on file extension"
- Future enhancement: Add magic byte verification for accuracy

**Residual Risk:** Low (acceptable for informational report)

---

### Risk 3: UI Complexity for Non-Technical Users

**Scenario:** User confused by three-category report, does not understand purpose

**Probability:** Low-Medium (depends on user base)

**Impact:** Low (report is optional, does not block workflow)

**Mitigation:**
- Add help text to report page explaining purpose: "Review all files found during scan"
- Provide "Continue to Processing" button prominently (clear next action)
- Make report optional (user can skip directly to PROCESSING)

**Residual Risk:** Low (optional feature + clear help text)

---

## File Locations

### New Files

- `wkmp-ai/src/handlers/file_classification.rs` - REST API endpoint handler
- `wkmp-ai/static/file-report.html` - Report page HTML template
- `wkmp-ai/static/file-report.js` - Report page JavaScript
- `wkmp-ai/static/file-report.css` - Report page styles

### Modified Files

- `wkmp-ai/src/services/file_scanner.rs` - Add classification logic
- `wkmp-ai/src/models/session.rs` - Add `FileClassification`, `FileInfo` structs
- `wkmp-ai/src/main.rs` - Add `/file-report` and `/api/import/file-classification` routes
- `wkmp-ai/static/import-progress.js` - Add "View Report" link after SCANNING

### Test Files

- `wkmp-ai/tests/file_classification_tests.rs` - Unit tests for classification logic
- `wkmp-ai/tests/api_file_classification_tests.rs` - Integration tests for API endpoint
- `wkmp-ai/tests/e2e_file_report_ui_tests.rs` - End-to-end UI tests

---

## Success Criteria

**Feature is complete when:**
1. ✅ All 41 acceptance tests pass
2. ✅ Code review approved (focus on performance, memory usage)
3. ✅ Manual testing with real music library (1000+ files) successful
4. ✅ Documentation updated (README, user guide)
5. ✅ No regressions in existing SCANNING phase functionality
6. ✅ UI renders correctly on desktop and mobile browsers
7. ✅ API response time < 100ms for 10,000 files (performance target)
8. ✅ Classification overhead < 5% of total SCANNING time (performance target)

---

## Future Enhancements (Out of Scope)

**Not included in this plan (defer to future work):**
- Content-based classification (magic byte verification) for accuracy
- Filtering/searching file lists within report UI
- Exporting file classification report to CSV/JSON
- Highlighting duplicate files (same name in different folders)
- Integration with file organization tools (auto-move images to subfolder)

---

**Document Version:** 1.0
**Last Updated:** 2025-11-16
**Status:** Ready for Implementation
**Approved By:** (Pending user approval)
