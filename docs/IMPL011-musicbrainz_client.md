# WKMP MusicBrainz Client Implementation

**⚙️ TIER 3 - IMPLEMENTATION SPECIFICATION**

Defines Rust implementation for MusicBrainz API client. Derived from [SPEC032](SPEC032-audio_ingest_architecture.md) and [SPEC008](SPEC008-library_management.md).

> **Related:** [Audio Ingest Architecture](SPEC032-audio_ingest_architecture.md) | [Library Management](SPEC008-library_management.md) | [Database Schema](IMPL001-database_schema.md)

---

## Overview

**Module:** `wkmp-ai/src/services/musicbrainz_client.rs`
**Purpose:** Query MusicBrainz API for recording metadata, artist, work, and album relationships
**Rate Limit:** 1 request/second (STRICT - enforced by MusicBrainz)

---

## API Endpoints

### Base URL
```
https://musicbrainz.org/ws/2/
```

### Endpoint: Recording Lookup

**URL:** `GET /recording/{mbid}`

**Query Parameters:**
- `inc` - Include relationships (e.g., `artist-credits+releases+work-rels`)
- `fmt` - Response format (`json`)

**Example:**
```
GET https://musicbrainz.org/ws/2/recording/5e8d5f0b-3f8a-4c7e-9c4b-5e8d5f0b3f8a?inc=artist-credits+releases+work-rels&fmt=json
```

**Response Structure:**
```json
{
  "id": "5e8d5f0b-3f8a-4c7e-9c4b-5e8d5f0b3f8a",
  "title": "Yesterday",
  "length": 123000,
  "artist-credit": [
    {
      "name": "The Beatles",
      "artist": {
        "id": "b10bbbfc-cf9e-42e0-be17-e2c3e1d2600d",
        "name": "The Beatles",
        "sort-name": "Beatles, The"
      }
    }
  ],
  "releases": [
    {
      "id": "album-uuid",
      "title": "Help!",
      "date": "1965-08-06"
    }
  ],
  "relations": [
    {
      "type": "performance",
      "type-id": "a3005666-a872-32c3-ad06-98af558e99b0",
      "work": {
        "id": "work-uuid",
        "title": "Yesterday"
      }
    }
  ]
}
```

---

## Rust Implementation

### Data Structures

```rust
// wkmp-ai/src/services/musicbrainz_client.rs

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// MusicBrainz Recording response
#[derive(Debug, Clone, Deserialize)]
pub struct MBRecording {
    pub id: String,  // MBID
    pub title: String,
    pub length: Option<u64>,  // Milliseconds
    #[serde(rename = "artist-credit")]
    pub artist_credit: Vec<MBArtistCredit>,
    pub releases: Option<Vec<MBRelease>>,
    pub relations: Option<Vec<MBRelation>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MBArtistCredit {
    pub name: String,  // Display name (may differ from artist.name)
    pub artist: MBArtist,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MBArtist {
    pub id: String,  // MBID
    pub name: String,
    #[serde(rename = "sort-name")]
    pub sort_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MBRelease {
    pub id: String,  // MBID
    pub title: String,
    pub date: Option<String>,  // YYYY-MM-DD format
}

#[derive(Debug, Clone, Deserialize)]
pub struct MBRelation {
    #[serde(rename = "type")]
    pub relation_type: String,
    #[serde(rename = "type-id")]
    pub type_id: String,
    pub work: Option<MBWork>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MBWork {
    pub id: String,  // MBID
    pub title: String,
}
```

### Rate Limiter

```rust
/// Rate limiter enforcing 1 request/second
pub struct RateLimiter {
    last_request: Mutex<Option<Instant>>,
    min_interval: Duration,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            last_request: Mutex::new(None),
            min_interval: Duration::from_millis(1000),  // 1 req/s
        }
    }

    /// Wait if necessary to comply with rate limit
    pub async fn wait(&self) {
        let mut last = self.last_request.lock().await;

        if let Some(last_time) = *last {
            let elapsed = last_time.elapsed();
            if elapsed < self.min_interval {
                let wait_time = self.min_interval - elapsed;
                tokio::time::sleep(wait_time).await;
            }
        }

        *last = Some(Instant::now());
    }
}
```

### MusicBrainz Client

