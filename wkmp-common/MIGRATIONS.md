# Database Schema Migrations

## Overview

WKMP uses a versioned schema migration system to allow seamless database upgrades without requiring manual deletion or data loss. When you upgrade WKMP and the database schema has changed, migrations run automatically on the next startup.

**Location:** [`wkmp-common/src/db/migrations.rs`](src/db/migrations.rs)

---

## Migration Principles

### 1. Never Modify Existing Migrations

Once a migration is released, **never change it**. Users may have already applied it to their databases. Instead:

- ✅ Add a new migration for schema changes
- ❌ Don't modify `migrate_v1()`, `migrate_v2()`, etc.

### 2. Always Use ALTER TABLE (When Possible)

Prefer `ALTER TABLE` over `DROP TABLE` + `CREATE TABLE` to preserve user data:

```rust
// ✅ Good - Preserves data
sqlx::query("ALTER TABLE passages ADD COLUMN new_field TEXT")
    .execute(pool)
    .await?;

// ❌ Bad - Loses data
sqlx::query("DROP TABLE passages").execute(pool).await?;
sqlx::query("CREATE TABLE passages (...)").execute(pool).await?;
```

### 3. Make Migrations Idempotent

Each migration should be safe to run multiple times. Use checks to avoid errors:

```rust
// Check if column already exists
let has_column: i64 = sqlx::query_scalar(
    "SELECT COUNT(*) FROM pragma_table_info('table_name') WHERE name = 'new_column'"
)
.fetch_one(pool)
.await?;

if has_column == 0 {
    sqlx::query("ALTER TABLE table_name ADD COLUMN new_column TEXT")
        .execute(pool)
        .await?;
}
```

### 4. Test Migrations on Old Schema

Before releasing, test migrations on databases with the old schema:

1. Create database with previous version
2. Add test data
3. Run new version with migration
4. Verify data preserved and schema updated

---

## Adding a New Migration

### Step 1: Increment Schema Version

In [`migrations.rs`](src/db/migrations.rs), update `CURRENT_SCHEMA_VERSION`:

```rust
const CURRENT_SCHEMA_VERSION: i32 = 2; // Was 1, now 2
```

### Step 2: Add Migration Function

Create a new migration function following the naming pattern `migrate_vN()`:

```rust
/// Migration v2: Add example_column to songs table
async fn migrate_v2(pool: &SqlitePool) -> Result<()> {
    info!("Running migration v2: Add example_column to songs");

    // Check if songs table exists
    let table_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM sqlite_master
            WHERE type='table' AND name='songs'
        )
        "#
    )
    .fetch_one(pool)
    .await?;

    if !table_exists {
        info!("  Songs table doesn't exist yet - skipping migration");
        return Ok(());
    }

    // Check if column already exists (idempotency)
    let has_column: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('songs') WHERE name = 'example_column'"
    )
    .fetch_one(pool)
    .await?;

    if has_column > 0 {
        info!("  example_column already exists - skipping");
        return Ok(());
    }

    // Add the column
    sqlx::query("ALTER TABLE songs ADD COLUMN example_column TEXT")
        .execute(pool)
        .await?;

    info!("  ✓ Added example_column to songs table");
    Ok(())
}
```

### Step 3: Register Migration in run_migrations()

Add the new migration to the sequence in `run_migrations()`:

```rust
pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    // ... existing code ...

    // Run migrations sequentially
    if current_version < 1 {
        migrate_v1(pool).await?;
        set_schema_version(pool, 1).await?;
        info!("✓ Migration v1 completed");
    }

    // NEW: Add your migration here
    if current_version < 2 {
        migrate_v2(pool).await?;
        set_schema_version(pool, 2).await?;
        info!("✓ Migration v2 completed");
    }

    // ... rest of function ...
}
```

### Step 4: Add Tests

Create tests for your migration in the `#[cfg(test)]` section:

