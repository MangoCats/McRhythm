# WKMP Audio Ingest Database Queries

**⚙️ TIER 3 - IMPLEMENTATION SPECIFICATION**

Defines SQL queries for wkmp-ai import workflow. Derived from [SPEC024](SPEC024-audio_ingest_architecture.md) and [IMPL001](IMPL001-database_schema.md).

> **Related:** [Audio Ingest Architecture](SPEC024-audio_ingest_architecture.md) | [Database Schema](IMPL001-database_schema.md) | [MusicBrainz Client](IMPL011-musicbrainz_client.md)

---

## Overview

**Module:** `wkmp-ai/src/db/queries.rs`
**Purpose:** Centralize all SQL queries for import workflow
**Database:** SQLite with JSON1 extension
**ORM:** sqlx (compile-time verified queries)

---

## File Operations

### Insert File Record

```rust
/// Insert new file record
pub async fn insert_file(
    db: &SqlitePool,
    file_path: &Path,
    hash: &str,
    duration_ticks: i64,
    sample_rate: u32,
    channels: u32,
    bit_depth: Option<u32>,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO files (
            file_uuid,
            file_path,
            hash,
            duration_ticks,
            sample_rate,
            channels,
            bit_depth
        )
        VALUES (
            lower(hex(randomblob(16))),
            ?,
            ?,
            ?,
            ?,
            ?,
            ?
        )
        RETURNING id"
    )
    .bind(file_path.to_string_lossy().as_ref())
    .bind(hash)
    .bind(duration_ticks)
    .bind(sample_rate as i32)
    .bind(channels as i32)
    .bind(bit_depth.map(|d| d as i32))
    .fetch_one(db)
    .await?;

    Ok(row.0)
}
```

### Check File Already Imported

```rust
/// Check if file hash already exists (duplicate detection)
pub async fn file_exists_by_hash(
    db: &SqlitePool,
    hash: &str,
) -> Result<bool, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM files WHERE hash = ?"
    )
    .bind(hash)
    .fetch_one(db)
    .await?;

    Ok(row.0 > 0)
}
```

---

## Passage Operations

### Insert Passage

```rust
/// Insert passage with timing points
pub async fn insert_passage(
    db: &SqlitePool,
    passage: &PassageInsert,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO passages (
            passage_uuid,
            file_id,
            start_time_ticks,
            end_time_ticks,
            lead_in_start_ticks,
            lead_out_start_ticks,
            fade_in_start_ticks,
            fade_out_start_ticks,
            fade_in_curve,
            fade_out_curve,
            title,
            artist,
            album,
            musical_flavor_vector,
            import_metadata
        )
        VALUES (
            lower(hex(randomblob(16))),
            ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
        )
        RETURNING id"
    )
    .bind(passage.file_id)
    .bind(passage.start_time_ticks)
    .bind(passage.end_time_ticks)
    .bind(passage.lead_in_start_ticks)
    .bind(passage.lead_out_start_ticks)
    .bind(passage.fade_in_start_ticks)
    .bind(passage.fade_out_start_ticks)
    .bind(&passage.fade_in_curve)
    .bind(&passage.fade_out_curve)
    .bind(&passage.title)
    .bind(&passage.artist)
    .bind(&passage.album)
    .bind(&passage.musical_flavor_vector)
    .bind(&passage.import_metadata)
    .fetch_one(db)
    .await?;

    Ok(row.0)
}

pub struct PassageInsert {
    pub file_id: i64,
    pub start_time_ticks: i64,
    pub end_time_ticks: i64,
    pub lead_in_start_ticks: Option<i64>,
    pub lead_out_start_ticks: Option<i64>,
    pub fade_in_start_ticks: Option<i64>,
    pub fade_out_start_ticks: Option<i64>,
    pub fade_in_curve: Option<String>,
    pub fade_out_curve: Option<String>,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub musical_flavor_vector: Option<String>,  // JSON
    pub import_metadata: Option<String>,  // JSON
}
```

### Batch Insert Passages

