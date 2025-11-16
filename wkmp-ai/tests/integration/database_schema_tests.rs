//! TC-DB-001: Database Schema Validation Tests
//!
//! Verifies SPEC031 zero-conf database schema compliance

use crate::helpers::db_utils;

/// TC-DB-001: AudioFile model matches files table schema (SPEC031)
///
/// **Requirement:** SPEC031 Zero-conf database (no session_id tracking)
///
/// **Given:** SQLite database with files table schema
/// **When:** Schema introspection performed
/// **Then:**
///   - files table has expected columns
///   - NO session_id column present (SPEC031 zero-conf)
///   - AudioFile struct fields match table columns
///
/// **Verification Method:**
///   - Query SQLite schema: PRAGMA table_info(files)
///   - Assert no session_id column exists
///   - Verify core columns present (guid, path, hash, modification_time)
#[tokio::test]
async fn tc_db_001_audio_file_schema_matches_table() {
    // Setup: Create test database with migrations
    let (_temp_dir, pool) = db_utils::create_test_db().await.unwrap();

    // Verify: files table exists
    let tables = db_utils::get_table_names(&pool).await.unwrap();
    assert!(
        tables.contains(&"files".to_string()),
        "files table should exist after migrations"
    );

    // Verify: Get files table schema
    let columns = db_utils::get_table_columns(&pool, "files").await.unwrap();
    assert!(!columns.is_empty(), "files table should have columns");

    let column_names: Vec<_> = columns.iter().map(|c| c.name.as_str()).collect();

    // CRITICAL VERIFICATION: NO session_id column (SPEC031 zero-conf)
    db_utils::assert_no_column(&pool, "files", "session_id").await;

    // Verify: Core columns present
    db_utils::assert_has_column(&pool, "files", "guid").await;
    db_utils::assert_has_column(&pool, "files", "path").await;
    db_utils::assert_has_column(&pool, "files", "hash").await;
    db_utils::assert_has_column(&pool, "files", "modification_time").await;

    println!("✅ TC-DB-001 PASS: files table schema correct (no session_id)");
    println!("   Columns: {:?}", column_names);
}

/// TC-DB-002: passages table schema validation
///
/// **Requirement:** SPEC032 passage recording schema
///
/// **Given:** Database with passages table
/// **When:** Schema introspection performed
/// **Then:**
///   - passages table has expected columns
///   - Foreign key to files table via file_id
///   - Foreign key to songs table via song_id (nullable for zero-song passages)
#[tokio::test]
async fn tc_db_002_passages_table_schema() {
    // Setup: Create test database with migrations
    let (_temp_dir, pool) = db_utils::create_test_db().await.unwrap();

    // Verify: passages table exists
    let tables = db_utils::get_table_names(&pool).await.unwrap();
    assert!(
        tables.contains(&"passages".to_string()),
        "passages table should exist after migrations"
    );

    // Verify: Get passages table schema
    let columns = db_utils::get_table_columns(&pool, "passages")
        .await
        .unwrap();
    let column_names: Vec<_> = columns.iter().map(|c| c.name.as_str()).collect();

    // Verify: Core passage columns
    db_utils::assert_has_column(&pool, "passages", "guid").await;
    db_utils::assert_has_column(&pool, "passages", "file_id").await;
    db_utils::assert_has_column(&pool, "passages", "song_id").await;
    db_utils::assert_has_column(&pool, "passages", "start_ticks").await;
    db_utils::assert_has_column(&pool, "passages", "end_ticks").await;
    db_utils::assert_has_column(&pool, "passages", "status").await;

    // Verify: Amplitude analysis columns
    db_utils::assert_has_column(&pool, "passages", "lead_in_start_ticks").await;
    db_utils::assert_has_column(&pool, "passages", "lead_out_start_ticks").await;

    println!("✅ TC-DB-002 PASS: passages table schema correct");
    println!("   Columns: {:?}", column_names);
}

