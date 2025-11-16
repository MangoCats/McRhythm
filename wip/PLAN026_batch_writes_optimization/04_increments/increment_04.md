# Increment 4: Batch Helper Functions

**Increment:** 4 of 13
**Phase:** Implementation (Phase 2)
**Estimated Effort:** 2-3 hours
**Prerequisites:** Increment 3 complete (baselines measured), Checkpoint A passed

---

## Objective

Create batch write helper functions in wkmp-ai/src/db/*.rs modules to support consolidated database writes.

**Success Criteria:**
- Batch insert functions created for songs, artists, albums, passages
- Unit tests pass (TC-U-BW-020-01, TC-U-BW-030-01)
- Functions follow passage_recorder.rs pattern
- Ready for use in phase refactors (Increments 5-8)

---

## Deliverables

### 1. batch_insert_songs() - wkmp-ai/src/db/songs.rs

**Signature:**
```rust
pub async fn batch_insert_songs(
    pool: &SqlitePool,
    songs: Vec<Song>,
) -> Result<HashMap<String, Uuid>> // mbid → guid mapping
```

**Implementation:**
- Accept vector of Song structs
- Execute batch INSERT...ON CONFLICT UPDATE in single transaction
- Return HashMap of mbid → guid for lookup
- Pattern: Pre-fetch existing songs by MBID (outside transaction)
- Pattern: Single transaction for all inserts

**Tests:** TC-U-BW-020-01 (single transaction verification)

---

### 2. batch_insert_artists() - wkmp-ai/src/db/artists.rs

**Signature:**
```rust
pub async fn batch_insert_artists(
    pool: &SqlitePool,
    artists: Vec<Artist>,
) -> Result<HashMap<String, Uuid>> // mbid → guid mapping
```

**Implementation:** Similar to batch_insert_songs

---

### 3. batch_insert_albums() - wkmp-ai/src/db/albums.rs

**Signature:**
```rust
pub async fn batch_insert_albums(
    pool: &SqlitePool,
    albums: Vec<Album>,
) -> Result<HashMap<String, Uuid>> // mbid → guid mapping
```

**Implementation:** Similar to batch_insert_songs

---

### 4. batch_insert_passages() - wkmp-ai/src/db/passages.rs

**Signature:**
```rust
pub async fn batch_insert_passages(
    pool: &SqlitePool,
    passages: Vec<Passage>,
) -> Result<Vec<Uuid>> // passage guids in order
```

**Implementation:**
- Accept vector of Passage structs
- Execute batch INSERT in single transaction
- Return vector of passage GUIDs in same order as input
- No pre-fetch needed (passages are new)

---

### 5. batch_link_passages_to_songs() - wkmp-ai/src/db/songs.rs

**Signature:**
```rust
pub async fn batch_link_passages_to_songs(
    pool: &SqlitePool,
    links: Vec<(Uuid, Uuid)>, // (passage_id, song_id) pairs
) -> Result<()>
```

**Implementation:**
- Accept vector of (passage_id, song_id) tuples
- Execute batch INSERT INTO passage_songs in single transaction
- Use ON CONFLICT DO NOTHING for idempotency

---

## Implementation Pattern (Reference)

**Based on passage_recorder.rs:103-145:**

```rust
pub async fn batch_insert_songs(
    pool: &SqlitePool,
    songs: Vec<Song>,
) -> Result<HashMap<String, Uuid>> {
    // Step 1: Pre-fetch existing songs by MBID (OUTSIDE transaction)
    let mbids: Vec<String> = songs.iter()
        .filter_map(|s| s.recording_mbid.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let existing: HashMap<String, Uuid> = if !mbids.is_empty() {
        let placeholders = mbids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!("SELECT recording_mbid, guid FROM songs WHERE recording_mbid IN ({})", placeholders);

        let rows = sqlx::query(&query)
            .bind_all(mbids)
            .fetch_all(pool)
            .await?;

        rows.into_iter()
            .map(|row| (row.get(0), row.get(1)))
            .collect()
    } else {
        HashMap::new()
    };

    // Step 2: Begin transaction (all lookups done, fast inserts only)
    let mut tx = begin_monitored(pool, "batch_insert_songs").await?;

    // Step 3: Batch insert songs
    let mut result = HashMap::new();
    for song in songs {
        // Check if already exists
        if let Some(guid) = song.recording_mbid.as_ref().and_then(|mbid| existing.get(mbid)) {
            result.insert(song.recording_mbid.clone().unwrap(), *guid);
            continue;
        }

        // Insert new song
        sqlx::query(
            r#"
            INSERT INTO songs (guid, title, recording_mbid, created_at, updated_at)
            VALUES (?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT(recording_mbid) DO UPDATE SET
                title = excluded.title,
                updated_at = CURRENT_TIMESTAMP
            "#
        )
        .bind(song.guid.to_string())
        .bind(&song.title)
        .bind(&song.recording_mbid)
        .execute(&mut *tx)
        .await?;

        if let Some(mbid) = song.recording_mbid {
            result.insert(mbid, song.guid);
        }
    }

    // Step 4: Commit transaction
    tx.commit().await?;

    Ok(result)
}
```

---

## Test Specifications

### TC-U-BW-020-01: Single Transaction Verification

**Purpose:** Verify batch insert uses only ONE transaction

**Test:**
```rust
#[tokio::test]
async fn test_batch_insert_songs_single_transaction() {
    let (pool, _temp) = setup_test_db().await;

    // Create test songs
    let songs = vec![
        Song { guid: Uuid::new_v4(), title: "Test 1".into(), recording_mbid: Some("mbid1".into()) },
        Song { guid: Uuid::new_v4(), title: "Test 2".into(), recording_mbid: Some("mbid2".into()) },
        Song { guid: Uuid::new_v4(), title: "Test 3".into(), recording_mbid: Some("mbid3".into()) },
    ];

    // Execute batch insert
    let result = batch_insert_songs(&pool, songs).await.unwrap();

    // Verify 3 songs inserted
    assert_eq!(result.len(), 3);

    // Verify transaction count (would need instrumentation or log parsing)
    // For now, verify behavior is correct
}
```

**Pass Criteria:** Function executes successfully, returns correct mappings

---

### TC-U-BW-030-01: Pre-fetch Reads Before Transaction

**Purpose:** Verify reads execute BEFORE transaction begins

**Test:**
```rust
#[tokio::test]
async fn test_batch_insert_prefetch_pattern() {
    let (pool, _temp) = setup_test_db().await;

    // Insert existing song
    sqlx::query("INSERT INTO songs (guid, title, recording_mbid) VALUES (?, ?, ?)")
        .bind(Uuid::new_v4().to_string())
        .bind("Existing Song")
        .bind("mbid_existing")
        .execute(&pool)
        .await.unwrap();

    // Create batch with mix of new and existing
    let songs = vec![
        Song { guid: Uuid::new_v4(), title: "Existing Song".into(), recording_mbid: Some("mbid_existing".into()) },
        Song { guid: Uuid::new_v4(), title: "New Song".into(), recording_mbid: Some("mbid_new".into()) },
    ];

    // Execute batch insert
    let result = batch_insert_songs(&pool, songs).await.unwrap();

    // Verify both in result (existing found via pre-fetch, new inserted)
    assert_eq!(result.len(), 2);
    assert!(result.contains_key("mbid_existing"));
    assert!(result.contains_key("mbid_new"));

    // Verify no duplicate created
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM songs WHERE recording_mbid = 'mbid_existing'")
        .fetch_one(&pool)
        .await.unwrap();
    assert_eq!(count, 1, "Pre-fetch should prevent duplicate insert");
}
```

**Pass Criteria:** Pre-fetch correctly identifies existing records, no duplicates created

---

## Files to Modify

### wkmp-ai/src/db/songs.rs
- Add `batch_insert_songs()` function (~50 lines)
- Add `batch_link_passages_to_songs()` function (~30 lines)
- Add unit tests (~80 lines)

### wkmp-ai/src/db/artists.rs
- Add `batch_insert_artists()` function (~50 lines)
- Add unit tests (~60 lines)

### wkmp-ai/src/db/albums.rs
- Add `batch_insert_albums()` function (~50 lines)
- Add unit tests (~60 lines)

### wkmp-ai/src/db/passages.rs
- Add `batch_insert_passages()` function (~40 lines)
- Add unit tests (~60 lines)

**Total New Code:** ~480 lines

---

## Dependencies

**Code Dependencies:**
- `wkmp_common::utils::db_retry::retry_on_lock` (optional, if wrapping batch operations)
- `wkmp-ai/src/utils/pool_monitor::begin_monitored` (transaction monitoring)
- Existing `save_song()`, `save_artist()`, etc. (reference implementation)

**Prerequisites:**
- Increment 3 complete (baselines measured)
- Checkpoint A passed
- Clean working directory (no uncommitted changes)

---

## Effort Breakdown

**Design:** 15 minutes
- Review passage_recorder.rs pattern
- Sketch function signatures

**Implementation:** 1.5-2 hours
- batch_insert_songs: 30 min
- batch_insert_artists: 20 min
- batch_insert_albums: 20 min
- batch_insert_passages: 20 min
- batch_link_passages_to_songs: 20 min

**Testing:** 30-45 minutes
- Write unit tests: 20 min
- Run and debug tests: 15-25 min

**Documentation:** 15 minutes
- Add doc comments to functions
- Update module-level docs

**Total:** 2-3 hours

---

## Verification Checklist

- [ ] `batch_insert_songs()` implemented and tested
- [ ] `batch_insert_artists()` implemented and tested
- [ ] `batch_insert_albums()` implemented and tested
- [ ] `batch_insert_passages()` implemented and tested
- [ ] `batch_link_passages_to_songs()` implemented and tested
- [ ] TC-U-BW-020-01 passes (single transaction verified)
- [ ] TC-U-BW-030-01 passes (pre-fetch pattern verified)
- [ ] cargo test passes (all existing tests still work)
- [ ] cargo clippy clean (no warnings)
- [ ] Git commit with clear message

---

## Success Criteria

**PASS if:**
- All 5 batch functions implemented
- Unit tests pass (TC-U-BW-020-01, TC-U-BW-030-01)
- No regression (cargo test passes)
- Code follows passage_recorder.rs pattern
- Functions ready for use in Increments 5-8

**FAIL if:**
- Any tests fail
- Regression in existing tests
- Functions don't follow pattern

---

## Next Increment

**Increment 5:** Fingerprinting Phase Refactor
- Use batch functions created in this increment
- Refactor phase_fingerprinting.rs to consolidate writes

---

## Rollback Plan

**If major issues:**
```bash
git checkout -- wkmp-ai/src/db/songs.rs
git checkout -- wkmp-ai/src/db/artists.rs
git checkout -- wkmp-ai/src/db/albums.rs
git checkout -- wkmp-ai/src/db/passages.rs
```

**Minimal viable rollback:** Keep functions, fix issues in next increment

---

## Sign-Off

**Increment Specified:** 2025-01-15
**Status:** Ready for implementation
**Depends On:** Increment 3, Checkpoint A
**Enables:** Increments 5-8 (phase refactors)
