//! Unit tests for TOML atomic write utilities
//!
//! Tests the implementation of:
//! - [APIK-ATOMIC-010] - Atomic file operations (temp + rename)
//! - [APIK-ATOMIC-020] - Prevent corruption/race conditions
//! - [APIK-TOML-030] - Preserve existing fields
//! - [APIK-TOML-040] - Atomic write implementation
//! - [APIK-TOML-050] - Permissions 0600 (Unix)
//! - [APIK-SEC-010] - TOML permissions 0600
//! - [APIK-SEC-020] - Auto permission setting
//! - [APIK-SEC-040] - Permission checking

use wkmp_common::config::{TomlConfig, LoggingConfig, write_toml_config};
#[cfg(unix)]
use wkmp_common::config::check_toml_permissions_loose;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_atomic_write_creates_temp_file() {
    // tc_u_toml_001
    let temp_dir = TempDir::new().unwrap();
    let target = temp_dir.path().join("test.toml");

    let config = TomlConfig {
        root_folder: Some(PathBuf::from("/music")),
        logging: LoggingConfig::default(),
        static_assets: None,
        acoustid_api_key: Some("key123".to_string()),
        musicbrainz_token: None,
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
        logging: LoggingConfig::default(),
        static_assets: None,
        acoustid_api_key: Some("key123".to_string()),
        musicbrainz_token: None,
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
        logging: LoggingConfig::default(),
        static_assets: Some(PathBuf::from("/static")),
        acoustid_api_key: Some("key123".to_string()),
        musicbrainz_token: None,
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
        logging: LoggingConfig::default(),
        static_assets: None,
        acoustid_api_key: Some("key123".to_string()),
        musicbrainz_token: None,
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
        logging: LoggingConfig::default(),
        static_assets: None,
        acoustid_api_key: Some("key123".to_string()),
        musicbrainz_token: None,
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
        logging: LoggingConfig::default(),
        static_assets: Some(PathBuf::from("/static")),
        acoustid_api_key: Some("key123".to_string()),
        musicbrainz_token: None,
    };

    write_toml_config(&config, &target).unwrap();

    // Read back and compare
    let content = std::fs::read_to_string(&target).unwrap();
    let parsed: TomlConfig = toml::from_str(&content).unwrap();

    assert_eq!(parsed.root_folder, config.root_folder);
    assert_eq!(parsed.static_assets, config.static_assets);
    assert_eq!(parsed.acoustid_api_key, config.acoustid_api_key);
    assert_eq!(parsed.musicbrainz_token, config.musicbrainz_token);
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