```rust
/// Insert multiple passages in single transaction (performance optimization)
pub async fn batch_insert_passages(
    db: &SqlitePool,
    passages: &[PassageInsert],
) -> Result<Vec<i64>, sqlx::Error> {
    if passages.is_empty() {
        return Ok(Vec::new());
    }

    let mut tx = db.begin().await?;
    let mut passage_ids = Vec::with_capacity(passages.len());

    for passage in passages {
        let row: (i64,) = sqlx::query_as(
            "INSERT INTO passages (
                passage_uuid, file_id, start_time_ticks, end_time_ticks,
                lead_in_start_ticks, lead_out_start_ticks,
                fade_in_start_ticks, fade_out_start_ticks,
                fade_in_curve, fade_out_curve,
                title, artist, album, musical_flavor_vector, import_metadata
            )
            VALUES (
                lower(hex(randomblob(16))), ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
            )
            RETURNING id"
        )
        .bind(passage.file_id)
        .bind(passage.start_time_ticks)
        .bind(passage.end_time_ticks)
        .bind(passage.lead_in_start_ticks)
        .bind(passage.lead_out_start_ticks)
        .bind(passage.fade_in_start_ticks)
        .bind(passage.fade_out_start_ticks)
        .bind(&passage.fade_in_curve)
        .bind(&passage.fade_out_curve)
        .bind(&passage.title)
        .bind(&passage.artist)
        .bind(&passage.album)
        .bind(&passage.musical_flavor_vector)
        .bind(&passage.import_metadata)
        .fetch_one(&mut *tx)
        .await?;

        passage_ids.push(row.0);
    }

    tx.commit().await?;

    Ok(passage_ids)
}
```

---

## Song/Artist/Work Entity Operations

### Upsert Song (INSERT or UPDATE)

```rust
/// Insert or update song by MBID
pub async fn upsert_song(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    mbid: &str,
    title: &str,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO songs (song_uuid, mbid, name)
         VALUES (lower(hex(randomblob(16))), ?, ?)
         ON CONFLICT(mbid) DO UPDATE SET
            name = excluded.name,
            updated_at = CURRENT_TIMESTAMP
         RETURNING id"
    )
    .bind(mbid)
    .bind(title)
    .fetch_one(&mut **tx)
    .await?;

    Ok(row.0)
}
```

### Upsert Artist

```rust
/// Insert or update artist by MBID
pub async fn upsert_artist(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    mbid: &str,
    name: &str,
    sort_name: &str,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO artists (artist_uuid, mbid, name, sort_name)
         VALUES (lower(hex(randomblob(16))), ?, ?, ?)
         ON CONFLICT(mbid) DO UPDATE SET
            name = excluded.name,
            sort_name = excluded.sort_name,
            updated_at = CURRENT_TIMESTAMP
         RETURNING id"
    )
    .bind(mbid)
    .bind(name)
    .bind(sort_name)
    .fetch_one(&mut **tx)
    .await?;

    Ok(row.0)
}
```

### Upsert Work

```rust
/// Insert or update work by MBID
pub async fn upsert_work(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    mbid: &str,
    title: &str,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO works (work_uuid, mbid, name)
         VALUES (lower(hex(randomblob(16))), ?, ?)
         ON CONFLICT(mbid) DO UPDATE SET
            name = excluded.name,
            updated_at = CURRENT_TIMESTAMP
         RETURNING id"
    )
    .bind(mbid)
    .bind(title)
    .fetch_one(&mut **tx)
    .await?;

    Ok(row.0)
}
```

### Upsert Album

```rust
/// Insert or update album by MBID
pub async fn upsert_album(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    mbid: &str,
    title: &str,
    release_date: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO albums (album_uuid, mbid, name, release_date)
         VALUES (lower(hex(randomblob(16))), ?, ?, ?)
         ON CONFLICT(mbid) DO UPDATE SET
            name = excluded.name,
            release_date = excluded.release_date,
            updated_at = CURRENT_TIMESTAMP
         RETURNING id"
    )
    .bind(mbid)
    .bind(title)
    .bind(release_date)
    .fetch_one(&mut **tx)
    .await?;

    Ok(row.0)
}
```

---

## Relationship Operations

### Link Passage to Song

```rust
/// Link passage to song (many-to-many)
pub async fn link_passage_song(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    passage_id: i64,
    song_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO passage_songs (passage_id, song_id)
         VALUES (?, ?)
         ON CONFLICT DO NOTHING"
    )
    .bind(passage_id)
    .bind(song_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
```

### Link Passage to Album

```rust
/// Link passage to album (many-to-many)
pub async fn link_passage_album(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    passage_id: i64,
    album_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO passage_albums (passage_id, album_id)
         VALUES (?, ?)
         ON CONFLICT DO NOTHING"
    )
    .bind(passage_id)
    .bind(album_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
```

### Link Song to Artist

```rust
/// Link song to artist (many-to-many)
pub async fn link_song_artist(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    song_id: i64,
    artist_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO song_artists (song_id, artist_id)
         VALUES (?, ?)
         ON CONFLICT DO NOTHING"
    )
    .bind(song_id)
    .bind(artist_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
```

