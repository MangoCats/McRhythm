# User Settings Export/Import Feature - Specification

**Category:** User Interface Enhancement
**Module:** wkmp-ui (User Interface microservice)
**Priority:** Medium
**Estimated Complexity:** Medium

---

## Purpose

Enable users to export their complete WKMP configuration (musical flavor preferences, timeslots, cooldown settings, and playback preferences) to a portable JSON file and import those settings on another WKMP instance or after database reset.

---

## Background

Users invest significant time configuring their musical flavor preferences and timeslot schedules. When setting up WKMP on a new machine, moving to a different database, or sharing configurations with friends, they currently must manually reconfigure all settings. This feature provides a convenient backup and portability mechanism.

---

## Requirements

### REQ-SE-010: Export Functionality
The system **must** provide a "Export Settings" button in the Settings page that, when clicked:
1. Gathers all user-configurable settings from the database
2. Serializes them to a JSON file with standardized structure
3. Triggers browser download with filename format: `wkmp-settings-YYYY-MM-DD.json`

**Included Settings:**
- Musical flavor preferences (all configured flavor vectors)
- Timeslot schedule (all timeslots with associated target flavors)
- Cooldown configuration (song/artist/work cooldown durations)
- Playback preferences (crossfade duration, fade curve type, volume)
- Queue preferences (auto-advance enabled, passage selection mode)

**Excluded Settings:**
- Authentication credentials (security)
- Database connection strings (environment-specific)
- API keys or secrets (security)
- Audit log data (not user configuration)

---

### REQ-SE-020: Export Format Validation
The exported JSON file **must** conform to a documented schema that:
1. Includes format version identifier (enables future format migrations)
2. Uses human-readable structure (allows manual inspection/editing)
3. Validates against JSON Schema specification
4. Includes export timestamp and WKMP version metadata

**Example structure:**
```json
{
  "format_version": "1.0",
  "exported_at": "2025-01-15T10:30:00Z",
  "wkmp_version": "0.2.0",
  "settings": {
    "musical_flavors": [...],
    "timeslots": [...],
    "cooldowns": {...},
    "playback": {...},
    "queue": {...}
  }
}
```

---

### REQ-SE-030: Import Functionality
The system **must** provide an "Import Settings" button in the Settings page that:
1. Opens file picker dialog (accept `.json` files only)
2. Validates uploaded file against schema
3. Shows preview of settings to be imported (with diff from current settings)
4. Requires explicit user confirmation before applying changes
5. Applies imported settings to database
6. Shows success/failure notification

---

### REQ-SE-040: Import Validation
The import process **must** validate the uploaded file and reject imports that:
- Fail JSON schema validation
- Use incompatible format version (major version mismatch)
- Contain invalid data types or out-of-range values
- Reference non-existent database entities (e.g., invalid passage IDs)

**Error Handling:**
- Display clear error messages indicating validation failure reason
- Do not modify database if validation fails
- Log validation errors for debugging

---

### REQ-SE-050: Merge Strategy
When importing settings that conflict with existing configuration, the system **must**:
1. Use "replace" strategy by default (imported settings overwrite existing)
2. Provide optional "merge" mode that:
   - Adds new timeslots without removing existing
   - Appends new musical flavors to existing set
   - Preserves existing cooldown values if not specified in import
3. Display merge strategy selector before applying import
4. Show clear indication of which settings will change

---

### REQ-SE-060: Data Integrity
The import process **must** maintain database integrity by:
1. Wrapping all database updates in a single transaction
2. Rolling back transaction if any update fails
3. Validating foreign key relationships before commit
4. Preserving existing passage library and playback history (only settings change)

**Atomicity Guarantee:**
Either all settings import successfully or none do (no partial imports).

---

### REQ-SE-070: User Feedback
The feature **must** provide clear user feedback throughout the workflow:
1. **Export:** Show "Exporting..." spinner during serialization (if >500ms)
2. **Export Success:** Toast notification with filename
3. **Import Upload:** Show file name and size after selection
4. **Import Preview:** Display side-by-side comparison of current vs. imported settings
5. **Import Progress:** Show "Applying settings..." spinner during database update
6. **Import Success:** Toast notification with count of changed settings
7. **Import Failure:** Error modal with specific failure reason and suggestions

---

### REQ-SE-080: Security Considerations
The feature **must** implement security measures:
1. Validate file size (reject files >5MB)
2. Sanitize all imported strings (prevent SQL injection)
3. Rate-limit import operations (max 5 imports per minute per user)
4. Log all export/import operations to audit trail (who, when, what changed)
5. Require authentication (logged-in users only)

---

## Success Criteria

The feature is complete when:

1. ✓ User can export settings to JSON file with one click
2. ✓ Exported file includes all specified settings in documented format
3. ✓ User can import settings from valid JSON file
4. ✓ Import validation rejects malformed or incompatible files
5. ✓ Import preview shows clear diff of changes
6. ✓ Database remains consistent after failed import (rollback works)
7. ✓ All operations logged to audit trail
8. ✓ All requirements have passing automated tests

