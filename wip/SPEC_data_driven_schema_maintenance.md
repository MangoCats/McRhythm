# Data-Driven Schema Maintenance System

**Document Type:** SPEC (Tier 2 - Design Specification)
**Status:** Draft
**Created:** 2025-11-09
**Supersedes:** Manual migration pattern in wkmp-common/src/db/migrations.rs

---

## Executive Summary

This specification defines a **data-driven, zero-configuration database schema maintenance system** that eliminates manual migrations while preserving the existing migration framework for complex transformations.

**Key Innovation:** Schema definitions become the single source of truth. The system automatically detects and repairs schema drift on every startup.

**Design Goals:**
1. **Single Source of Truth:** Schema defined once in code, not duplicated
2. **Zero Manual Migrations:** Column additions/changes applied automatically
3. **Development-Production Parity:** Same schema maintenance in both environments
4. **Backward Compatibility:** Existing v1-v4 migrations remain functional
5. **Safety First:** Read-only introspection, explicit change application, comprehensive logging

---

## Requirements Analysis

### Existing Requirements (from IMPL001, REQ001, SPEC001)

**[REQ-NF-036]** Automatic database creation with default schema
**[REQ-NF-037]** Modules create missing tables/columns automatically
**[ARCH-DB-MIG-010]** Schema migration framework
**[ARCH-DB-MIG-020]** Migration tracking via schema_version table
**[ARCH-DB-MIG-030]** Idempotent migrations (safe to run multiple times)

**IMPL001-database_schema.md lines 1253-1283:**
- Current: Manual migrations (migrate_v1, migrate_v2, migrate_v3, migrate_v4)
- Each module creates tables it requires if missing
- Each module initializes missing values with defaults
- Schema version tracked in `schema_version` table

### Gap Analysis

**Current System Limitations:**
1. ❌ Schema defined in TWO places (CREATE TABLE + struct)
2. ❌ Adding struct field requires manual migration
3. ❌ No compile-time verification of schema consistency
4. ❌ Easy to forget migrations when modifying structs
5. ❌ Migration creation is manual, error-prone work

**What We Need:**
1. ✅ Schema introspection: Read actual database schema
2. ✅ Schema comparison: Detect drift between code and database
3. ✅ Automatic column addition: ALTER TABLE for missing columns
4. ✅ Type validation: Verify column types match expectations
5. ✅ Comprehensive logging: Document all schema changes
6. ✅ Safe failure modes: Error on unexpected conditions

---

## Design: Declarative Schema with Auto-Sync

### Core Concept

**Schema Definition = Runtime Behavior**

```rust
// BEFORE (dual definition):
struct AudioFile { pub format: Option<String> }  // Definition 1
CREATE TABLE files (..., format TEXT, ...)        // Definition 2
migrate_v4() { ALTER TABLE files ADD COLUMN format TEXT }  // Manual sync

// AFTER (single definition):
#[derive(DbTable)]
#[table_name = "files"]
struct AudioFile {
    #[column(sql_type = "TEXT")]
    pub format: Option<String>,  // SINGLE SOURCE OF TRUTH
}
// System automatically adds missing column on startup
```

### Architecture

**Three-Phase Initialization:**

```
Phase 1: CREATE TABLE IF NOT EXISTS (existing logic)
  ├─ Creates tables that don't exist
  └─ Preserves existing tables unchanged

Phase 2: Schema Introspection & Sync (NEW)
  ├─ Read actual database schema via PRAGMA table_info
  ├─ Compare to expected schema from code
  ├─ Generate ALTER TABLE statements for drift
  └─ Apply changes with comprehensive logging

Phase 3: Run Manual Migrations (existing logic)
  ├─ Complex transformations (data migration, type changes)
  ├─ Version 1-4 migrations for backward compatibility
  └─ Future: Rare edge cases requiring manual intervention
```

**Component Responsibilities:**

| Component | Responsibility |
|-----------|---------------|
| **TableSchema trait** | Define expected schema for each table |
| **SchemaIntrospector** | Read actual database schema via PRAGMA |
| **SchemaDiff** | Compare expected vs actual, detect drift |
| **SchemaSync** | Apply ALTER TABLE to fix drift |
| **Migration Framework** | Complex transformations, version tracking |