### Link Song to Work

```rust
/// Link song to work (one-to-many: song has one work)
pub async fn link_song_work(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    song_id: i64,
    work_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE songs SET work_id = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?"
    )
    .bind(work_id)
    .bind(song_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
```

---

## Cache Operations

### AcoustID Cache

```rust
/// Get cached MBID from fingerprint
pub async fn get_acoustid_cache(
    db: &SqlitePool,
    fingerprint_hash: &str,
) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT mbid FROM acoustid_cache
         WHERE fingerprint_hash = ?"
    )
    .bind(fingerprint_hash)
    .fetch_optional(db)
    .await?;

    Ok(row.map(|(mbid,)| mbid))
}

/// Cache fingerprint → MBID mapping
pub async fn insert_acoustid_cache(
    db: &SqlitePool,
    fingerprint_hash: &str,
    mbid: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO acoustid_cache (fingerprint_hash, mbid, cached_at)
         VALUES (?, ?, datetime('now'))
         ON CONFLICT(fingerprint_hash) DO UPDATE SET
            mbid = excluded.mbid,
            cached_at = excluded.cached_at"
    )
    .bind(fingerprint_hash)
    .bind(mbid)
    .execute(db)
    .await?;

    Ok(())
}
```

### MusicBrainz Cache

```rust
/// Get cached MusicBrainz response
pub async fn get_musicbrainz_cache(
    db: &SqlitePool,
    mbid: &str,
    entity_type: &str,
) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT response_json FROM musicbrainz_cache
         WHERE mbid = ? AND entity_type = ?"
    )
    .bind(mbid)
    .bind(entity_type)
    .fetch_optional(db)
    .await?;

    Ok(row.map(|(json,)| json))
}

/// Cache MusicBrainz response
pub async fn insert_musicbrainz_cache(
    db: &SqlitePool,
    mbid: &str,
    entity_type: &str,
    response_json: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO musicbrainz_cache (mbid, entity_type, response_json, cached_at)
         VALUES (?, ?, ?, datetime('now'))
         ON CONFLICT(mbid, entity_type) DO UPDATE SET
            response_json = excluded.response_json,
            cached_at = excluded.cached_at"
    )
    .bind(mbid)
    .bind(entity_type)
    .bind(response_json)
    .execute(db)
    .await?;

    Ok(())
}
```

### AcousticBrainz Cache

```rust
/// Get cached AcousticBrainz musical flavor vector
pub async fn get_acousticbrainz_cache(
    db: &SqlitePool,
    mbid: &str,
) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT flavor_json FROM acousticbrainz_cache
         WHERE recording_mbid = ?"
    )
    .bind(mbid)
    .fetch_optional(db)
    .await?;

    Ok(row.map(|(json,)| json))
}

/// Cache AcousticBrainz flavor vector
pub async fn insert_acousticbrainz_cache(
    db: &SqlitePool,
    mbid: &str,
    flavor_json: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO acousticbrainz_cache (recording_mbid, flavor_json, cached_at)
         VALUES (?, ?, datetime('now'))
         ON CONFLICT(recording_mbid) DO UPDATE SET
            flavor_json = excluded.flavor_json,
            cached_at = excluded.cached_at"
    )
    .bind(mbid)
    .bind(flavor_json)
    .execute(db)
    .await?;

    Ok(())
}
```

---

## Parameter Management

### Get Global Import Parameters

```rust
/// Load global import parameters from settings table
pub async fn get_import_parameters(
    db: &SqlitePool,
) -> Result<Option<String>, sqlx::Error> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT value_text FROM settings
         WHERE key = 'import_parameters' AND value_type = 'json'"
    )
    .fetch_optional(db)
    .await?;

    Ok(row.map(|(json,)| json))
}
```

### Save Global Import Parameters

```rust
/// Save global import parameters to settings table
pub async fn save_import_parameters(
    db: &SqlitePool,
    parameters_json: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO settings (key, value_type, value_text)
         VALUES ('import_parameters', 'json', ?)
         ON CONFLICT(key) DO UPDATE SET
            value_text = excluded.value_text"
    )
    .bind(parameters_json)
    .execute(db)
    .await?;

    Ok(())
}
```

---

## Transaction Helpers

### Complete Import Transaction