---

## Test Scenarios

### Scenario 1: Complete Export/Import Cycle
1. Configure 3 custom musical flavors
2. Create 5 timeslots with different target flavors
3. Set custom cooldown values
4. Export settings → verify file downloads
5. Clear all settings in database
6. Import exported file → verify settings restored exactly

**Expected:** All settings restored to original state.

---

### Scenario 2: Import Validation - Invalid JSON
1. Create text file with invalid JSON syntax
2. Rename to `.json`
3. Attempt import

**Expected:** Error message: "Invalid JSON file. Please check file format."

---

### Scenario 3: Import Validation - Wrong Format Version
1. Create valid JSON with `"format_version": "99.0"`
2. Attempt import

**Expected:** Error message: "Incompatible settings format version 99.0. This WKMP version supports format 1.x."

---

### Scenario 4: Merge Strategy
1. Configure 2 timeslots (Morning, Evening)
2. Export settings
3. Manually edit JSON to add 1 new timeslot (Afternoon)
4. Import with "Merge" strategy selected

**Expected:** 3 timeslots present (Morning, Afternoon, Evening). Original 2 unchanged.

---

### Scenario 5: Transaction Rollback
1. Create valid import file
2. Manually corrupt database (e.g., delete required table row mid-import)
3. Attempt import

**Expected:** Import fails, error message displayed, no partial changes committed to database.

---

### Scenario 6: Large File Rejection
1. Create JSON file >5MB (e.g., thousands of musical flavors)
2. Attempt import

**Expected:** Error message: "File too large. Maximum size: 5MB."

---

### Scenario 7: Audit Trail Logging
1. Export settings
2. Import settings
3. Check audit log

**Expected:** Two log entries:
- "User [username] exported settings at [timestamp]"
- "User [username] imported settings at [timestamp] (X settings changed)"

---

## Implementation Notes

### API Endpoints (wkmp-ui)

**Export:**
```
GET /api/settings/export
Response: application/json (settings file)
```

**Import:**
```
POST /api/settings/import
Content-Type: multipart/form-data
Body: JSON file
Response: 200 OK with change summary, or 400 Bad Request with validation errors
```

**Preview:**
```
POST /api/settings/import/preview
Content-Type: multipart/form-data
Body: JSON file
Response: JSON diff showing current vs. imported settings
```

---

### Database Schema Changes

**New Table: `settings_exports`**
```sql
CREATE TABLE settings_exports (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  exported_at DATETIME NOT NULL,
  file_size_bytes INTEGER NOT NULL,
  FOREIGN KEY (user_id) REFERENCES users(id)
);
```

**New Table: `settings_imports`**
```sql
CREATE TABLE settings_imports (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,
  imported_at DATETIME NOT NULL,
  file_size_bytes INTEGER NOT NULL,
  format_version TEXT NOT NULL,
  merge_strategy TEXT NOT NULL, -- 'replace' or 'merge'
  settings_changed INTEGER NOT NULL,
  success BOOLEAN NOT NULL,
  error_message TEXT,
  FOREIGN KEY (user_id) REFERENCES users(id)
);
```

---

### JSON Schema Definition

The exported JSON must validate against this JSON Schema (stored in `docs/schemas/settings-export-v1.schema.json`):

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": ["format_version", "exported_at", "wkmp_version", "settings"],
  "properties": {
    "format_version": {
      "type": "string",
      "pattern": "^1\\.[0-9]+$"
    },
    "exported_at": {
      "type": "string",
      "format": "date-time"
    },
    "wkmp_version": {
      "type": "string"
    },
    "settings": {
      "type": "object",
      "properties": {
        "musical_flavors": {"type": "array"},
        "timeslots": {"type": "array"},
        "cooldowns": {"type": "object"},
        "playback": {"type": "object"},
        "queue": {"type": "object"}
      }
    }
  }
}
```

---

## Dependencies

- **Prerequisite:** User authentication system (users must be logged in)
- **Prerequisite:** Settings database schema (tables: `musical_flavors`, `timeslots`, `settings`, etc.)
- **Library Dependency:** JSON Schema validator (e.g., `jsonschema` crate for Rust)
- **UI Dependency:** File picker component (HTML5 `<input type="file">`)
- **UI Dependency:** Diff viewer component (show current vs. imported settings)

---

## Future Enhancements (Out of Scope for v1)

- **Cloud Backup:** Automatic backup to cloud storage (Dropbox, Google Drive)
- **Sharing:** Share settings via URL or QR code
- **Versioning:** Maintain history of imported settings (restore previous versions)
- **Selective Export:** Choose which settings to include in export
- **Import from URL:** Provide URL to JSON file instead of upload

---

## References

- **User Authentication:** See `docs/SPEC006-authentication.md` (if exists)
- **Settings Database Schema:** See `docs/IMPL001-database_schema.md`
- **Audit Logging:** See `docs/SPEC009-audit_trail.md` (if exists)

---

**Document Version:** 1.0
**Last Updated:** 2025-01-15
**Author:** WKMP Development Team
