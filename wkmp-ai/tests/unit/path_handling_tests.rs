//! TC-PATH-001: Path Handling Correctness Tests
//!
//! Verifies relative paths stored in database, absolute paths used for audio operations

use std::path::PathBuf;
use tempfile::TempDir;

/// TC-PATH-001: Relative paths stored, absolute paths used for decoding
///
/// **Requirement:** Database stores relative paths, audio operations use absolute
///
/// **Given:** File record with relative path "Artist/Album/Track.mp3"
/// **When:** File path retrieved for processing
/// **Then:**
///   - Database stores relative path
///   - Absolute path constructed by joining with root folder
///   - Audio decoder receives absolute path
///
/// **Verification Method:**
///   - Create file with known relative path
///   - Verify database contains relative path
///   - Verify path joining produces correct absolute path
#[tokio::test]
async fn tc_path_001_relative_to_absolute_conversion() {
    use wkmp_ai::db::files::AudioFile;

    // Setup: Create test database
    let (_temp_db_dir, db_pool) = crate::helpers::db_utils::create_test_db()
        .await
        .unwrap();

    // Setup: Define root folder and relative path
    let root_folder = PathBuf::from("/music/library");
    let relative_path = "Artist/Album/Track.mp3";

    // Create file record with relative path
    let file = AudioFile::new(
        relative_path.to_string(),
        "abc123".to_string(),
        chrono::Utc::now(),
    );

    let guid = wkmp_ai::db::files::save_file(&db_pool, &file)
        .await
        .unwrap();

    // Verify: Database contains relative path (NOT absolute)
    let stored_path: String = sqlx::query_scalar("SELECT path FROM files WHERE guid = ?")
        .bind(&guid)
        .fetch_one(&db_pool)
        .await
        .unwrap();

    assert_eq!(
        stored_path, relative_path,
        "Database should store relative path"
    );
    assert!(
        !stored_path.starts_with('/') && !stored_path.starts_with("C:"),
        "Stored path should not be absolute"
    );

    // Verify: Absolute path construction
    let absolute_path = root_folder.join(&stored_path);

    assert!(
        absolute_path.is_absolute(),
        "Joined path should be absolute"
    );
    assert_eq!(
        absolute_path.to_string_lossy(),
        "/music/library/Artist/Album/Track.mp3",
        "Absolute path should be correctly constructed"
    );

    println!("✅ TC-PATH-001 PASS: Relative → absolute path conversion correct");
}

/// TC-PATH-002: Path handling with Windows drive letters
///
/// **Requirement:** Cross-platform path handling
///
/// **Given:** Windows-style root folder "C:\\Music"
/// **When:** Relative path joined with root
/// **Then:**
///   - Produces correct Windows absolute path
///   - No path corruption or truncation
#[tokio::test]
#[cfg(target_os = "windows")]
async fn tc_path_002_windows_path_handling() {
    use std::path::Path;

    let root_folder = Path::new("C:\\Music");
    let relative_path = "Artist\\Album\\Track.mp3";

    let absolute_path = root_folder.join(relative_path);

    assert!(absolute_path.is_absolute());
    assert_eq!(
        absolute_path.to_string_lossy(),
        "C:\\Music\\Artist\\Album\\Track.mp3"
    );

    println!("✅ TC-PATH-002 PASS: Windows path handling correct");
}

/// TC-PATH-003: Path handling with Unix-style paths
///
/// **Requirement:** Cross-platform path handling
///
/// **Given:** Unix-style root folder "/home/user/Music"
/// **When:** Relative path joined with root
/// **Then:**
///   - Produces correct Unix absolute path
///   - No path corruption or truncation
#[tokio::test]
#[cfg(target_family = "unix")]
async fn tc_path_003_unix_path_handling() {
    use std::path::Path;

    let root_folder = Path::new("/home/user/Music");
    let relative_path = "Artist/Album/Track.mp3";

    let absolute_path = root_folder.join(relative_path);

    assert!(absolute_path.is_absolute());
    assert_eq!(
        absolute_path.to_string_lossy(),
        "/home/user/Music/Artist/Album/Track.mp3"
    );

    println!("✅ TC-PATH-003 PASS: Unix path handling correct");
}