```rust
/// Complete import workflow for single file
pub async fn import_file_transaction(
    db: &SqlitePool,
    file_insert: FileInsert,
    passages: Vec<PassageInsert>,
    song_id: Option<i64>,
    album_id: Option<i64>,
) -> Result<Vec<i64>, sqlx::Error> {
    let mut tx = db.begin().await?;

    // 1. Insert file
    let file_id = insert_file_tx(&mut tx, &file_insert).await?;

    // 2. Insert passages
    let mut passage_ids = Vec::new();
    for mut passage in passages {
        passage.file_id = file_id;
        let passage_id = insert_passage_tx(&mut tx, &passage).await?;
        passage_ids.push(passage_id);

        // 3. Link to song/album if present
        if let Some(sid) = song_id {
            link_passage_song(&mut tx, passage_id, sid).await?;
        }
        if let Some(aid) = album_id {
            link_passage_album(&mut tx, passage_id, aid).await?;
        }
    }

    tx.commit().await?;

    Ok(passage_ids)
}

async fn insert_file_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    file: &FileInsert,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO files (file_uuid, file_path, hash, duration_ticks, sample_rate, channels)
         VALUES (lower(hex(randomblob(16))), ?, ?, ?, ?, ?)
         RETURNING id"
    )
    .bind(&file.file_path)
    .bind(&file.hash)
    .bind(file.duration_ticks)
    .bind(file.sample_rate as i32)
    .bind(file.channels as i32)
    .fetch_one(&mut **tx)
    .await?;

    Ok(row.0)
}

async fn insert_passage_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    passage: &PassageInsert,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO passages (
            passage_uuid, file_id, start_time_ticks, end_time_ticks,
            lead_in_start_ticks, lead_out_start_ticks,
            title, artist, album, import_metadata
        )
        VALUES (lower(hex(randomblob(16))), ?, ?, ?, ?, ?, ?, ?, ?, ?)
        RETURNING id"
    )
    .bind(passage.file_id)
    .bind(passage.start_time_ticks)
    .bind(passage.end_time_ticks)
    .bind(passage.lead_in_start_ticks)
    .bind(passage.lead_out_start_ticks)
    .bind(&passage.title)
    .bind(&passage.artist)
    .bind(&passage.album)
    .bind(&passage.import_metadata)
    .fetch_one(&mut **tx)
    .await?;

    Ok(row.0)
}

pub struct FileInsert {
    pub file_path: String,
    pub hash: String,
    pub duration_ticks: i64,
    pub sample_rate: u32,
    pub channels: u32,
}
```

---

## Tick Conversion Utilities

```rust
/// Convert seconds to ticks (28,224,000 ticks/second)
pub fn seconds_to_ticks(seconds: f64) -> i64 {
    const TICKS_PER_SECOND: i64 = 28_224_000;
    (seconds * TICKS_PER_SECOND as f64).floor() as i64
}

/// Convert ticks to seconds
pub fn ticks_to_seconds(ticks: i64) -> f64 {
    const TICKS_PER_SECOND: i64 = 28_224_000;
    ticks as f64 / TICKS_PER_SECOND as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_conversion() {
        assert_eq!(seconds_to_ticks(1.0), 28_224_000);
        assert_eq!(seconds_to_ticks(2.5), 70_560_000);
        assert_eq!(ticks_to_seconds(28_224_000), 1.0);
        assert_eq!(ticks_to_seconds(70_560_000), 2.5);

        // Round trip
        let original = 123.456789;
        let ticks = seconds_to_ticks(original);
        let converted = ticks_to_seconds(ticks);
        assert!((original - converted).abs() < 0.0001);
    }
}
```

---

## Testing

### Integration Tests with In-Memory Database

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqlitePool {
        let db = SqlitePoolOptions::new()
            .connect(":memory:")
            .await
            .unwrap();

        // Run migrations
        sqlx::migrate!("../migrations")
            .run(&db)
            .await
            .unwrap();

        db
    }

    #[tokio::test]
    async fn test_insert_file() {
        let db = setup_test_db().await;

        let file_id = insert_file(
            &db,
            Path::new("/test/file.mp3"),
            "hash123",
            28_224_000,  // 1 second
            44100,
            2,
            Some(16),
        ).await.unwrap();

        assert!(file_id > 0);
    }

    #[tokio::test]
    async fn test_upsert_song() {
        let db = setup_test_db().await;
        let mut tx = db.begin().await.unwrap();

        let song_id_1 = upsert_song(&mut tx, "mbid-123", "Test Song").await.unwrap();
        let song_id_2 = upsert_song(&mut tx, "mbid-123", "Test Song Updated").await.unwrap();

        // Same MBID should return same ID (upsert)
        assert_eq!(song_id_1, song_id_2);

        tx.commit().await.unwrap();
    }
}
```

---

**Document Version:** 1.0
**Last Updated:** 2025-10-27
**Status:** Implementation specification (ready for coding)
