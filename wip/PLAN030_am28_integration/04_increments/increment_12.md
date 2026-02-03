# Increment 12: MusicBrainz Integration

**Estimated Effort:** 4 hours
**Dependencies:** Increment 1, 6
**Deliverables:** services/musicbrainz/ enhancements

---

## Objective

Extend MusicBrainz client with comprehensive search and response caching for album matching.

---

## Source Files

| am28 File | Lines | Action |
|-----------|-------|--------|
| musicbrainz/api.rs | ~400 | Integrate into existing client |
| musicbrainz/cache.rs | ~200 | Add caching layer |

---

## Tasks

### 12.1 Extend MusicBrainzClient

Add to existing `services/musicbrainz_client.rs`:

```rust
use crate::matching::types::*;

impl MusicBrainzClient {
    /// Comprehensive album search with track details
    ///
    /// # Arguments
    /// * `artist` - Artist name
    /// * `album` - Album/release title
    /// * `limit` - Maximum releases to return
    ///
    /// # Returns
    /// Vector of releases with full track details
    pub async fn comprehensive_search(
        &self,
        artist: &str,
        album: &str,
        limit: usize,
    ) -> Result<Vec<MBReleaseDetails>, MusicBrainzError> {
        // Check cache first
        let cache_key = format!("search:{}:{}", artist.to_lowercase(), album.to_lowercase());
        if let Some(cached) = self.cache.get(&cache_key).await {
            return Ok(cached);
        }

        // Search for releases
        let query = format!(
            "artist:\"{}\" AND release:\"{}\"",
            escape_lucene(artist),
            escape_lucene(album)
        );

        let search_results = self.search_releases(&query, limit).await?;

        // Fetch full details for each release
        let mut details = Vec::new();
        for release in search_results {
            if let Ok(full) = self.get_release_details(&release.id).await {
                details.push(full);
            }
        }

        // Cache results
        self.cache.set(&cache_key, details.clone()).await;

        Ok(details)
    }

    /// Get full release details including tracks
    pub async fn get_release_details(
        &self,
        release_mbid: &str,
    ) -> Result<MBReleaseDetails, MusicBrainzError> {
        // Check cache first
        let cache_key = format!("release:{}", release_mbid);
        if let Some(cached) = self.cache.get(&cache_key).await {
            return Ok(cached);
        }

        // Rate limit
        self.rate_limiter.wait().await;

        let url = format!(
            "{}/release/{}?inc=recordings+media&fmt=json",
            self.base_url,
            release_mbid
        );

        let response = self.client.get(&url)
            .header("User-Agent", &self.user_agent)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(MusicBrainzError::ApiError(response.status().as_u16()));
        }

        let details: MBReleaseDetails = response.json().await?;

        // Cache result
        self.cache.set(&cache_key, details.clone()).await;

        Ok(details)
    }
}

/// Escape special Lucene query characters
fn escape_lucene(s: &str) -> String {
    let special_chars = [
        '+', '-', '&', '|', '!', '(', ')', '{', '}',
        '[', ']', '^', '"', '~', '*', '?', ':', '\\', '/'
    ];
    let mut result = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        if special_chars.contains(&c) {
            result.push('\\');
        }
        result.push(c);
    }
    result
}
```

### 12.2 Add Response Caching

Create `services/musicbrainz/cache.rs`:

```rust
//! MusicBrainz Response Cache
//!
//! File-based caching for API responses to reduce rate limiting impact.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use tokio::fs;

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Cache directory
    pub cache_dir: PathBuf,
    /// Cache entry TTL
    pub ttl: Duration,
    /// Enable caching
    pub enabled: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_dir: PathBuf::from(".cache/musicbrainz"),
            ttl: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
            enabled: true,
        }
    }
}

/// Cache entry wrapper
#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry<T> {
    data: T,
    timestamp: u64,
}

/// MusicBrainz response cache
pub struct MusicBrainzCache {
    config: CacheConfig,
}

impl MusicBrainzCache {
    pub fn new(config: CacheConfig) -> Self {
        Self { config }
    }

    /// Get cached value
    pub async fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        if !self.config.enabled {
            return None;
        }

        let path = self.cache_path(key);
        let content = fs::read_to_string(&path).await.ok()?;
        let entry: CacheEntry<T> = serde_json::from_str(&content).ok()?;

        // Check TTL
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if now - entry.timestamp > self.config.ttl.as_secs() {
            // Expired
            let _ = fs::remove_file(&path).await;
            return None;
        }

        Some(entry.data)
    }

    /// Set cached value
    pub async fn set<T: Serialize>(&self, key: &str, value: T) {
        if !self.config.enabled {
            return;
        }

        let entry = CacheEntry {
            data: value,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let path = self.cache_path(key);
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent).await;
        }

        if let Ok(json) = serde_json::to_string(&entry) {
            let _ = fs::write(&path, json).await;
        }
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        // Implementation for tracking hits/misses
        CacheStats::default()
    }

    fn cache_path(&self, key: &str) -> PathBuf {
        let hash = blake3::hash(key.as_bytes());
        let hex = hash.to_hex();
        self.config.cache_dir.join(&hex[..2]).join(&hex[2..])
    }
}

/// Cache statistics
#[derive(Debug, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub entries: u64,
}
```

---

## Tests

| Test ID | Description | Pass Criteria |
|---------|-------------|---------------|
| TC-U-012-01 | Comprehensive search query | Correct Lucene query |
| TC-U-012-02 | Lucene escaping | Special chars escaped |
| TC-U-012-03 | Cache hit | Returns cached value |
| TC-U-012-04 | Cache miss | Fetches from API |
| TC-U-012-05 | Cache TTL expiry | Expired entries ignored |
| TC-I-012-01 | Live API search | Returns real releases |

---

## Acceptance Criteria

- [ ] MusicBrainz client extended
- [ ] Comprehensive search working
- [ ] Response caching functional
- [ ] Cache TTL working
- [ ] All 6 tests pass
