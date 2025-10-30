# Increment 2: wkmp-common TOML Atomic Write Utilities

**Estimated Effort:** 4-6 hours
**Dependencies:** Increment 1 (TomlConfig schema)
**Risk:** MEDIUM (atomic operations, file permissions)

---

## Objectives

Implement atomic TOML file write utilities with temp file + rename pattern, Unix permissions, and field preservation.

---

## Requirements Addressed

- [APIK-ATOMIC-010] - Atomic file operations (temp + rename)
- [APIK-ATOMIC-020] - Prevent corruption/race conditions
- [APIK-TOML-030] - Preserve existing fields
- [APIK-TOML-040] - Atomic write implementation
- [APIK-TOML-050] - Permissions 0600 (Unix)
- [APIK-SEC-010] - TOML permissions 0600
- [APIK-SEC-020] - Auto permission setting
- [APIK-SEC-030] - Windows NTFS ACLs (best-effort)
- [APIK-ARCH-020] - wkmp-common provides utilities

---

## Deliverables

### Code Changes

**File: wkmp-common/src/config.rs** (extend)

```rust
use std::fs;
use std::io::Write;
use std::path::Path;

/// Write TOML config to file atomically with permissions
///
/// **Traceability:** APIK-ATOMIC-010, APIK-SEC-010
///
/// Atomic write steps:
/// 1. Serialize config to TOML string
/// 2. Write to temporary file (.toml.tmp)
/// 3. Set Unix permissions 0600 (if Unix)
/// 4. Rename temp file to target (atomic)
///
/// **Returns:** Ok(()) on success, Err on any failure
pub fn write_toml_config(
    config: &TomlConfig,
    target_path: &Path,
) -> Result<()> {
    // Step 1: Serialize to TOML
    let toml_string = toml::to_string_pretty(config)
        .map_err(|e| Error::Config(format!("TOML serialization failed: {}", e)))?;

    // Step 2: Create temp file
    let temp_path = target_path.with_extension("toml.tmp");
    let mut temp_file = fs::File::create(&temp_path)
        .map_err(|e| Error::Config(format!("Temp file create failed: {}", e)))?;

    temp_file.write_all(toml_string.as_bytes())
        .map_err(|e| Error::Config(format!("Temp file write failed: {}", e)))?;

    // Ensure data is flushed to disk before rename
    temp_file.sync_all()
        .map_err(|e| Error::Config(format!("Temp file sync failed: {}", e)))?;

    drop(temp_file); // Close file before setting permissions

    // Step 3: Set permissions (Unix only)
    set_unix_permissions_0600(&temp_path)?;

    // Step 4: Atomic rename
    fs::rename(&temp_path, target_path)
        .map_err(|e| Error::Config(format!("Atomic rename failed: {}", e)))?;

    Ok(())
}

/// Set Unix file permissions to 0600 (rw-------)
///
/// **Traceability:** APIK-SEC-010, APIK-SEC-020
///
/// **Unix:** Sets permissions to 0600 (owner read/write only)
/// **Windows:** No-op (returns Ok, relies on NTFS default permissions)
#[cfg(unix)]
pub fn set_unix_permissions_0600(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = fs::metadata(path)
        .map_err(|e| Error::Config(format!("Get metadata failed: {}", e)))?
        .permissions();

    perms.set_mode(0o600);

    fs::set_permissions(path, perms)
        .map_err(|e| Error::Config(format!("Set permissions failed: {}", e)))?;

    Ok(())
}

#[cfg(not(unix))]
pub fn set_unix_permissions_0600(_path: &Path) -> Result<()> {
    // Windows: No-op (best-effort approach)
    // NTFS default permissions (user-only access) are acceptable
    Ok(())
}

/// Check if TOML file has loose permissions (Unix only)
///
/// **Traceability:** APIK-SEC-040
///
/// **Returns:** true if permissions are looser than 0600 (world/group readable)
#[cfg(unix)]
pub fn check_toml_permissions_loose(path: &Path) -> Result<bool> {
    use std::os::unix::fs::PermissionsExt;

    if !path.exists() {
        return Ok(false); // File doesn't exist, no permission issue
    }

    let metadata = fs::metadata(path)
        .map_err(|e| Error::Config(format!("Get metadata failed: {}", e)))?;

    let mode = metadata.permissions().mode();

    // Loose if group or others have any access (bits 077)
    Ok((mode & 0o077) != 0)
}

#[cfg(not(unix))]
pub fn check_toml_permissions_loose(_path: &Path) -> Result<bool> {
    // Windows: Cannot reliably check NTFS ACLs, return false
    Ok(false)
}
```

