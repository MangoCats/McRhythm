# Error Handling Tests - wkmp-ai

**Requirements:** AIA-ERR-010, AIA-ERR-020
**Priority:** P0 (Critical)
**Test Count:** 11

---

## TEST-057: Warning Severity (Continue Processing)

**Requirement:** AIA-ERR-010
**Type:** Integration
**Priority:** P0

**Given:**
- Import session with file missing album art
- Error severity: Warning

**When:**
- Process file

**Then:**
- Warning logged
- File import continues
- Passage created successfully
- No album art extracted (NULL in database)
- Import does NOT abort

**Acceptance Criteria:**
- ✅ Warning logged with file path
- ✅ Passage created in database
- ✅ Import continues to next file
- ✅ Final status = COMPLETED (not FAILED)

---

## TEST-058: Skip File Severity

**Requirement:** AIA-ERR-010
**Type:** Integration
**Priority:** P0

**Given:**
- Import session with corrupt audio file
- Error severity: Skip File

**When:**
- Attempt to decode corrupt file

**Then:**
- Error logged: DECODE_ERROR
- Current file skipped
- Next file processed normally
- Skipped file NOT in passages table
- files_failed counter incremented

**Acceptance Criteria:**
- ✅ Error code = DECODE_ERROR
- ✅ Corrupt file not in database
- ✅ Next file processes successfully
- ✅ files_failed = 1 in completion summary

---

## TEST-059: Critical Severity (Abort Import)

**Requirement:** AIA-ERR-010
**Type:** Integration
**Priority:** P0

**Given:**
- Import session in progress
- Database write error occurs (disk full)
- Error severity: Critical

**When:**
- Database write fails

**Then:**
- Import immediately aborted
- State = FAILED
- Error reported via SSE and status endpoint
- No further files processed
- Partial data may exist in database

**Acceptance Criteria:**
- ✅ Import state = FAILED
- ✅ Error code = DATABASE_ERROR
- ✅ No files processed after error
- ✅ Completion event shows failure

---

## TEST-060: Corrupt Audio File Skipped

**Requirement:** AIA-ERR-010
**Type:** Integration
**Priority:** P0

**Given:**
- File with .mp3 extension but invalid content
- symphonia decode failure

**When:**
- Attempt fingerprint generation

**Then:**
- Error: DECODE_ERROR
- Severity: Skip File
- Error message: "Failed to decode audio"
- File path logged
- Import continues

**Acceptance Criteria:**
- ✅ Decode error caught gracefully
- ✅ No panic or crash
- ✅ Error details in SSE error event
- ✅ files_failed counter incremented

---

## TEST-061: Database Write Error (Critical)

**Requirement:** AIA-ERR-010
**Type:** Integration
**Priority:** P0

**Given:**
- Database connection lost mid-import
- OR disk full
- OR database locked

**When:**
- Attempt to insert passage

**Then:**
- Error: DATABASE_ERROR
- Severity: Critical
- Import aborted immediately
- State = FAILED
- Error message includes SQLite error details

**Acceptance Criteria:**
- ✅ Import stops immediately
- ✅ No corruption in database
- ✅ Transaction rolled back
- ✅ Clear error message to user

---

## TEST-062: Missing Album Art (Warning)

**Requirement:** AIA-ERR-010
**Type:** Integration
**Priority:** P0

**Given:**
- Audio file with no embedded album art
- Album art extraction attempted

**When:**
- Extract metadata

**Then:**
- Warning: MISSING_ALBUM_ART
- Severity: Warning
- File processed normally
- No album art saved
- Import continues

**Acceptance Criteria:**
- ✅ Warning logged (not error)
- ✅ Passage created successfully
- ✅ files_processed counter incremented
- ✅ No impact on import success

---

## TEST-063: Error via SSE Event

**Requirement:** AIA-ERR-020
**Type:** Integration
**Priority:** P0

**Given:**
- SSE connection active
- Error occurs (MBID_LOOKUP_FAILED)

**When:**
- MusicBrainz lookup returns 404

**Then:**
- SSE error event sent:
  ```json
  {
    "type": "error",
    "session_id": "uuid",
    "file_path": "unknown_track.mp3",
    "error_code": "MBID_LOOKUP_FAILED",
    "error_message": "Recording not found in MusicBrainz",
    "timestamp": "2025-10-27T12:34:58Z"
  }
  ```

**Acceptance Criteria:**
- ✅ Event sent immediately when error occurs
- ✅ error_code standardized
- ✅ error_message human-readable
- ✅ file_path included