```rust
#[tokio::test]
async fn test_migrate_v2_adds_column() {
    let pool = setup_test_db().await;

    // Create songs table WITHOUT example_column
    sqlx::query(
        r#"
        CREATE TABLE songs (
            guid TEXT PRIMARY KEY,
            recording_mbid TEXT NOT NULL
        )
        "#
    )
    .execute(&pool)
    .await
    .unwrap();

    // Run migration
    migrate_v2(&pool).await.unwrap();

    // Verify column was added
    let has_column: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('songs') WHERE name = 'example_column'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(has_column, 1);
}

#[tokio::test]
async fn test_migrate_v2_idempotent() {
    let pool = setup_test_db().await;

    // Create songs table WITH example_column
    sqlx::query(
        r#"
        CREATE TABLE songs (
            guid TEXT PRIMARY KEY,
            recording_mbid TEXT NOT NULL,
            example_column TEXT
        )
        "#
    )
    .execute(&pool)
    .await
    .unwrap();

    // Run migration twice - should not fail
    migrate_v2(&pool).await.unwrap();
    migrate_v2(&pool).await.unwrap();

    // Verify column exists only once
    let column_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('songs') WHERE name = 'example_column'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(column_count, 1);
}
```

### Step 5: Run Tests

```bash
cargo test -p wkmp-common db::migrations
```

---

## SQLite Schema Inspection

Useful queries for checking schema state:

### Check if Table Exists

```sql
SELECT EXISTS(
    SELECT 1 FROM sqlite_master
    WHERE type='table' AND name='table_name'
);
```

### Check if Column Exists

```sql
SELECT COUNT(*) FROM pragma_table_info('table_name')
WHERE name = 'column_name';
```

### List All Tables

```sql
SELECT name FROM sqlite_master
WHERE type='table' ORDER BY name;
```

### List Columns in Table

```sql
SELECT * FROM pragma_table_info('table_name');
```

---

## Migration History

### v1 (2025-11-01)

**Added:** `import_metadata` column to `passages` table

**Rationale:** The `passages` table was initially created without an `import_metadata` column, which is needed by wkmp-ai to track import source metadata.

**SQL:**
```sql
ALTER TABLE passages ADD COLUMN import_metadata TEXT;
```

**Breaking:** No (additive change only)

---

## Troubleshooting

### Migration Fails with "column already exists"

**Cause:** Migration is not idempotent

**Fix:** Add column existence check before ALTER TABLE:

```rust
let has_column: i64 = sqlx::query_scalar(
    "SELECT COUNT(*) FROM pragma_table_info('table_name') WHERE name = 'column_name'"
)
.fetch_one(pool)
.await?;

if has_column == 0 {
    // Only add if doesn't exist
    sqlx::query("ALTER TABLE table_name ADD COLUMN column_name TEXT")
        .execute(pool)
        .await?;
}
```

### Database Version Newer Than Code

**Symptom:** Log shows "Database schema version (X) is newer than code version (Y)"

**Cause:** User downgraded WKMP to older version

**Action:** This is a warning, not an error. The system continues with caution. If incompatibilities arise, user should upgrade to latest version.

### Migration Hangs or Fails

**Debug Steps:**

1. Check logs for specific error message
2. Verify database file permissions
3. Ensure no other process has database locked
4. Try running migration tests: `cargo test -p wkmp-common db::migrations`
5. If all else fails, backup database and delete it - WKMP will recreate with latest schema

---

## Best Practices

### DO

- ✅ Test migrations on databases with old schema
- ✅ Document breaking changes clearly
- ✅ Use transactions for multi-step migrations
- ✅ Log migration progress (info level)
- ✅ Make migrations idempotent

### DON'T

- ❌ Modify existing migration functions
- ❌ Drop tables unless absolutely necessary
- ❌ Skip version numbers (always sequential)
- ❌ Assume data format without checking
- ❌ Forget to update CURRENT_SCHEMA_VERSION

---

## References

- SQLite ALTER TABLE: https://www.sqlite.org/lang_altertable.html
- SQLite PRAGMA: https://www.sqlite.org/pragma.html
- WKMP Database Schema: [`docs/IMPL001-database_schema.md`](../../docs/IMPL001-database_schema.md)
