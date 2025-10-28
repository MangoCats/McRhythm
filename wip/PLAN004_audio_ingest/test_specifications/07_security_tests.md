# Security Tests - wkmp-ai

**Requirements:** AIA-SEC-010, AIA-SEC-020
**Priority:** P0 (Critical)
**Test Count:** 7

---

## TEST-074: Root Folder Path Validation

**Requirement:** AIA-SEC-010
**Type:** Security
**Priority:** P0

**Given:**
- POST /import/start with root_folder path

**When:**
- Validate path

**Then:**
- Path must exist
- Path must be a directory (not a file)
- Path must be readable
- Path must not be system folder (/, /etc, /sys, /proc)

**Acceptance Criteria:**
- ✅ Non-existent path rejected (400 Bad Request)
- ✅ File path rejected (must be directory)
- ✅ Unreadable path rejected (permission denied)
- ✅ System folders rejected (/etc, /sys, /proc, /dev)

---

## TEST-075: Directory Traversal Prevention

**Requirement:** AIA-SEC-010
**Type:** Security
**Priority:** P0

**Given:**
- Root folder: /home/user/music
- Malicious file path: /home/user/music/../../etc/passwd
- OR symlink: /home/user/music/link → /etc/passwd

**When:**
- Process files

**Then:**
- Canonical path checked
- Files outside root folder rejected
- Symlink loops detected and skipped
- No access to /etc/passwd

**Acceptance Criteria:**
- ✅ Path canonicalization applied
- ✅ Files outside root rejected
- ✅ Symlink traversal blocked
- ✅ No unauthorized file access

---

## TEST-076: Parameter Range Validation

**Requirement:** AIA-SEC-010
**Type:** Security
**Priority:** P0

**Given:**
- Import parameters with out-of-range values:
  - rms_window_ms: -10 (negative)
  - lead_in_threshold_db: 50 (> 0dB)
  - import_parallelism: 1000 (> max 16)

**When:**
- Validate parameters

**Then:**
- rms_window_ms rejected (must be 10-1000)
- lead_in_threshold_db rejected (must be -60.0 to 0.0)
- import_parallelism rejected (must be 1-16)
- Response: 400 Bad Request with validation errors

**Acceptance Criteria:**
- ✅ All ranges enforced
- ✅ Clear error messages per parameter
- ✅ No invalid values processed
- ✅ Default values within range

---

## TEST-077: Symlink Loop Detection

**Requirement:** AIA-SEC-010
**Type:** Security
**Priority:** P0

**Given:**
- Directory structure:
  ```
  /music/
    a/ → symlink to b/
    b/ → symlink to a/
  ```
- Infinite loop potential

**When:**
- Scan /music/

**Then:**
- Loop detected
- Scan does not hang
- Warning logged
- Symlink directories skipped

**Acceptance Criteria:**
- ✅ No infinite loop
- ✅ Scan completes in finite time (<10s)
- ✅ Warning logged about loop
- ✅ No crash or timeout

---

## TEST-078: AcoustID API Key from Environment

**Requirement:** AIA-SEC-020
**Type:** Security
**Priority:** P0

**Given:**
- Environment variable: ACOUSTID_API_KEY=secret_key_123

**When:**
- Initialize AcoustID client

**Then:**
- API key loaded from environment
- NOT hardcoded in source
- NOT in configuration file

**Acceptance Criteria:**
- ✅ Key loaded from env var
- ✅ Server fails to start if key missing (with clear error)
- ✅ No default/placeholder keys in code
- ✅ Key validation on startup

---

## TEST-079: API Key Not in Logs

**Requirement:** AIA-SEC-020
**Type:** Security
**Priority:** P0

**Given:**
- ACOUSTID_API_KEY=secret_key_123
- AcoustID request fails (network error)

**When:**
- Error logged

**Then:**
- Log entry does NOT contain "secret_key_123"
- Request URL logged without client parameter
- API key redacted or omitted

**Acceptance Criteria:**
- ✅ API key never in log output
- ✅ Error messages don't leak key
- ✅ Request URLs sanitized
- ✅ Grep for key in logs returns empty

---

## TEST-080: API Key Not in Responses

**Requirement:** AIA-SEC-020
**Type:** Security
**Priority:** P0

**Given:**
- Client requests GET /import/status/{session_id}
- OR GET /parameters/global

**When:**
- Return response

**Then:**
- Response does NOT contain ACOUSTID_API_KEY
- Parameters response excludes API keys
- Error responses don't leak credentials

**Acceptance Criteria:**
- ✅ API key not in JSON responses
- ✅ Parameters response filters sensitive fields
- ✅ Error details sanitized
- ✅ No credential exposure in any endpoint

---

## Test Implementation Notes

**Framework:** `cargo test --test security_tests -p wkmp-ai`

**Path Validation:**
```rust
#[test]
fn test_directory_traversal_prevention() {
    let root = PathBuf::from("/home/user/music");
    let malicious = PathBuf::from("/home/user/music/../../etc/passwd");

    let scanner = FileScanner::new();
    let result = scanner.validate_path(&malicious, &root);

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "Path outside root folder");
}
```

**Symlink Loop Detection:**
```rust
#[test]
fn test_symlink_loop_detection() {
    let temp_dir = create_symlink_loop();
    let scanner = FileScanner::new();

    let start = Instant::now();
    let files = scanner.scan(&temp_dir).unwrap();
    let duration = start.elapsed();

    assert!(duration < Duration::from_secs(10), "Scan hung on symlink loop");
    assert_eq!(files.len(), 0, "Should not find files in loop");
}
```

**Parameter Validation:**
```rust
#[test]
fn test_parameter_range_validation() {
    let params = ImportParameters {
        rms_window_ms: 5000, // Out of range (max 1000)
        lead_in_threshold_db: 10.0, // Out of range (max 0.0)
        import_parallelism: 100, // Out of range (max 16)
        ..Default::default()
    };

    let result = params.validate();

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("rms_window_ms"));
    assert!(err.to_string().contains("lead_in_threshold_db"));
    assert!(err.to_string().contains("import_parallelism"));
}
```

**API Key Redaction:**
```rust
#[test]
fn test_api_key_not_in_logs() {
    std::env::set_var("ACOUSTID_API_KEY", "secret_test_key");

    let log_output = capture_logs(|| {
        let client = AcoustIDClient::new();
        // Trigger error that logs
        let _ = client.lookup("invalid").await;
    });

    assert!(
        !log_output.contains("secret_test_key"),
        "API key leaked in logs: {}", log_output
    );
}
```

---

End of security tests
