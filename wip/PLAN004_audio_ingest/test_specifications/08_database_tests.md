# Database Tests - wkmp-ai

**Requirements:** AIA-DB-010
**Priority:** P0 (Critical)
**Test Count:** 8

---

## TEST-021: Files Table Written

**Requirement:** AIA-DB-010
**Type:** Integration
**Priority:** P0

**Given:**
- Audio file: test.mp3 (SHA-256 hash, 180s duration, 44100 Hz, 2 channels)

**When:**
- Insert file record

**Then:**
- Row in files table:
  - file_uuid: Valid UUID
  - file_path: "/path/to/test.mp3"
  - hash: "sha256_hash_value"
  - duration_ticks: 5,080,320,000 (180 × 28,224,000)
  - sample_rate: 44100
  - channels: 2

**Acceptance Criteria:**
- ✅ File record inserted
- ✅ UUID generated automatically
- ✅ Tick conversion accurate
- ✅ No duplicate hash allowed (UNIQUE constraint)

---

## TEST-022: Passages Table Written

**Requirement:** AIA-DB-010
**Type:** Integration
**Priority:** P0

**Given:**
- File already in database (file_id = 1)
- Passage timing: start=10s, end=170s, lead_in=2s, lead_out=3s

**When:**
- Insert passage record

**Then:**
- Row in passages table:
  - passage_uuid: Valid UUID
  - file_id: 1
  - start_time_ticks: 282,240,000
  - end_time_ticks: 4,798,080,000
  - lead_in_start_ticks: 56,448,000
  - lead_out_start_ticks: 84,672,000
  - title: "Song Title"
  - import_metadata: JSON with amplitude analysis

**Acceptance Criteria:**
- ✅ Passage record inserted
- ✅ Foreign key to files table valid
- ✅ Tick values correct
- ✅ import_metadata is valid JSON

---

## TEST-023: Songs/Artists/Works Tables

**Requirement:** AIA-DB-010
**Type:** Integration
**Priority:** P0

**Given:**
- MusicBrainz Recording with:
  - MBID: recording-uuid
  - Title: "Yesterday"
  - Artist: "The Beatles" (artist-uuid)
  - Work: "Yesterday" (work-uuid)

**When:**
- Insert song, artist, work entities

**Then:**
- songs table: 1 row with recording MBID
- artists table: 1 row with artist MBID
- works table: 1 row with work MBID
- song.work_id links to work.id

**Acceptance Criteria:**
- ✅ All entities inserted
- ✅ MBIDs stored correctly
- ✅ Foreign key relationships valid
- ✅ No duplicates on MBID (UNIQUE constraint)

---

## TEST-024: Passage Relationships

**Requirement:** AIA-DB-010
**Type:** Integration
**Priority:** P0

**Given:**
- passage_id = 1
- song_id = 1
- album_id = 1

**When:**
- Link passage to song and album

**Then:**
- passage_songs table: 1 row (passage_id=1, song_id=1)
- passage_albums table: 1 row (passage_id=1, album_id=1)
- Relationships queryable

**Acceptance Criteria:**
- ✅ Many-to-many relationships created
- ✅ No duplicate links (UNIQUE constraint)
- ✅ Foreign keys valid
- ✅ CASCADE delete works (delete passage → delete relationships)

---

## TEST-025: Cache Tables Written

**Requirement:** AIA-DB-010
**Type:** Integration
**Priority:** P0

**Given:**
- AcoustID fingerprint hash: "hash123"
- MusicBrainz MBID: "recording-uuid"
- AcousticBrainz flavor JSON: "{...}"

**When:**
- Cache API responses

**Then:**
- acoustid_cache: 1 row (fingerprint_hash, mbid)
- musicbrainz_cache: 1 row (mbid, entity_type='recording', response_json)
- acousticbrainz_cache: 1 row (recording_mbid, flavor_json)

**Acceptance Criteria:**
- ✅ All cache tables populated
- ✅ Cached_at timestamp set
- ✅ Upsert logic works (ON CONFLICT DO UPDATE)
- ✅ Cache retrieval accurate

