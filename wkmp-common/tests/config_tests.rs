//! Unit tests for configuration and graceful degradation
//!
//! Tests the implementation of:
//! - [REQ-NF-031]: Missing TOML files SHALL NOT cause termination
//! - [REQ-NF-032]: Missing configs → warning + defaults + startup
//! - [REQ-NF-033]: Default root folder locations per platform
//! - [REQ-NF-035]: Priority order for root folder resolution
//! - [REQ-NF-036]: Automatic directory/database creation
//! - [APIK-TOML-SCHEMA-010]: TomlConfig acoustid_api_key field
//! - [APIK-TOML-SCHEMA-020]: Backward compatibility
//!
//! Note: Uses serial_test crate to prevent ENV variable race conditions.
//! Tests that manipulate WKMP_ROOT_FOLDER or WKMP_ROOT are marked with #[serial]
//! to ensure they run sequentially, not in parallel.

use wkmp_common::config::{CompiledDefaults, RootFolderResolver, RootFolderInitializer, TomlConfig, LoggingConfig};
use std::path::PathBuf;
use std::env;
use serial_test::serial;

#[test]
fn test_compiled_defaults_for_current_platform() {
    // [REQ-NF-033]: Default root folder locations per platform
    let defaults = CompiledDefaults::for_current_platform();

    // Verify non-empty paths
    assert!(!defaults.root_folder.as_os_str().is_empty());
    assert_eq!(defaults.log_level, "info");
    assert!(defaults.log_file.is_none());
    assert!(!defaults.static_assets_path.as_os_str().is_empty());

    // Platform-specific verification
    #[cfg(target_os = "linux")]
    {
        // Should contain "Music" directory
        let path_str = defaults.root_folder.to_string_lossy();
        assert!(path_str.contains("Music"), "Linux default should be ~/Music");
    }

    #[cfg(target_os = "macos")]
    {
        let path_str = defaults.root_folder.to_string_lossy();
        assert!(path_str.contains("Music"), "macOS default should be ~/Music");
    }

    #[cfg(target_os = "windows")]
    {
        let path_str = defaults.root_folder.to_string_lossy();
        assert!(path_str.contains("Music"),
                "Windows default should be %USERPROFILE%\\Music");
    }
}

#[test]
#[serial]
fn test_resolver_with_no_overrides_uses_default() {
    // [REQ-NF-032]: Use compiled defaults when no config available

    // Clear environment variables
    env::remove_var("WKMP_ROOT_FOLDER");
    env::remove_var("WKMP_ROOT");

    let resolver = RootFolderResolver::new("test-module");
    let root_folder = resolver.resolve();

    // Should return a valid path (the compiled default)
    assert!(!root_folder.as_os_str().is_empty());

    // Should match compiled default
    let defaults = CompiledDefaults::for_current_platform();
    assert_eq!(root_folder, defaults.root_folder);
}

#[test]
#[serial]
fn test_resolver_env_var_wkmp_root_folder() {
    // [REQ-NF-035]: Environment variable priority
    let test_path = "/tmp/wkmp-test-env-folder";
    env::set_var("WKMP_ROOT_FOLDER", test_path);

    let resolver = RootFolderResolver::new("test-module");
    let root_folder = resolver.resolve();

    assert_eq!(root_folder, PathBuf::from(test_path));

    // Cleanup
    env::remove_var("WKMP_ROOT_FOLDER");
}

#[test]
#[serial]
fn test_resolver_env_var_wkmp_root() {
    // [REQ-NF-035]: Alternative environment variable
    let test_path = "/tmp/wkmp-test-env-root";
    env::set_var("WKMP_ROOT", test_path);

    let resolver = RootFolderResolver::new("test-module");
    let root_folder = resolver.resolve();

    assert_eq!(root_folder, PathBuf::from(test_path));

    // Cleanup
    env::remove_var("WKMP_ROOT");
}

#[test]
#[serial]
fn test_resolver_wkmp_root_folder_takes_precedence() {
    // [REQ-NF-035]: WKMP_ROOT_FOLDER has priority over WKMP_ROOT

    // Clean up first to ensure no interference
    env::remove_var("WKMP_ROOT_FOLDER");
    env::remove_var("WKMP_ROOT");

    env::set_var("WKMP_ROOT_FOLDER", "/tmp/wkmp-priority-1");
    env::set_var("WKMP_ROOT", "/tmp/wkmp-priority-2");

    let resolver = RootFolderResolver::new("test-module");
    let root_folder = resolver.resolve();

    assert_eq!(root_folder, PathBuf::from("/tmp/wkmp-priority-1"));

    // Cleanup
    env::remove_var("WKMP_ROOT_FOLDER");
    env::remove_var("WKMP_ROOT");
}