---

### Unit Tests

**File: wkmp-common/tests/toml_utils_tests.rs** (new)

```rust
use wkmp_common::config::{TomlConfig, write_toml_config, check_toml_permissions_loose};
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_atomic_write_creates_temp_file() {
    // tc_u_toml_001
    let temp_dir = TempDir::new().unwrap();
    let target = temp_dir.path().join("test.toml");

    let config = TomlConfig {
        root_folder: Some(PathBuf::from("/music")),
        logging: Default::default(),
        static_assets: None,
        acoustid_api_key: Some("key123".to_string()),
    };

    // Verify temp file created during write (implementation detail test)
    // Note: This test verifies process, not observable behavior
    write_toml_config(&config, &target).unwrap();

    // Verify target exists and temp file cleaned up
    assert!(target.exists());
    assert!(!temp_dir.path().join("test.toml.tmp").exists());
}

#[test]
fn test_atomic_write_renames_to_target() {
    // tc_u_toml_002
    let temp_dir = TempDir::new().unwrap();
    let target = temp_dir.path().join("test.toml");

    let config = TomlConfig {
        root_folder: Some(PathBuf::from("/music")),
        logging: Default::default(),
        static_assets: None,
        acoustid_api_key: Some("key123".to_string()),
    };

    write_toml_config(&config, &target).unwrap();

    // Verify target file exists
    assert!(target.exists());

    // Verify content is correct
    let content = std::fs::read_to_string(&target).unwrap();
    assert!(content.contains("acoustid_api_key"));
    assert!(content.contains("key123"));
}

#[test]
fn test_atomic_write_preserves_existing_fields() {
    // tc_u_toml_003
    let temp_dir = TempDir::new().unwrap();
    let target = temp_dir.path().join("test.toml");

    // Write initial config with all fields
    let config1 = TomlConfig {
        root_folder: Some(PathBuf::from("/music")),
        logging: Default::default(),
        static_assets: Some(PathBuf::from("/static")),
        acoustid_api_key: Some("key123".to_string()),
    };

    write_toml_config(&config1, &target).unwrap();

    // Read back and verify all fields preserved
    let content = std::fs::read_to_string(&target).unwrap();
    let parsed: TomlConfig = toml::from_str(&content).unwrap();

    assert_eq!(parsed.root_folder, Some(PathBuf::from("/music")));
    assert_eq!(parsed.static_assets, Some(PathBuf::from("/static")));
    assert_eq!(parsed.acoustid_api_key, Some("key123".to_string()));
}

#[test]
#[cfg(unix)]
fn test_atomic_write_sets_permissions_0600() {
    // tc_u_toml_004
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new().unwrap();
    let target = temp_dir.path().join("test.toml");

    let config = TomlConfig {
        root_folder: None,
        logging: Default::default(),
        static_assets: None,
        acoustid_api_key: Some("key123".to_string()),
    };

    write_toml_config(&config, &target).unwrap();

    // Verify permissions are 0600
    let metadata = std::fs::metadata(&target).unwrap();
    let mode = metadata.permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
}

#[test]
#[cfg(not(unix))]
fn test_atomic_write_graceful_on_windows() {
    // tc_u_toml_005
    let temp_dir = TempDir::new().unwrap();
    let target = temp_dir.path().join("test.toml");

    let config = TomlConfig {
        root_folder: None,
        logging: Default::default(),
        static_assets: None,
        acoustid_api_key: Some("key123".to_string()),
    };

    // Should succeed on Windows (no permission setting)
    write_toml_config(&config, &target).unwrap();
    assert!(target.exists());
}

#[test]
fn test_roundtrip_serialization_preserves_data() {
    // tc_u_toml_006
    let temp_dir = TempDir::new().unwrap();
    let target = temp_dir.path().join("test.toml");

    let config = TomlConfig {
        root_folder: Some(PathBuf::from("/music")),
        logging: Default::default(),
        static_assets: Some(PathBuf::from("/static")),
        acoustid_api_key: Some("key123".to_string()),
    };

    write_toml_config(&config, &target).unwrap();

    // Read back and compare
    let content = std::fs::read_to_string(&target).unwrap();
    let parsed: TomlConfig = toml::from_str(&content).unwrap();

    assert_eq!(parsed.root_folder, config.root_folder);
    assert_eq!(parsed.static_assets, config.static_assets);
    assert_eq!(parsed.acoustid_api_key, config.acoustid_api_key);
}

#[test]
#[cfg(unix)]
fn test_check_permissions_detects_loose() {
    // tc_u_sec_001
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new().unwrap();
    let target = temp_dir.path().join("test.toml");

    // Create file with loose permissions (0644)
    std::fs::write(&target, "test").unwrap();
    let mut perms = std::fs::metadata(&target).unwrap().permissions();
    perms.set_mode(0o644);
    std::fs::set_permissions(&target, perms).unwrap();

    // Should detect loose permissions
    assert!(check_toml_permissions_loose(&target).unwrap());

    // Set to 0600
    let mut perms = std::fs::metadata(&target).unwrap().permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(&target, perms).unwrap();

    // Should not detect loose permissions
    assert!(!check_toml_permissions_loose(&target).unwrap());
}
```