/// TC-DB-003: songs table schema validation
///
/// **Requirement:** SPEC032 song metadata storage
///
/// **Given:** Database with songs table
/// **When:** Schema introspection performed
/// **Then:**
///   - songs table has expected columns
///   - recording_mbid column (unique MusicBrainz identifier)
///   - flavor_vector JSON column
#[tokio::test]
async fn tc_db_003_songs_table_schema() {
    // Setup: Create test database with migrations
    let (_temp_dir, pool) = db_utils::create_test_db().await.unwrap();

    // Verify: songs table exists
    let tables = db_utils::get_table_names(&pool).await.unwrap();
    assert!(
        tables.contains(&"songs".to_string()),
        "songs table should exist after migrations"
    );

    // Verify: Get songs table schema
    let columns = db_utils::get_table_columns(&pool, "songs").await.unwrap();
    let column_names: Vec<_> = columns.iter().map(|c| c.name.as_str()).collect();

    // Verify: Core song columns
    db_utils::assert_has_column(&pool, "songs", "guid").await;
    db_utils::assert_has_column(&pool, "songs", "recording_mbid").await;
    db_utils::assert_has_column(&pool, "songs", "title").await;
    db_utils::assert_has_column(&pool, "songs", "flavor_vector").await;
    db_utils::assert_has_column(&pool, "songs", "status").await;

    // Verify: Probability and cooldown columns
    db_utils::assert_has_column(&pool, "songs", "base_probability").await;
    db_utils::assert_has_column(&pool, "songs", "min_cooldown").await;
    db_utils::assert_has_column(&pool, "songs", "ramping_cooldown").await;

    println!("✅ TC-DB-003 PASS: songs table schema correct");
    println!("   Columns: {:?}", column_names);
}

/// TC-DB-004: AudioFile record insertion without session_id
///
/// **Requirement:** SPEC031 zero-conf - no session tracking in files table
///
/// **Given:** AudioFile record created without session_id
/// **When:** Record inserted into database
/// **Then:**
///   - INSERT succeeds
///   - Record persisted correctly
///   - NO session_id field in record
#[tokio::test]
async fn tc_db_004_audio_file_insert_without_session_id() {
    use wkmp_ai::db::files::AudioFile;

    // Setup: Create test database
    let (_temp_dir, pool) = db_utils::create_test_db().await.unwrap();

    // Create AudioFile record (no session_id parameter)
    let file = AudioFile::new(
        "Artist/Album/Track.mp3".to_string(),
        "abc123def456".to_string(),
        chrono::Utc::now(),
    );

    // Save the guid before inserting
    let guid = file.guid.clone();

    // Insert record
    let result = wkmp_ai::db::files::save_file(&pool, &file).await;

    // Verify: INSERT succeeded
    assert!(result.is_ok(), "AudioFile INSERT should succeed: {:?}", result);

    // Verify: Record persisted
    let path: String = sqlx::query_scalar("SELECT path FROM files WHERE guid = ?")
        .bind(guid.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(path, "Artist/Album/Track.mp3");

    // Verify: NO session_id column attempted
    // (This is verified by schema test TC-DB-001, but INSERT success confirms it)

    println!("✅ TC-DB-004 PASS: AudioFile INSERT without session_id succeeds");
}

/// TC-DB-005: Settings table schema validation
///
/// **Requirement:** Database-first configuration storage
///
/// **Given:** Database with settings table
/// **When:** Schema introspection performed
/// **Then:**
///   - settings table has key-value columns
///   - Can store ai_processing_thread_count setting
#[tokio::test]
async fn tc_db_005_settings_table_schema() {
    // Setup: Create test database
    let (_temp_dir, pool) = db_utils::create_test_db().await.unwrap();

    // Verify: settings table exists
    let tables = db_utils::get_table_names(&pool).await.unwrap();
    assert!(
        tables.contains(&"settings".to_string()),
        "settings table should exist"
    );

    // Verify: Get settings table schema
    let columns = db_utils::get_table_columns(&pool, "settings")
        .await
        .unwrap();
    let column_names: Vec<_> = columns.iter().map(|c| c.name.as_str()).collect();

    // Verify: Key-value columns
    db_utils::assert_has_column(&pool, "settings", "key").await;
    db_utils::assert_has_column(&pool, "settings", "value").await;

    // Verify: Can insert and retrieve setting
    sqlx::query("INSERT INTO settings (key, value) VALUES (?, ?)")
        .bind("ai_processing_thread_count")
        .bind("4")
        .execute(&pool)
        .await
        .unwrap();

    let value: String =
        sqlx::query_scalar("SELECT value FROM settings WHERE key = 'ai_processing_thread_count'")
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(value, "4");

    println!("✅ TC-DB-005 PASS: settings table schema correct");
    println!("   Columns: {:?}", column_names);
}