---

## TEST-026: Settings Table Read

**Requirement:** AIA-DB-010
**Type:** Integration
**Priority:** P0

**Given:**
- settings table has row:
  - key = 'import_parameters'
  - value_type = 'json'
  - value_text = '{"rms_window_ms": 100, ...}'

**When:**
- Load import parameters

**Then:**
- Parameters deserialized from JSON
- Default values used if key missing
- Type validation applied

**Acceptance Criteria:**
- ✅ JSON parsed successfully
- ✅ Parameters struct populated
- ✅ Defaults applied for missing keys
- ✅ Invalid JSON handled gracefully

---

## TEST-027: Transaction Handling

**Requirement:** AIA-DB-010
**Type:** Integration
**Priority:** P0

**Given:**
- Transaction to insert: file + passage + song + relationships
- Midway through: database error occurs

**When:**
- Error causes transaction rollback

**Then:**
- NO file record in database
- NO passage record
- NO song record
- Database in consistent state (all-or-nothing)

**Acceptance Criteria:**
- ✅ Transaction rolled back completely
- ✅ No partial data
- ✅ No orphaned records
- ✅ Foreign key constraints maintained

---

## TEST-028: Foreign Key Cascades

**Requirement:** AIA-DB-010
**Type:** Integration
**Priority:** P0

**Given:**
- file_id = 1
- passage_id = 1 (linked to file_id = 1)
- passage_songs row (passage_id = 1, song_id = 1)

**When:**
- DELETE FROM files WHERE id = 1

**Then:**
- passages with file_id = 1 deleted (CASCADE)
- passage_songs with passage_id = 1 deleted (CASCADE)
- song_id = 1 remains (not cascade deleted)

**Acceptance Criteria:**
- ✅ File deletion cascades to passages
- ✅ Passage deletion cascades to relationships
- ✅ Songs/artists/works NOT cascade deleted (shared entities)
- ✅ Database integrity maintained

---

## Test Implementation Notes

**Framework:** `cargo test --test database_tests -p wkmp-ai`

**In-Memory Database Setup:**
```rust
use sqlx::sqlite::SqlitePoolOptions;

async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();

    // Run migrations
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .unwrap();

    pool
}

#[tokio::test]
async fn test_files_table_insert() {
    let db = setup_test_db().await;

    let file_id = insert_file(
        &db,
        Path::new("/test/file.mp3"),
        "hash123",
        28_224_000, // 1 second
        44100,
        2,
        Some(16),
    ).await.unwrap();

    assert!(file_id > 0);

    // Verify inserted
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM files")
        .fetch_one(&db)
        .await
        .unwrap();

    assert_eq!(count.0, 1);
}
```

**Transaction Test:**
```rust
#[tokio::test]
async fn test_transaction_rollback() {
    let db = setup_test_db().await;

    let result = async {
        let mut tx = db.begin().await?;

        // Insert file
        insert_file_tx(&mut tx, &file_data).await?;

        // Simulate error
        return Err(sqlx::Error::RowNotFound);

        tx.commit().await?;
        Ok(())
    }.await;

    assert!(result.is_err());

    // Verify no data inserted
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM files")
        .fetch_one(&db)
        .await
        .unwrap();

    assert_eq!(count.0, 0); // Rolled back
}
```

**Foreign Key Cascade Test:**
```rust
#[tokio::test]
async fn test_cascade_delete() {
    let db = setup_test_db().await;

    // Insert file → passage → relationships
    let file_id = insert_file(&db, ...).await.unwrap();
    let passage_id = insert_passage(&db, file_id, ...).await.unwrap();
    link_passage_song(&db, passage_id, song_id).await.unwrap();

    // Delete file
    sqlx::query("DELETE FROM files WHERE id = ?")
        .bind(file_id)
        .execute(&db)
        .await
        .unwrap();

    // Verify cascades
    let passage_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM passages WHERE id = ?")
        .bind(passage_id)
        .fetch_one(&db)
        .await
        .unwrap();

    assert_eq!(passage_count.0, 0); // Cascade deleted
}
```

---

End of database tests
