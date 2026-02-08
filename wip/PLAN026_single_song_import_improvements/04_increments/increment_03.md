# Increment 3: Recording Cache

**Estimated Effort:** 2 hours
**Dependencies:** Increment 2 (recording_matcher)
**Tests:** TC-I-INT-030-01

---

## Objective

Add database caching for MusicBrainz recording lookups to prevent redundant API calls.

---

## Deliverables

### 1. Database Migration

Add to migrations or use SPEC031 data-driven schema:

```sql
CREATE TABLE IF NOT EXISTS recording_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    artist_normalized TEXT NOT NULL,
    title_normalized TEXT NOT NULL,
    mbid TEXT,  -- NULL if not found
    confidence REAL,
    cached_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT NOT NULL,
    UNIQUE(artist_normalized, title_normalized)
);

CREATE INDEX idx_recording_cache_lookup
ON recording_cache(artist_normalized, title_normalized);
```

### 2. New File: `src/db/recording_cache.rs`

```rust
//! Recording cache database operations
//!
//! Caches MusicBrainz recording lookup results to prevent redundant API calls.

use sqlx::SqlitePool;
use chrono::{DateTime, Utc, Duration};

const CACHE_TTL_DAYS: i64 = 7;

#[derive(Debug, Clone)]
pub struct CachedRecording {
    pub mbid: Option<String>,
    pub confidence: Option<f64>,
    pub cached_at: DateTime<Utc>,
}

/// Get cached recording by normalized artist/title
pub async fn get_cached(
    pool: &SqlitePool,
    artist_normalized: &str,
    title_normalized: &str,
) -> Result<Option<CachedRecording>, sqlx::Error> {
    let row: Option<(Option<String>, Option<f64>, String)> = sqlx::query_as(
        "SELECT mbid, confidence, cached_at FROM recording_cache
         WHERE artist_normalized = ? AND title_normalized = ?
         AND datetime(expires_at) > datetime('now')"
    )
    .bind(artist_normalized)
    .bind(title_normalized)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(mbid, confidence, cached_at_str)| {
        CachedRecording {
            mbid,
            confidence,
            cached_at: cached_at_str.parse().unwrap_or_else(|_| Utc::now()),
        }
    }))
}

/// Cache recording lookup result
pub async fn cache_result(
    pool: &SqlitePool,
    artist_normalized: &str,
    title_normalized: &str,
    mbid: Option<&str>,
    confidence: Option<f64>,
) -> Result<(), sqlx::Error> {
    let expires_at = Utc::now() + Duration::days(CACHE_TTL_DAYS);

    sqlx::query(
        "INSERT INTO recording_cache (artist_normalized, title_normalized, mbid, confidence, expires_at)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(artist_normalized, title_normalized) DO UPDATE SET
            mbid = excluded.mbid,
            confidence = excluded.confidence,
            cached_at = datetime('now'),
            expires_at = excluded.expires_at"
    )
    .bind(artist_normalized)
    .bind(title_normalized)
    .bind(mbid)
    .bind(confidence)
    .bind(expires_at.to_rfc3339())
    .execute(pool)
    .await?;

    Ok(())
}
```

### 3. Update RecordingMatcher to use cache

```rust
impl RecordingMatcher {
    pub async fn search_with_cache(
        &self,
        artist: &str,
        title: &str,
        duration_secs: Option<f64>,
        pool: &SqlitePool,
    ) -> Result<Vec<RecordingCandidate>, MBError> {
        let artist_norm = normalize_for_comparison(artist);
        let title_norm = normalize_for_comparison(title);

        // Check cache first
        if let Some(cached) = get_cached(pool, &artist_norm, &title_norm).await.ok().flatten() {
            if let Some(mbid) = cached.mbid {
                return Ok(vec![RecordingCandidate {
                    mbid,
                    title: title.to_string(),
                    artist: artist.to_string(),
                    duration: duration_secs,
                    similarity: cached.confidence.unwrap_or(1.0),
                }]);
            } else {
                return Ok(vec![]); // Cached "not found"
            }
        }

        // Query MusicBrainz
        let candidates = self.search(artist, title, duration_secs).await?;

        // Cache result
        let (mbid, confidence) = candidates.first()
            .map(|c| (Some(c.mbid.as_str()), Some(c.similarity)))
            .unwrap_or((None, None));

        let _ = cache_result(pool, &artist_norm, &title_norm, mbid, confidence).await;

        Ok(candidates)
    }
}
```

---

## Files Modified

| File | Action | Lines |
|------|--------|-------|
| `src/db/recording_cache.rs` | Create | ~80 |
| `src/db/mod.rs` | Modify | +1 |
| `src/services/recording_matcher.rs` | Modify | +30 |

---

## Verification

- [ ] TC-I-INT-030-01 passes (cache prevents redundant queries)
- [ ] Cache hit returns immediately
- [ ] Cache miss triggers API call and caches result
- [ ] Expired cache entries are not returned
- [ ] `cargo test recording_cache` passes

---

## Success Criteria

- Cache table created on startup
- Cache hit avoids API call
- Cache miss populates cache
- TTL (7 days) enforced