---

## TEST-064: Error in Status Endpoint

**Requirement:** AIA-ERR-020
**Type:** Integration
**Priority:** P0

**Given:**
- Import with 3 errors encountered
- Client polls `/import/status/{session_id}`

**When:**
- Get status

**Then:**
- Response includes errors array:
  ```json
  {
    "errors": [
      {"file_path": "file1.mp3", "error_code": "DECODE_ERROR", "error_message": "..."},
      {"file_path": "file2.flac", "error_code": "MBID_LOOKUP_FAILED", "error_message": "..."},
      {"file_path": "file3.ogg", "error_code": "ACOUSTID_NO_MATCH", "error_message": "..."}
    ]
  }
  ```

**Acceptance Criteria:**
- ✅ All errors in array
- ✅ Chronological order
- ✅ Error details complete
- ✅ Array persists for session lifetime

---

## TEST-065: Error in Completion Summary

**Requirement:** AIA-ERR-020
**Type:** Integration
**Priority:** P0

**Given:**
- Import completes with files_failed > 0

**When:**
- Import reaches COMPLETED state

**Then:**
- Completion event includes error summary:
  ```json
  {
    "type": "completed",
    "files_processed": 95,
    "files_failed": 5,
    "error_summary": {
      "DECODE_ERROR": 3,
      "MBID_LOOKUP_FAILED": 2
    }
  }
  ```

**Acceptance Criteria:**
- ✅ files_failed count accurate
- ✅ error_summary groups by error_code
- ✅ Counts per error type correct
- ✅ User can identify problem patterns

---

## TEST-066: Error Details Include File Path

**Requirement:** AIA-ERR-020
**Type:** Integration
**Priority:** P0

**Given:**
- File-specific error (not session-level)

**When:**
- Error occurs

**Then:**
- Error details include full file path
- Path is relative to root folder OR absolute
- Path is consistent across all error reports (SSE, status, completion)

**Acceptance Criteria:**
- ✅ file_path field populated
- ✅ Path format consistent
- ✅ User can locate problem file
- ✅ No truncation of long paths

---

## TEST-067: Error Code Enumeration

**Requirement:** AIA-ERR-020
**Type:** Unit
**Priority:** P0

**Given:**
- Standard error codes defined

**When:**
- Validate error code enum

**Then:**
- Error codes include:
  - DECODE_ERROR
  - UNSUPPORTED_FORMAT
  - ACOUSTID_NO_MATCH
  - ACOUSTID_NETWORK_ERROR
  - MBID_LOOKUP_FAILED
  - MUSICBRAINZ_RATE_LIMIT
  - MUSICBRAINZ_NETWORK_ERROR
  - ACOUSTICBRAINZ_NO_DATA
  - DATABASE_ERROR
  - MISSING_ALBUM_ART (Warning)
  - FINGERPRINT_FAILED

**Acceptance Criteria:**
- ✅ All codes defined in enum
- ✅ Codes are uppercase with underscores
- ✅ Codes map to HTTP-friendly strings
- ✅ Documentation includes severity per code

---

## Test Implementation Notes

**Framework:** `cargo test --test error_handling_tests -p wkmp-ai`

**Error Injection Helper:**
```rust
// Inject database error
#[tokio::test]
async fn test_database_error_critical() {
    let db = create_broken_db_pool().await; // Connection fails

    let result = import_file(&db, "test.mp3").await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), "DATABASE_ERROR");
}

// Inject corrupt file
#[tokio::test]
async fn test_decode_error_skip() {
    let corrupt_file = create_corrupt_mp3();

    let result = fingerprint_file(&corrupt_file).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), "DECODE_ERROR");
}
```

**Error Code Enum:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    DecodeError,
    UnsupportedFormat,
    AcoustIDNoMatch,
    AcoustIDNetworkError,
    MBIDLookupFailed,
    MusicBrainzRateLimit,
    MusicBrainzNetworkError,
    AcousticBrainzNoData,
    DatabaseError,
    MissingAlbumArt,
    FingerprintFailed,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DecodeError => "DECODE_ERROR",
            Self::UnsupportedFormat => "UNSUPPORTED_FORMAT",
            // ...
        }
    }

    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::DecodeError => ErrorSeverity::SkipFile,
            Self::DatabaseError => ErrorSeverity::Critical,
            Self::MissingAlbumArt => ErrorSeverity::Warning,
            // ...
        }
    }
}
```

---

End of error handling tests
