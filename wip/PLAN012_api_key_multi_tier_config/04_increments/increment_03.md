# Increment 3: wkmp-ai Database Accessors

**Estimated Effort:** 2-3 hours
**Dependencies:** None (uses existing settings table)
**Risk:** LOW

---

## Objectives

Implement get/set_acoustid_api_key() functions following wkmp-ap/src/db/settings.rs pattern.

---

## Requirements Addressed

- [APIK-DB-010] - Settings table key-value pattern
- [APIK-DB-020] - Generic get/set_setting()
- [APIK-DB-030] - No schema changes
- [APIK-ACID-040] - Database storage for acoustid_api_key
- [APIK-ARCH-030] - Module provides database accessors

---

## Deliverables

### Code Changes

**File: wkmp-ai/src/db/settings.rs** (new or extend existing)

```rust
use sqlx::{Pool, Sqlite};
use wkmp_common::{Error, Result};

/// Get AcoustID API key from database
///
/// **Traceability:** APIK-DB-010, APIK-ACID-040
///
/// **Returns:** Some(key) if exists, None if not set
pub async fn get_acoustid_api_key(db: &Pool<Sqlite>) -> Result<Option<String>> {
    get_setting::<String>(db, "acoustid_api_key").await
}

/// Set AcoustID API key in database
///
/// **Traceability:** APIK-DB-020, APIK-ACID-040
pub async fn set_acoustid_api_key(db: &Pool<Sqlite>, key: String) -> Result<()> {
    set_setting(db, "acoustid_api_key", key).await
}

/// Generic setting getter (internal)
async fn get_setting<T>(db: &Pool<Sqlite>, key: &str) -> Result<Option<T>>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM settings WHERE key = ?"
    )
    .bind(key)
    .fetch_optional(db)
    .await
    .map_err(|e| Error::Database(format!("Get setting failed: {}", e)))?;

    match row {
        Some((value,)) => {
            let parsed = value.parse::<T>()
                .map_err(|e| Error::Config(format!("Parse setting failed: {}", e)))?;
            Ok(Some(parsed))
        }
        None => Ok(None),
    }
}

/// Generic setting setter (internal)
async fn set_setting<T>(db: &Pool<Sqlite>, key: &str, value: T) -> Result<()>
where
    T: std::fmt::Display,
{
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value"
    )
    .bind(key)
    .bind(value.to_string())
    .execute(db)
    .await
    .map_err(|e| Error::Database(format!("Set setting failed: {}", e)))?;

    Ok(())
}
```

---

### Unit Tests

**File: wkmp-ai/tests/unit/db_settings_tests.rs** (new)

```rust
use wkmp_ai::db::settings::{get_acoustid_api_key, set_acoustid_api_key};
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn test_get_acoustid_api_key_returns_value() {
    // tc_u_db_001
    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();

    // Run migrations
    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .unwrap();

    // Set key
    set_acoustid_api_key(&pool, "test-key-123".to_string())
        .await
        .unwrap();

    // Get key
    let key = get_acoustid_api_key(&pool).await.unwrap();
    assert_eq!(key, Some("test-key-123".to_string()));
}

#[tokio::test]
async fn test_set_acoustid_api_key_writes_value() {
    // tc_u_db_002
    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .unwrap();

    // Set key
    set_acoustid_api_key(&pool, "new-key-456".to_string())
        .await
        .unwrap();

    // Verify by direct query
    let row: (String,) = sqlx::query_as(
        "SELECT value FROM settings WHERE key = 'acoustid_api_key'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.0, "new-key-456");
}

#[tokio::test]
async fn test_get_acoustid_api_key_returns_none_when_missing() {
    // tc_u_db_001 (edge case)
    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .unwrap();

    // Get key (not set)
    let key = get_acoustid_api_key(&pool).await.unwrap();
    assert_eq!(key, None);
}

#[tokio::test]
async fn test_set_acoustid_api_key_updates_existing() {
    // tc_u_db_002 (update case)
    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .unwrap();

    // Set initial key
    set_acoustid_api_key(&pool, "old-key".to_string())
        .await
        .unwrap();

    // Update key
    set_acoustid_api_key(&pool, "new-key".to_string())
        .await
        .unwrap();

    // Verify updated
    let key = get_acoustid_api_key(&pool).await.unwrap();
    assert_eq!(key, Some("new-key".to_string()));
}
```

---

## Verification Steps

1. Unit tests pass (4 tests)
2. Verify database write (check settings table directly)
3. Verify upsert behavior (INSERT or UPDATE)
4. Verify None returned when key not set

---

## Acceptance Criteria

- [ ] get_acoustid_api_key() implemented
- [ ] set_acoustid_api_key() implemented
- [ ] Generic get/set_setting() helpers implemented
- [ ] All unit tests pass (4 tests)
- [ ] Follows wkmp-ap/src/db/settings.rs pattern

---

## Test Traceability

- tc_u_db_001: Get returns value
- tc_u_db_002: Set writes value

---

## Implementation Notes

**Pattern Consistency:**
- Matches wkmp-ap/src/db/settings.rs (get_volume, set_volume)
- Uses generic get/set_setting() helpers (reusable for future settings)
- UPSERT behavior (INSERT or UPDATE)

**Error Handling:**
- Database errors propagated to caller
- Parse errors for invalid data types

---

## Rollback Plan

If increment fails:
- Remove wkmp-ai/src/db/settings.rs
- No downstream impact (no other modules depend on this yet)