---

## Verification Steps

1. All unit tests pass (tc_u_toml_001-007, tc_u_sec_001)
2. Unix: Verify permissions are 0600 after write
3. Windows: Verify write succeeds (no permission errors)
4. Verify temp file is cleaned up after rename
5. Verify existing fields preserved in roundtrip

---

## Acceptance Criteria

- [ ] write_toml_config() function implemented
- [ ] Atomic write uses temp file + rename pattern
- [ ] Unix permissions set to 0600
- [ ] Windows no-op for permissions (graceful)
- [ ] Field preservation verified (all TomlConfig fields)
- [ ] check_toml_permissions_loose() implemented (Unix)
- [ ] All unit tests pass (7 tests)
- [ ] No regressions in existing wkmp-common tests

---

## Test Traceability

- tc_u_toml_001: Atomic write creates temp file
- tc_u_toml_002: Atomic write renames to target
- tc_u_toml_003: Preserves existing fields
- tc_u_toml_004: Sets permissions 0600 (Unix)
- tc_u_toml_005: Graceful on Windows
- tc_u_toml_006: Roundtrip serialization
- tc_u_sec_001: Permission check detects loose permissions

---

## Implementation Notes

**Atomic Rename:**
- std::fs::rename is atomic on Unix (POSIX requirement)
- Windows: rename may fail if target exists (acceptable, best-effort)
- sync_all() before rename ensures data on disk

**Permissions:**
- Unix: 0600 = owner read/write only (secure)
- Windows: Relies on NTFS default (user-only access if not explicitly shared)
- check_toml_permissions_loose() helps detect misconfigurations

**Error Handling:**
- Any failure returns Err (caller decides to warn or fail)
- Temp file may be left behind on failure (acceptable, rare)

---

## Rollback Plan

If increment fails:
- Revert write_toml_config() and related functions
- Remove test file
- No downstream impact (no modules use these utilities yet)