```rust
pub struct MusicBrainzClient {
    http_client: reqwest::Client,
    rate_limiter: RateLimiter,
    db: SqlitePool,
}

impl MusicBrainzClient {
    pub fn new(db: SqlitePool) -> Self {
        let http_client = reqwest::Client::builder()
            .user_agent("WKMP/1.0 (https://github.com/yourproject/wkmp)")
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            http_client,
            rate_limiter: RateLimiter::new(),
            db,
        }
    }

    /// Lookup recording by MBID
    pub async fn lookup_recording(
        &self,
        mbid: &str,
    ) -> Result<MBRecording, MBError> {
        // 1. Check cache first
        if let Some(cached) = self.get_cached_recording(mbid).await? {
            return Ok(cached);
        }

        // 2. Rate limit
        self.rate_limiter.wait().await;

        // 3. Query API
        let url = format!(
            "https://musicbrainz.org/ws/2/recording/{}?inc=artist-credits+releases+work-rels&fmt=json",
            mbid
        );

        let response = self.http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| MBError::NetworkError(e.to_string()))?;

        if response.status() == 404 {
            return Err(MBError::RecordingNotFound(mbid.to_string()));
        }

        if response.status() == 503 {
            return Err(MBError::RateLimitExceeded);
        }

        if !response.status().is_success() {
            return Err(MBError::ApiError(
                response.status().as_u16(),
                response.text().await.unwrap_or_default()
            ));
        }

        let recording: MBRecording = response.json().await
            .map_err(|e| MBError::ParseError(e.to_string()))?;

        // 4. Cache response
        self.cache_recording(&recording).await?;

        Ok(recording)
    }

    /// Get cached recording from database
    async fn get_cached_recording(&self, mbid: &str) -> Result<Option<MBRecording>, MBError> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT response_json FROM musicbrainz_cache
             WHERE mbid = ? AND entity_type = 'recording'"
        )
        .bind(mbid)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| MBError::DatabaseError(e.to_string()))?;

        if let Some((json,)) = row {
            let recording: MBRecording = serde_json::from_str(&json)
                .map_err(|e| MBError::ParseError(e.to_string()))?;
            Ok(Some(recording))
        } else {
            Ok(None)
        }
    }

    /// Cache recording response to database
    async fn cache_recording(&self, recording: &MBRecording) -> Result<(), MBError> {
        let json = serde_json::to_string(recording)
            .map_err(|e| MBError::ParseError(e.to_string()))?;

        sqlx::query(
            "INSERT INTO musicbrainz_cache (mbid, entity_type, response_json, cached_at)
             VALUES (?, 'recording', ?, datetime('now'))
             ON CONFLICT(mbid, entity_type) DO UPDATE SET
                response_json = excluded.response_json,
                cached_at = excluded.cached_at"
        )
        .bind(&recording.id)
        .bind(&json)
        .execute(&self.db)
        .await
        .map_err(|e| MBError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}
```

---

## Database Entity Creation

### Entity Creation Workflow

```rust
impl MusicBrainzClient {
    /// Create database entities from MusicBrainz recording
    pub async fn create_entities(
        &self,
        recording: &MBRecording,
        file_id: i64,
    ) -> Result<i64, MBError> {
        let mut tx = self.db.begin().await
            .map_err(|e| MBError::DatabaseError(e.to_string()))?;

        // 1. Create/get song (MusicBrainz Recording)
        let song_id = self.upsert_song(&mut tx, recording).await?;

        // 2. Create/get artists
        for artist_credit in &recording.artist_credit {
            let artist_id = self.upsert_artist(&mut tx, &artist_credit.artist).await?;
            self.link_song_artist(&mut tx, song_id, artist_id).await?;
        }

        // 3. Create/get work (if present)
        if let Some(relations) = &recording.relations {
            for relation in relations {
                if relation.relation_type == "performance" {
                    if let Some(work) = &relation.work {
                        let work_id = self.upsert_work(&mut tx, work).await?;
                        self.link_song_work(&mut tx, song_id, work_id).await?;
                    }
                }
            }
        }

        // 4. Create/get albums (releases)
        if let Some(releases) = &recording.releases {
            for release in releases {
                let album_id = self.upsert_album(&mut tx, release).await?;
                // Note: passage_albums link created later when passage is created
            }
        }

        tx.commit().await
            .map_err(|e| MBError::DatabaseError(e.to_string()))?;

        Ok(song_id)
    }

    async fn upsert_song(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        recording: &MBRecording,
    ) -> Result<i64, MBError> {
        let row: (i64,) = sqlx::query_as(
            "INSERT INTO songs (song_uuid, mbid, name)
             VALUES (lower(hex(randomblob(16))), ?, ?)
             ON CONFLICT(mbid) DO UPDATE SET name = excluded.name
             RETURNING id"
        )
        .bind(&recording.id)
        .bind(&recording.title)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| MBError::DatabaseError(e.to_string()))?;

        Ok(row.0)
    }

    async fn upsert_artist(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        artist: &MBArtist,
    ) -> Result<i64, MBError> {
        let row: (i64,) = sqlx::query_as(
            "INSERT INTO artists (artist_uuid, mbid, name, sort_name)
             VALUES (lower(hex(randomblob(16))), ?, ?, ?)
             ON CONFLICT(mbid) DO UPDATE SET
                name = excluded.name,
                sort_name = excluded.sort_name
             RETURNING id"
        )
        .bind(&artist.id)
        .bind(&artist.name)
        .bind(&artist.sort_name)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| MBError::DatabaseError(e.to_string()))?;

        Ok(row.0)
    }

    async fn link_song_artist(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        song_id: i64,
        artist_id: i64,
    ) -> Result<(), MBError> {
        sqlx::query(
            "INSERT INTO song_artists (song_id, artist_id)
             VALUES (?, ?)
             ON CONFLICT DO NOTHING"
        )
        .bind(song_id)
        .bind(artist_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| MBError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn upsert_work(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        work: &MBWork,
    ) -> Result<i64, MBError> {
        let row: (i64,) = sqlx::query_as(
            "INSERT INTO works (work_uuid, mbid, name)
             VALUES (lower(hex(randomblob(16))), ?, ?)
             ON CONFLICT(mbid) DO UPDATE SET name = excluded.name
             RETURNING id"
        )
        .bind(&work.id)
        .bind(&work.title)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| MBError::DatabaseError(e.to_string()))?;

        Ok(row.0)
    }

    async fn link_song_work(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        song_id: i64,
        work_id: i64,
    ) -> Result<(), MBError> {
        sqlx::query(
            "UPDATE songs SET work_id = ? WHERE id = ?"
        )
        .bind(work_id)
        .bind(song_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| MBError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn upsert_album(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        release: &MBRelease,
    ) -> Result<i64, MBError> {
        let row: (i64,) = sqlx::query_as(
            "INSERT INTO albums (album_uuid, mbid, name, release_date)
             VALUES (lower(hex(randomblob(16))), ?, ?, ?)
             ON CONFLICT(mbid) DO UPDATE SET
                name = excluded.name,
                release_date = excluded.release_date
             RETURNING id"
        )
        .bind(&release.id)
        .bind(&release.title)
        .bind(&release.date)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| MBError::DatabaseError(e.to_string()))?;

        Ok(row.0)
    }
}
```