/// TC-PATH-004: Path stripping (absolute → relative)
///
/// **Requirement:** SCANNING phase path normalization
///
/// **Given:** Absolute file path within root folder
/// **When:** Converted to relative path for database storage
/// **Then:**
///   - Produces correct relative path
///   - Removes root folder prefix
#[test]
fn tc_path_004_absolute_to_relative_stripping() {
    use std::path::Path;

    let root_folder = Path::new("/music/library");
    let absolute_path = Path::new("/music/library/Artist/Album/Track.mp3");

    let relative_path = absolute_path
        .strip_prefix(root_folder)
        .unwrap()
        .to_string_lossy()
        .to_string();

    assert_eq!(relative_path, "Artist/Album/Track.mp3");
    assert!(!relative_path.starts_with('/'));

    println!("✅ TC-PATH-004 PASS: Absolute → relative path stripping correct");
}

/// TC-PATH-005: Path with special characters
///
/// **Requirement:** Robust path handling
///
/// **Given:** File path with spaces and special characters
/// **When:** Stored and retrieved from database
/// **Then:**
///   - No path corruption
///   - Special characters preserved
#[tokio::test]
async fn tc_path_005_special_characters_in_path() {
    use wkmp_ai::db::files::AudioFile;

    // Setup: Create test database
    let (_temp_db_dir, db_pool) = crate::helpers::db_utils::create_test_db()
        .await
        .unwrap();

    // Path with spaces, parentheses, apostrophes
    let relative_path = "Artist Name/Album (2024)/Track's Title.mp3";

    // Create file record
    let file = AudioFile::new(
        relative_path.to_string(),
        "hash123".to_string(),
        chrono::Utc::now(),
    );

    let guid = wkmp_ai::db::files::save_file(&db_pool, &file)
        .await
        .unwrap();

    // Verify: Path stored correctly
    let stored_path: String = sqlx::query_scalar("SELECT path FROM files WHERE guid = ?")
        .bind(&guid)
        .fetch_one(&db_pool)
        .await
        .unwrap();

    assert_eq!(
        stored_path, relative_path,
        "Path with special characters should be preserved"
    );

    println!("✅ TC-PATH-005 PASS: Special characters in path preserved");
}

/// TC-PATH-006: Long path handling
///
/// **Requirement:** Path length limits (Windows MAX_PATH = 260)
///
/// **Given:** Very long file path (>200 characters)
/// **When:** Stored in database
/// **Then:**
///   - Path stored without truncation
///   - Can be retrieved correctly
#[tokio::test]
async fn tc_path_006_long_path_handling() {
    use wkmp_ai::db::files::AudioFile;

    // Setup: Create test database
    let (_temp_db_dir, db_pool) = crate::helpers::db_utils::create_test_db()
        .await
        .unwrap();

    // Create very long path (>200 characters)
    let long_path = format!(
        "Very/Long/Directory/Structure/{}/Track.mp3",
        "SubFolder/".repeat(20)
    );

    assert!(
        long_path.len() > 200,
        "Test path should be >200 characters"
    );

    // Create file record
    let file = AudioFile::new(long_path.clone(), "hash456".to_string(), chrono::Utc::now());

    let guid = wkmp_ai::db::files::save_file(&db_pool, &file)
        .await
        .unwrap();

    // Verify: Full path stored without truncation
    let stored_path: String = sqlx::query_scalar("SELECT path FROM files WHERE guid = ?")
        .bind(&guid)
        .fetch_one(&db_pool)
        .await
        .unwrap();

    assert_eq!(
        stored_path.len(),
        long_path.len(),
        "Long path should not be truncated"
    );
    assert_eq!(stored_path, long_path, "Long path should match exactly");

    println!("✅ TC-PATH-006 PASS: Long path handled without truncation");
}