#[test]
fn test_initializer_database_path() {
    let root = PathBuf::from("/tmp/wkmp-test-root");
    let initializer = RootFolderInitializer::new(root.clone());

    let db_path = initializer.database_path();
    assert_eq!(db_path, root.join("wkmp.db"));
}

#[test]
fn test_initializer_database_exists() {
    let root = PathBuf::from("/tmp/wkmp-test-nonexistent");
    let initializer = RootFolderInitializer::new(root);

    // Should return false for non-existent database
    assert!(!initializer.database_exists());
}

#[test]
fn test_initializer_creates_directory() {
    // [REQ-NF-036]: Automatic directory creation
    let test_dir = format!("/tmp/wkmp-test-create-{}", std::process::id());
    let root = PathBuf::from(&test_dir);

    // Ensure directory doesn't exist
    let _ = std::fs::remove_dir_all(&root);

    let initializer = RootFolderInitializer::new(root.clone());
    let result = initializer.ensure_directory_exists();

    assert!(result.is_ok(), "Failed to create directory: {:?}", result.err());
    assert!(root.exists(), "Directory was not created");
    assert!(root.is_dir(), "Created path is not a directory");

    // Cleanup
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn test_initializer_idempotent_directory_creation() {
    // [REQ-NF-036]: Safe to call multiple times
    let test_dir = format!("/tmp/wkmp-test-idempotent-{}", std::process::id());
    let root = PathBuf::from(&test_dir);

    // Ensure directory doesn't exist
    let _ = std::fs::remove_dir_all(&root);

    let initializer = RootFolderInitializer::new(root.clone());

    // First call - should create
    let result1 = initializer.ensure_directory_exists();
    assert!(result1.is_ok());

    // Second call - should succeed (idempotent)
    let result2 = initializer.ensure_directory_exists();
    assert!(result2.is_ok());

    assert!(root.exists());

    // Cleanup
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
#[serial]
fn test_resolver_missing_config_file_does_not_error() {
    // [REQ-NF-031]: Missing TOML files SHALL NOT cause termination

    // Clear environment to force config file lookup
    env::remove_var("WKMP_ROOT_FOLDER");
    env::remove_var("WKMP_ROOT");

    // Use a module name that definitely won't have a config file
    let resolver = RootFolderResolver::new("nonexistent-test-module-12345");

    // Should not panic - should return compiled default
    let root_folder = resolver.resolve();

    assert!(!root_folder.as_os_str().is_empty());

    // Should match compiled default
    let defaults = CompiledDefaults::for_current_platform();
    assert_eq!(root_folder, defaults.root_folder);
}

#[test]
fn test_module_name_in_config_path() {
    let resolver = RootFolderResolver::new("test-module");

    // This is a bit of a whitebox test - we can't directly access config_file_path()
    // But we can verify the module name is used by checking the logic would work

    // The config file path should be constructed with module name
    #[cfg(target_os = "linux")]
    {
        // Expected: ~/.config/wkmp/test-module.toml
        // We can't test this directly without accessing private methods
        // This test serves as documentation of expected behavior
    }
}

#[test]
fn test_compiled_defaults_linux() {
    // Platform-specific test - only runs on Linux
    #[cfg(target_os = "linux")]
    {
        use wkmp_common::config::CompiledDefaults;

        let defaults = CompiledDefaults::for_current_platform();

        let home = env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
        let expected = PathBuf::from(home).join("Music");

        assert_eq!(defaults.root_folder, expected);
        assert_eq!(defaults.log_level, "info");
        assert_eq!(defaults.log_file, None);
        assert_eq!(defaults.static_assets_path, PathBuf::from("/usr/local/share/wkmp"));
    }
}

#[test]
fn test_compiled_defaults_macos() {
    // Platform-specific test - only runs on macOS
    #[cfg(target_os = "macos")]
    {
        use wkmp_common::config::CompiledDefaults;

        let defaults = CompiledDefaults::for_current_platform();

        let home = env::var("HOME").unwrap_or_else(|_| "/Users/user".to_string());
        let expected = PathBuf::from(home).join("Music");

        assert_eq!(defaults.root_folder, expected);
        assert_eq!(defaults.log_level, "info");
        assert_eq!(defaults.log_file, None);
        assert_eq!(defaults.static_assets_path, PathBuf::from("/Applications/WKMP.app/Contents/Resources"));
    }
}

#[test]
fn test_compiled_defaults_windows() {
    // Platform-specific test - only runs on Windows
    #[cfg(target_os = "windows")]
    {
        use wkmp_common::config::CompiledDefaults;

        let defaults = CompiledDefaults::for_current_platform();

        let userprofile = env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\user".to_string());
        let expected = PathBuf::from(userprofile).join("Music");

        assert_eq!(defaults.root_folder, expected);
        assert_eq!(defaults.log_level, "info");
        assert_eq!(defaults.log_file, None);
        assert_eq!(defaults.static_assets_path, PathBuf::from("C:\\Program Files\\WKMP\\share"));
    }
}

#[test]
#[serial]
fn test_graceful_degradation_end_to_end() {
    // [REQ-NF-031, REQ-NF-032, REQ-NF-036]: Complete graceful degradation flow

    // Clear environment
    env::remove_var("WKMP_ROOT_FOLDER");
    env::remove_var("WKMP_ROOT");

    // Step 1: Resolve root folder (should use default, no error)
    let resolver = RootFolderResolver::new("test-graceful-degradation");
    let root_folder = resolver.resolve();

    assert!(!root_folder.as_os_str().is_empty());

    // For testing, use a temp directory instead
    let test_root = PathBuf::from(format!("/tmp/wkmp-graceful-test-{}", std::process::id()));

    // Step 2: Create directory (should succeed even if doesn't exist)
    let initializer = RootFolderInitializer::new(test_root.clone());
    let result = initializer.ensure_directory_exists();

    assert!(result.is_ok(), "Directory creation failed: {:?}", result.err());
    assert!(test_root.exists());

    // Step 3: Database path should be constructable
    let db_path = initializer.database_path();
    assert_eq!(db_path, test_root.join("wkmp.db"));

    // Cleanup
    let _ = std::fs::remove_dir_all(&test_root);
}

#[test]
fn test_initializer_nested_directory_creation() {
    // [REQ-NF-036]: Should create nested directories
    let test_dir = format!("/tmp/wkmp-test-nested-{}/level1/level2", std::process::id());
    let root = PathBuf::from(&test_dir);

    // Ensure directory doesn't exist
    let _ = std::fs::remove_dir_all(PathBuf::from(format!("/tmp/wkmp-test-nested-{}", std::process::id())));

    let initializer = RootFolderInitializer::new(root.clone());
    let result = initializer.ensure_directory_exists();

    assert!(result.is_ok(), "Failed to create nested directories: {:?}", result.err());
    assert!(root.exists(), "Nested directory was not created");
    assert!(root.is_dir(), "Created nested path is not a directory");

    // Cleanup
    let _ = std::fs::remove_dir_all(PathBuf::from(format!("/tmp/wkmp-test-nested-{}", std::process::id())));
}

#[test]
fn test_toml_roundtrip_with_acoustid_key() {
    // [APIK-TOML-SCHEMA-010]: Verify acoustid_api_key field serialization/deserialization
    let config = TomlConfig {
        root_folder: Some(PathBuf::from("/music")),
        logging: LoggingConfig::default(),
        static_assets: None,
        acoustid_api_key: Some("test-key-123".to_string()),
    };

    let toml_str = toml::to_string(&config).unwrap();
    let parsed: TomlConfig = toml::from_str(&toml_str).unwrap();

    assert_eq!(parsed.acoustid_api_key, Some("test-key-123".to_string()));
    assert_eq!(parsed.root_folder, Some(PathBuf::from("/music")));
}

#[test]
fn test_backward_compatible_missing_field() {
    // [APIK-TOML-SCHEMA-020]: Missing acoustid_api_key field deserializes as None
    let toml_str = r#"
        root_folder = "/music"
        [logging]
        level = "info"
    "#;

    let config: TomlConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.acoustid_api_key, None);
    assert_eq!(config.root_folder, Some(PathBuf::from("/music")));
}