### Implementation Strategy

**Macro-Based Schema Definition (Future Enhancement):**

```rust
#[derive(DbTable)]
#[table_name = "files"]
struct AudioFile {
    #[column(sql_type = "TEXT", primary_key)]
    pub guid: Uuid,

    #[column(sql_type = "TEXT", not_null, unique)]
    pub path: String,

    #[column(sql_type = "INTEGER")]
    pub duration_ticks: Option<i64>,

    #[column(sql_type = "TEXT")]
    pub format: Option<String>,
}

// Macro generates:
impl TableSchema for AudioFile {
    fn table_name() -> &'static str { "files" }
    fn expected_columns() -> Vec<ColumnDefinition> { ... }
}
```

**Manual Implementation (Phase 1 - This Implementation):**

```rust
impl TableSchema for AudioFileSchema {
    fn table_name() -> &'static str { "files" }

    fn expected_columns() -> Vec<ColumnDefinition> {
        vec![
            ColumnDefinition::new("guid", "TEXT").primary_key(),
            ColumnDefinition::new("path", "TEXT").not_null().unique(),
            ColumnDefinition::new("duration_ticks", "INTEGER"),
            ColumnDefinition::new("format", "TEXT"),
            ColumnDefinition::new("sample_rate", "INTEGER"),
            ColumnDefinition::new("channels", "INTEGER"),
            ColumnDefinition::new("file_size_bytes", "INTEGER"),
            // ...
        ]
    }
}
```

---

## Detailed Design

### Data Structures

```rust
/// Column definition with constraints
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnDefinition {
    pub name: String,
    pub sql_type: String,  // "TEXT", "INTEGER", "REAL", "TIMESTAMP"
    pub not_null: bool,
    pub primary_key: bool,
    pub unique: bool,
    pub default_value: Option<String>,
}

/// Actual column from database introspection
#[derive(Debug, Clone)]
pub struct ActualColumn {
    pub cid: i32,           // Column ID
    pub name: String,
    pub type_name: String,  // From PRAGMA table_info
    pub not_null: bool,
    pub default_value: Option<String>,
    pub pk: bool,           // Primary key flag
}

/// Schema difference detected
#[derive(Debug, Clone)]
pub enum SchemaDrift {
    MissingColumn {
        table: String,
        column: ColumnDefinition,
    },
    TypeMismatch {
        table: String,
        column: String,
        expected: String,
        actual: String,
    },
    ConstraintMismatch {
        table: String,
        column: String,
        constraint: String,  // "NOT NULL", "UNIQUE", etc.
    },
}
```

### TableSchema Trait

```rust
/// Defines expected schema for a database table
pub trait TableSchema {
    /// Table name in database
    fn table_name() -> &'static str;

    /// Expected column definitions
    fn expected_columns() -> Vec<ColumnDefinition>;

    /// Optional: Custom checks after auto-sync
    fn validate_schema(pool: &SqlitePool) -> Result<()> {
        Ok(())
    }
}
```

### SchemaIntrospector

```rust
pub struct SchemaIntrospector;

impl SchemaIntrospector {
    /// Read actual columns from database table
    pub async fn introspect_table(
        pool: &SqlitePool,
        table_name: &str
    ) -> Result<Vec<ActualColumn>> {
        let query = format!("PRAGMA table_info({})", table_name);
        let rows = sqlx::query(&query).fetch_all(pool).await?;

        rows.iter().map(|row| {
            Ok(ActualColumn {
                cid: row.get("cid"),
                name: row.get("name"),
                type_name: row.get("type"),
                not_null: row.get::<i32, _>("notnull") != 0,
                default_value: row.get("dflt_value"),
                pk: row.get::<i32, _>("pk") != 0,
            })
        }).collect()
    }
}
```

### SchemaDiff