---

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum MBError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Recording not found: {0}")]
    RecordingNotFound(String),

    #[error("MusicBrainz rate limit exceeded")]
    RateLimitExceeded,

    #[error("MusicBrainz API error {0}: {1}")]
    ApiError(u16, String),

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}
```

---

## Usage Example

```rust
// In import workflow
let mb_client = MusicBrainzClient::new(db_pool.clone());

// After AcoustID lookup returns MBID
let mbid = "5e8d5f0b-3f8a-4c7e-9c4b-5e8d5f0b3f8a";

match mb_client.lookup_recording(mbid).await {
    Ok(recording) => {
        // Create database entities
        let song_id = mb_client.create_entities(&recording, file_id).await?;

        // Link passage to song (later in workflow)
        sqlx::query(
            "INSERT INTO passage_songs (passage_id, song_id)
             VALUES (?, ?)"
        )
        .bind(passage_id)
        .bind(song_id)
        .execute(&db_pool)
        .await?;
    }
    Err(MBError::RecordingNotFound(_)) => {
        // Warning: No MusicBrainz metadata
        log::warn!("Recording {} not found in MusicBrainz", mbid);
    }
    Err(MBError::RateLimitExceeded) => {
        // Critical: Wait and retry
        tokio::time::sleep(Duration::from_secs(2)).await;
        // Retry logic here
    }
    Err(e) => {
        return Err(e.into());
    }
}
```

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new();

        let start = Instant::now();
        limiter.wait().await;
        limiter.wait().await;
        limiter.wait().await;
        let elapsed = start.elapsed();

        // Should take at least 2 seconds (3 requests at 1/s)
        assert!(elapsed >= Duration::from_millis(2000));
        assert!(elapsed < Duration::from_millis(2500));
    }

    #[test]
    fn test_recording_deserialization() {
        let json = r#"{
            "id": "test-mbid",
            "title": "Test Song",
            "length": 180000,
            "artist-credit": [
                {
                    "name": "Test Artist",
                    "artist": {
                        "id": "artist-mbid",
                        "name": "Test Artist",
                        "sort-name": "Artist, Test"
                    }
                }
            ]
        }"#;

        let recording: MBRecording = serde_json::from_str(json).unwrap();
        assert_eq!(recording.title, "Test Song");
        assert_eq!(recording.artist_credit[0].artist.name, "Test Artist");
    }
}
```

### Integration Tests (Mock Server)

```rust
#[tokio::test]
async fn test_lookup_recording_with_mock() {
    let mock_server = mockito::Server::new();
    let mock = mock_server.mock("GET", "/recording/test-mbid")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(include_str!("../fixtures/mb_recording_response.json"))
        .create();

    // Test lookup with mock
    // ...

    mock.assert();
}
```

---

**Document Version:** 1.0
**Last Updated:** 2025-10-27
**Status:** Implementation specification (ready for coding)