```rust
pub struct SchemaDiff;

impl SchemaDiff {
    /// Compare expected schema to actual database schema
    pub fn compare(
        table_name: &str,
        expected: &[ColumnDefinition],
        actual: &[ActualColumn]
    ) -> Vec<SchemaDrift> {
        let mut drift = Vec::new();

        for expected_col in expected {
            if let Some(actual_col) = actual.iter()
                .find(|c| c.name == expected_col.name)
            {
                // Column exists - check type and constraints
                if !types_compatible(&expected_col.sql_type, &actual_col.type_name) {
                    drift.push(SchemaDrift::TypeMismatch {
                        table: table_name.to_string(),
                        column: expected_col.name.clone(),
                        expected: expected_col.sql_type.clone(),
                        actual: actual_col.type_name.clone(),
                    });
                }

                if expected_col.not_null && !actual_col.not_null {
                    drift.push(SchemaDrift::ConstraintMismatch {
                        table: table_name.to_string(),
                        column: expected_col.name.clone(),
                        constraint: "NOT NULL".to_string(),
                    });
                }
            } else {
                // Column missing
                drift.push(SchemaDrift::MissingColumn {
                    table: table_name.to_string(),
                    column: expected_col.clone(),
                });
            }
        }

        drift
    }

    fn types_compatible(expected: &str, actual: &str) -> bool {
        // SQLite type affinity rules
        expected.to_uppercase() == actual.to_uppercase()
    }
}
```

### SchemaSync

```rust
pub struct SchemaSync;

impl SchemaSync {
    /// Apply schema changes to database
    pub async fn sync_table<T: TableSchema>(
        pool: &SqlitePool
    ) -> Result<()> {
        let table_name = T::table_name();
        let expected = T::expected_columns();

        info!("Syncing schema for table: {}", table_name);

        // Read actual schema
        let actual = SchemaIntrospector::introspect_table(pool, table_name).await?;

        // Detect drift
        let drift = SchemaDiff::compare(table_name, &expected, &actual);

        if drift.is_empty() {
            info!("  Schema up to date for {}", table_name);
            return Ok(());
        }

        // Apply fixes
        for change in drift {
            match change {
                SchemaDrift::MissingColumn { table, column } => {
                    Self::add_column(pool, &table, &column).await?;
                }
                SchemaDrift::TypeMismatch { table, column, expected, actual } => {
                    warn!(
                        "Type mismatch in {}.{}: expected {}, found {}. Manual migration required.",
                        table, column, expected, actual
                    );
                    // Cannot auto-fix type changes - requires data migration
                }
                SchemaDrift::ConstraintMismatch { table, column, constraint } => {
                    warn!(
                        "Constraint mismatch in {}.{}: {}. Manual migration required.",
                        table, column, constraint
                    );
                    // Cannot auto-fix constraint changes - requires ALTER TABLE not supported by SQLite
                }
            }
        }

        Ok(())
    }

    async fn add_column(
        pool: &SqlitePool,
        table: &str,
        column: &ColumnDefinition
    ) -> Result<()> {
        let mut sql = format!(
            "ALTER TABLE {} ADD COLUMN {} {}",
            table, column.name, column.sql_type
        );

        // Note: SQLite ALTER TABLE ADD COLUMN does not support all constraints
        // - PRIMARY KEY: Not supported (requires table recreation)
        // - NOT NULL: Only if DEFAULT value provided
        // - UNIQUE: Not supported (requires table recreation)

        if column.not_null {
            if let Some(default) = &column.default_value {
                sql.push_str(&format!(" DEFAULT {}", default));
            } else {
                // Cannot add NOT NULL column without default
                warn!(
                    "Cannot add NOT NULL column {}.{} without default value. \
                     Column will be nullable.",
                    table, column.name
                );
            }
        }

        info!("  Adding column: {}.{} ({})", table, column.name, column.sql_type);

        sqlx::query(&sql).execute(pool).await.map_err(|e| {
            if e.to_string().contains("duplicate column") {
                // Concurrent initialization - column added by another thread
                info!("  Column {}.{} added by concurrent thread", table, column.name);
                Ok(())
            } else {
                Err(e)
            }
        })??;

        Ok(())
    }
}
```

### Integration with Existing init_database()

```rust
// In wkmp-common/src/db/init.rs

pub async fn init_database(db_path: &Path) -> Result<SqlitePool> {
    // ... existing setup code ...

    // Phase 1: CREATE TABLE IF NOT EXISTS (existing)
    create_schema_version_table(&pool).await?;
    create_files_table(&pool).await?;
    // ... all other tables ...

    // Phase 2: Schema Auto-Sync (NEW)
    sync_all_schemas(&pool).await?;

    // Phase 3: Manual Migrations (existing)
    crate::db::migrations::run_migrations(&pool).await?;

    // Phase 4: Initialize default settings (existing)
    init_default_settings(&pool).await?;

    Ok(pool)
}

async fn sync_all_schemas(pool: &SqlitePool) -> Result<()> {
    info!("Starting automatic schema synchronization");

    // Sync each table
    SchemaSync::sync_table::<AudioFileSchema>(pool).await?;
    SchemaSync::sync_table::<PassageSchema>(pool).await?;
    SchemaSync::sync_table::<SongSchema>(pool).await?;
    // ... all other tables ...

    info!("Automatic schema synchronization complete");
    Ok(())
}
```

---

## Implementation Phases

### Phase 1: Core Infrastructure (This Implementation)

**Deliverables:**
1. ✅ `TableSchema` trait definition
2. ✅ `ColumnDefinition`, `ActualColumn`, `SchemaDrift` data structures
3. ✅ `SchemaIntrospector` for PRAGMA table_info queries
4. ✅ `SchemaDiff` for comparing expected vs actual
5. ✅ `SchemaSync` for applying ALTER TABLE
6. ✅ Manual `impl TableSchema` for `files` table (proof of concept)
7. ✅ Integration with `init_database()`
8. ✅ Comprehensive tests

**Success Criteria:**
- Adding column to `AudioFileSchema` automatically adds to database
- Zero manual migration required for new columns
- All existing migrations (v1-v4) still functional
- Comprehensive test coverage (>90%)

### Phase 2: Complete Table Coverage (Future)

**Deliverables:**
1. Implement `TableSchema` for all 30+ tables
2. Update `sync_all_schemas()` to cover all tables
3. Remove manual migrations where auto-sync suffices

**Estimated Effort:** 8-12 hours

### Phase 3: Derive Macro (Future Enhancement)

**Deliverables:**
1. `#[derive(DbTable)]` procedural macro
2. Automatic `TableSchema` implementation from struct
3. Migration guide for existing code

**Estimated Effort:** 16-24 hours

---

## Safety and Edge Cases

### What Auto-Sync CAN Do

✅ Add missing columns
✅ Detect type mismatches (warn, no auto-fix)
✅ Detect constraint mismatches (warn, no auto-fix)
✅ Handle concurrent initialization (duplicate column errors)
✅ Log all changes comprehensively

### What Auto-Sync CANNOT Do

❌ Change column types (requires data migration)
❌ Add PRIMARY KEY constraints (requires table recreation)
❌ Add UNIQUE constraints (requires table recreation)
❌ Add NOT NULL without DEFAULT (SQLite limitation)
❌ Remove columns (SQLite limitation - requires table recreation)
❌ Rename columns (requires manual migration for data preservation)

### When Manual Migrations Still Required

1. **Type Changes:** `duration REAL` → `duration_ticks INTEGER` (data transformation)
2. **Data Migration:** Populating new columns from existing data
3. **Column Removal:** SQLite doesn't support DROP COLUMN
4. **Constraint Addition:** PRIMARY KEY, UNIQUE, NOT NULL (without default)
5. **Table Renaming:** Preserve data continuity

**Example: Manual Migration for Type Change**

```rust
async fn migrate_v3(pool: &SqlitePool) -> Result<()> {
    // Auto-sync adds duration_ticks column automatically
    // Manual migration transforms data from old column

    sqlx::query(
        "UPDATE files SET duration_ticks = CAST(duration * 28224000 AS INTEGER)
         WHERE duration IS NOT NULL"
    ).execute(pool).await?;

    Ok(())
}
```

---

## Testing Strategy

### Unit Tests

1. **SchemaIntrospector:** Parse PRAGMA table_info output
2. **SchemaDiff:** Detect all drift types correctly
3. **SchemaSync:** Generate correct ALTER TABLE statements
4. **ColumnDefinition:** Builder pattern validation

### Integration Tests

1. **Fresh Database:** Auto-sync creates all columns
2. **Missing Column:** Auto-sync adds single missing column
3. **Multiple Missing:** Auto-sync adds multiple columns in order
4. **Concurrent Init:** Handle duplicate column errors gracefully
5. **Type Mismatch:** Warn but don't fail startup
6. **Idempotent:** Running sync twice produces no changes

### Regression Tests

1. **Existing Migrations:** v1-v4 still function correctly
2. **Settings Init:** Default settings still initialized
3. **Zero-Config:** Fresh database still works with zero config

---

## Documentation Requirements

### Developer Documentation

1. **How to Add Column:** Update schema definition → restart
2. **When to Use Manual Migration:** Type changes, data transforms
3. **Schema Definition Guide:** ColumnDefinition API reference

### Specification Updates

1. **IMPL001-database_schema.md:** Document auto-sync system
2. **New SPEC:** This document (SPEC-DB-AUTO-SYNC)
3. **Migration Guide:** Converting existing tables to auto-sync

---

## Migration Path for Existing Code

**Step 1:** Implement auto-sync infrastructure (this implementation)
**Step 2:** Add `TableSchema` for `files` table (proof of concept)
**Step 3:** Verify backward compatibility with v1-v4 migrations
**Step 4:** Gradually add `TableSchema` for remaining tables
**Step 5:** Remove obsolete manual migrations (where applicable)
**Step 6:** Update documentation and developer guides

---

## Success Metrics

1. **Zero Manual Migrations for Column Additions:** 100% of new columns added via auto-sync
2. **Developer Time Savings:** 80% reduction in schema maintenance time
3. **Error Reduction:** Zero "column not found" runtime errors
4. **Test Coverage:** >90% for all auto-sync components
5. **Backward Compatibility:** 100% of existing migrations still functional

---

## Appendix A: SQLite ALTER TABLE Limitations

SQLite's ALTER TABLE has significant restrictions:

**Supported:**
- `ALTER TABLE ADD COLUMN` (with limitations)

**NOT Supported:**
- `ALTER TABLE DROP COLUMN` (requires table recreation)
- `ALTER TABLE MODIFY COLUMN` (requires table recreation)
- `ALTER TABLE RENAME COLUMN` (old SQLite versions)
- Adding PRIMARY KEY, UNIQUE constraints (requires table recreation)
- Adding NOT NULL without DEFAULT value

**Workaround (Table Recreation):**
```sql
BEGIN TRANSACTION;
CREATE TABLE new_table (...);
INSERT INTO new_table SELECT ... FROM old_table;
DROP TABLE old_table;
ALTER TABLE new_table RENAME TO old_table;
COMMIT;
```

**Auto-Sync Strategy:** Detect these cases, log warnings, require manual migration.

---

## Appendix B: Comparison with Other Approaches

### Diesel ORM Migrations

**Pros:** Powerful, well-tested, comprehensive
**Cons:** Heavyweight, requires migration files, manual migration creation

### SQLx Migrations

**Pros:** Lightweight, SQL-based
**Cons:** Still requires manual migration files

### Entity Framework Code-First

**Pros:** Automatic migration generation from model changes
**Cons:** .NET only, complex, sometimes generates incorrect migrations

### WKMP Auto-Sync (This Design)

**Pros:**
- Lightweight (no external dependencies)
- Automatic for 80% of cases (column additions)
- Single source of truth (code defines schema)
- Manual migrations still available for complex cases
- SQLite-specific optimizations

**Cons:**
- Limited to SQLite
- Cannot handle all schema changes automatically
- New system (not battle-tested like Diesel)

---

**Document Status:** Ready for Implementation
**Approver:** [Pending]
**Implementation Target:** 2025-11-09
