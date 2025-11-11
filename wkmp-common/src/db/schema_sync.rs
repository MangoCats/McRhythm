//! Automatic Schema Synchronization
//!
//! Data-driven schema maintenance system that eliminates manual migrations for column additions.
//!
//! **Design:** Single source of truth - schema definitions in code automatically sync to database.
//!
//! **[ARCH-DB-SYNC-010]** Declarative schema definition
//! **[ARCH-DB-SYNC-020]** Automatic column addition
//! **[ARCH-DB-SYNC-030]** Schema introspection and drift detection
//!
//! # Architecture
//!
//! Three-phase initialization:
//! 1. **CREATE TABLE IF NOT EXISTS** - Create missing tables
//! 2. **Auto-Sync** - Add missing columns via ALTER TABLE (THIS MODULE)
//! 3. **Manual Migrations** - Complex transformations (existing migrations.rs)
//!
//! # Usage
//!
//! ```rust,ignore
//! // Define expected schema
//! pub struct FilesTableSchema;
//!
//! impl TableSchema for FilesTableSchema {
//!     fn table_name() -> &'static str { "files" }
//!
//!     fn expected_columns() -> Vec<ColumnDefinition> {
//!         vec![
//!             ColumnDefinition::new("guid", "TEXT").primary_key(),
//!             ColumnDefinition::new("path", "TEXT").not_null().unique(),
//!             ColumnDefinition::new("format", "TEXT"),  // ADD COLUMN HERE
//!             // ... system automatically adds missing column on startup
//!         ]
//!     }
//! }
//!
//! // Sync schema automatically
//! SchemaSync::sync_table::<FilesTableSchema>(&pool).await?;
//! ```

use crate::Result;
use sqlx::{Row, SqlitePool};
use tracing::{info, warn};

/// Column definition with SQL constraints
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnDefinition {
    /// Column name
    pub name: String,
    /// SQL type (e.g., "TEXT", "INTEGER", "REAL", "TIMESTAMP")
    pub sql_type: String,
    /// NOT NULL constraint
    pub not_null: bool,
    /// PRIMARY KEY constraint
    pub primary_key: bool,
    /// UNIQUE constraint
    pub unique: bool,
    /// DEFAULT value
    pub default_value: Option<String>,
}

impl ColumnDefinition {
    /// Create new column definition
    pub fn new(name: impl Into<String>, sql_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            sql_type: sql_type.into(),
            not_null: false,
            primary_key: false,
            unique: false,
            default_value: None,
        }
    }

    /// Mark column as PRIMARY KEY
    pub fn primary_key(mut self) -> Self {
        self.primary_key = true;
        self
    }

    /// Mark column as NOT NULL
    pub fn not_null(mut self) -> Self {
        self.not_null = true;
        self
    }

    /// Mark column as UNIQUE
    pub fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    /// Set DEFAULT value
    pub fn default(mut self, value: impl Into<String>) -> Self {
        self.default_value = Some(value.into());
        self
    }
}

/// Actual column from database introspection (PRAGMA table_info result)
#[derive(Debug, Clone)]
pub struct ActualColumn {
    /// Column ID (position in table)
    pub cid: i32,
    /// Column name
    pub name: String,
    /// SQL type from PRAGMA table_info
    pub type_name: String,
    /// NOT NULL constraint
    pub not_null: bool,
    /// DEFAULT value
    pub default_value: Option<String>,
    /// PRIMARY KEY flag (1 = yes, 0 = no)
    pub pk: bool,
}

/// Schema drift detected between expected and actual schema
#[derive(Debug, Clone)]
pub enum SchemaDrift {
    /// Column missing from database
    MissingColumn {
        table: String,
        column: ColumnDefinition,
    },
    /// Column type mismatch (cannot auto-fix - requires manual migration)
    TypeMismatch {
        table: String,
        column: String,
        expected: String,
        actual: String,
    },
    /// Constraint mismatch (cannot auto-fix - requires manual migration)
    ConstraintMismatch {
        table: String,
        column: String,
        constraint: String,  // "NOT NULL", "UNIQUE", "PRIMARY KEY"
    },
}

/// Defines expected schema for a database table
///
/// **[ARCH-DB-SYNC-010]** Declarative schema definition
pub trait TableSchema {
    /// Table name in database
    fn table_name() -> &'static str;

    /// Expected column definitions (order matters for new table creation)
    fn expected_columns() -> Vec<ColumnDefinition>;

    /// Optional: Custom validation after auto-sync
    fn validate_schema(_pool: &SqlitePool) -> Result<()> {
        Ok(())
    }
}

/// Schema introspection - read actual database schema
///
/// **[ARCH-DB-SYNC-030]** Schema introspection via PRAGMA table_info
pub struct SchemaIntrospector;

impl SchemaIntrospector {
    /// Read actual columns from database table using PRAGMA table_info
    ///
    /// Returns columns in database order (by cid)
    pub async fn introspect_table(
        pool: &SqlitePool,
        table_name: &str
    ) -> Result<Vec<ActualColumn>> {
        let query = format!("PRAGMA table_info({})", table_name);
        let rows = sqlx::query(&query).fetch_all(pool).await?;

        let mut columns: Vec<ActualColumn> = rows.iter().map(|row| {
            ActualColumn {
                cid: row.get("cid"),
                name: row.get("name"),
                type_name: row.get("type"),
                not_null: row.get::<i32, _>("notnull") != 0,
                default_value: row.get("dflt_value"),
                pk: row.get::<i32, _>("pk") != 0,
            }
        }).collect();

        // Sort by cid to ensure consistent order
        columns.sort_by_key(|c| c.cid);

        Ok(columns)
    }

    /// Check if table exists
    pub async fn table_exists(pool: &SqlitePool, table_name: &str) -> Result<bool> {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM sqlite_master
                WHERE type='table' AND name = ?
            )
            "#
        )
        .bind(table_name)
        .fetch_one(pool)
        .await?;

        Ok(exists)
    }
}

/// Schema comparison - detect drift between expected and actual
pub struct SchemaDiff;

impl SchemaDiff {
    /// Compare expected schema to actual database schema
    ///
    /// Returns list of schema drift items that need correction
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
                if !Self::types_compatible(&expected_col.sql_type, &actual_col.type_name) {
                    drift.push(SchemaDrift::TypeMismatch {
                        table: table_name.to_string(),
                        column: expected_col.name.clone(),
                        expected: expected_col.sql_type.clone(),
                        actual: actual_col.type_name.clone(),
                    });
                }

                // Check NOT NULL constraint
                if expected_col.not_null && !actual_col.not_null {
                    drift.push(SchemaDrift::ConstraintMismatch {
                        table: table_name.to_string(),
                        column: expected_col.name.clone(),
                        constraint: "NOT NULL".to_string(),
                    });
                }

                // Check PRIMARY KEY constraint
                if expected_col.primary_key && !actual_col.pk {
                    drift.push(SchemaDrift::ConstraintMismatch {
                        table: table_name.to_string(),
                        column: expected_col.name.clone(),
                        constraint: "PRIMARY KEY".to_string(),
                    });
                }
            } else {
                // Column missing from database
                drift.push(SchemaDrift::MissingColumn {
                    table: table_name.to_string(),
                    column: expected_col.clone(),
                });
            }
        }

        drift
    }

    /// Check if SQL types are compatible (SQLite type affinity rules)
    fn types_compatible(expected: &str, actual: &str) -> bool {
        let exp = expected.to_uppercase();
        let act = actual.to_uppercase();

        // Exact match
        if exp == act {
            return true;
        }

        // SQLite type affinity: INTEGER types
        if (exp.contains("INT") || exp == "INTEGER") &&
           (act.contains("INT") || act == "INTEGER") {
            return true;
        }

        // SQLite type affinity: TEXT types
        if (exp.contains("TEXT") || exp.contains("CHAR") || exp.contains("CLOB")) &&
           (act.contains("TEXT") || act.contains("CHAR") || act.contains("CLOB")) {
            return true;
        }

        // SQLite type affinity: REAL types
        if (exp.contains("REAL") || exp.contains("FLOAT") || exp.contains("DOUBLE")) &&
           (act.contains("REAL") || act.contains("FLOAT") || act.contains("DOUBLE")) {
            return true;
        }

        false
    }
}

/// Schema synchronization - apply schema changes to database
///
/// **[ARCH-DB-SYNC-020]** Automatic column addition via ALTER TABLE
pub struct SchemaSync;

impl SchemaSync {
    /// Synchronize table schema: detect drift and apply fixes
    ///
    /// **What this CAN fix:**
    /// - Missing columns (via ALTER TABLE ADD COLUMN)
    ///
    /// **What this CANNOT fix (requires manual migration):**
    /// - Type changes (requires data migration)
    /// - Constraint changes (requires table recreation)
    /// - Column removal (SQLite limitation)
    pub async fn sync_table<T: TableSchema>(pool: &SqlitePool) -> Result<()> {
        let table_name = T::table_name();
        let expected = T::expected_columns();

        info!("Schema sync: Checking table '{}'", table_name);

        // Check if table exists
        if !SchemaIntrospector::table_exists(pool, table_name).await? {
            warn!(
                "  Table '{}' does not exist - should be created by CREATE TABLE IF NOT EXISTS first",
                table_name
            );
            return Ok(());
        }

        // Read actual schema
        let actual = SchemaIntrospector::introspect_table(pool, table_name).await?;

        // Detect drift
        let drift = SchemaDiff::compare(table_name, &expected, &actual);

        if drift.is_empty() {
            info!("  ✓ Schema up to date for '{}'", table_name);
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
                        "  ⚠ Type mismatch in {}.{}: expected '{}', found '{}'. Manual migration required.",
                        table, column, expected, actual
                    );
                    // Cannot auto-fix type changes - requires data migration
                }
                SchemaDrift::ConstraintMismatch { table, column, constraint } => {
                    warn!(
                        "  ⚠ Constraint mismatch in {}.{}: missing '{}'. Manual migration required.",
                        table, column, constraint
                    );
                    // Cannot auto-fix constraint changes - requires table recreation in SQLite
                }
            }
        }

        // Optional: Run custom validation
        T::validate_schema(pool)?;

        Ok(())
    }

    /// Add missing column to table via ALTER TABLE ADD COLUMN
    async fn add_column(
        pool: &SqlitePool,
        table: &str,
        column: &ColumnDefinition
    ) -> Result<()> {
        let mut sql = format!(
            "ALTER TABLE {} ADD COLUMN {} {}",
            table, column.name, column.sql_type
        );

        // SQLite ALTER TABLE ADD COLUMN limitations:
        // - PRIMARY KEY: Not supported (requires table recreation)
        // - NOT NULL: Only if DEFAULT value provided
        // - UNIQUE: Not supported (requires table recreation)

        if column.primary_key {
            warn!(
                "  ⚠ Cannot add PRIMARY KEY column {}.{} via ALTER TABLE. \
                 Column will be created without PRIMARY KEY constraint.",
                table, column.name
            );
        }

        if column.unique {
            warn!(
                "  ⚠ Cannot add UNIQUE column {}.{} via ALTER TABLE. \
                 Column will be created without UNIQUE constraint.",
                table, column.name
            );
        }

        if column.not_null {
            if let Some(default) = &column.default_value {
                sql.push_str(&format!(" NOT NULL DEFAULT {}", default));
            } else {
                // Cannot add NOT NULL column without default in SQLite
                warn!(
                    "  ⚠ Cannot add NOT NULL column {}.{} without DEFAULT value. \
                     Column will be nullable.",
                    table, column.name
                );
            }
        } else if let Some(default) = &column.default_value {
            // Column is nullable but has default
            sql.push_str(&format!(" DEFAULT {}", default));
        }

        info!("  ✓ Adding column: {}.{} ({})", table, column.name, column.sql_type);

        // Execute ALTER TABLE
        match sqlx::query(&sql).execute(pool).await {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(db_err)) if db_err.message().contains("duplicate column") => {
                // Concurrent initialization - column added by another thread
                info!("  Column {}.{} already added (concurrent initialization)", table, column.name);
                Ok(())
            }
            Err(e) => Err(e.into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqlitePool {
        SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap()
    }

    #[test]
    fn test_column_definition_builder() {
        let col = ColumnDefinition::new("test_col", "TEXT")
            .not_null()
            .unique()
            .default("'default_value'");

        assert_eq!(col.name, "test_col");
        assert_eq!(col.sql_type, "TEXT");
        assert!(col.not_null);
        assert!(col.unique);
        assert_eq!(col.default_value, Some("'default_value'".to_string()));
    }

    #[test]
    fn test_types_compatible() {
        // Exact match
        assert!(SchemaDiff::types_compatible("TEXT", "TEXT"));
        assert!(SchemaDiff::types_compatible("INTEGER", "INTEGER"));

        // Case insensitive
        assert!(SchemaDiff::types_compatible("text", "TEXT"));
        assert!(SchemaDiff::types_compatible("Integer", "INTEGER"));

        // Integer affinity
        assert!(SchemaDiff::types_compatible("INTEGER", "INT"));
        assert!(SchemaDiff::types_compatible("INT", "INTEGER"));

        // Text affinity
        assert!(SchemaDiff::types_compatible("TEXT", "VARCHAR"));
        assert!(SchemaDiff::types_compatible("CHAR", "TEXT"));

        // Real affinity
        assert!(SchemaDiff::types_compatible("REAL", "FLOAT"));
        assert!(SchemaDiff::types_compatible("DOUBLE", "REAL"));

        // Incompatible
        assert!(!SchemaDiff::types_compatible("TEXT", "INTEGER"));
        assert!(!SchemaDiff::types_compatible("REAL", "TEXT"));
    }

    #[tokio::test]
    async fn test_introspect_empty_table() {
        let pool = setup_test_db().await;

        // Create empty table
        sqlx::query(
            r#"
            CREATE TABLE test_table (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                value REAL
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Introspect
        let columns = SchemaIntrospector::introspect_table(&pool, "test_table")
            .await
            .unwrap();

        assert_eq!(columns.len(), 3);

        assert_eq!(columns[0].name, "id");
        assert_eq!(columns[0].type_name, "INTEGER");
        assert!(columns[0].pk);

        assert_eq!(columns[1].name, "name");
        assert_eq!(columns[1].type_name, "TEXT");
        assert!(columns[1].not_null);

        assert_eq!(columns[2].name, "value");
        assert_eq!(columns[2].type_name, "REAL");
        assert!(!columns[2].not_null);
    }

    #[tokio::test]
    async fn test_detect_missing_column() {
        let pool = setup_test_db().await;

        // Create table with only 2 columns
        sqlx::query(
            r#"
            CREATE TABLE test_table (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Expected schema has 3 columns
        let expected = vec![
            ColumnDefinition::new("id", "INTEGER").primary_key(),
            ColumnDefinition::new("name", "TEXT").not_null(),
            ColumnDefinition::new("value", "REAL"),  // Missing!
        ];

        let actual = SchemaIntrospector::introspect_table(&pool, "test_table")
            .await
            .unwrap();

        let drift = SchemaDiff::compare("test_table", &expected, &actual);

        assert_eq!(drift.len(), 1);
        match &drift[0] {
            SchemaDrift::MissingColumn { table, column } => {
                assert_eq!(table, "test_table");
                assert_eq!(column.name, "value");
                assert_eq!(column.sql_type, "REAL");
            }
            _ => panic!("Expected MissingColumn"),
        }
    }

    #[tokio::test]
    async fn test_detect_type_mismatch() {
        let pool = setup_test_db().await;

        // Create table with TEXT column
        sqlx::query(
            r#"
            CREATE TABLE test_table (
                id INTEGER PRIMARY KEY,
                value TEXT
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Expected schema has INTEGER column
        let expected = vec![
            ColumnDefinition::new("id", "INTEGER").primary_key(),
            ColumnDefinition::new("value", "INTEGER"),  // Type mismatch!
        ];

        let actual = SchemaIntrospector::introspect_table(&pool, "test_table")
            .await
            .unwrap();

        let drift = SchemaDiff::compare("test_table", &expected, &actual);

        assert_eq!(drift.len(), 1);
        match &drift[0] {
            SchemaDrift::TypeMismatch { table, column, expected, actual } => {
                assert_eq!(table, "test_table");
                assert_eq!(column, "value");
                assert_eq!(expected, "INTEGER");
                assert_eq!(actual, "TEXT");
            }
            _ => panic!("Expected TypeMismatch"),
        }
    }

    #[tokio::test]
    async fn test_add_column() {
        let pool = setup_test_db().await;

        // Create table with 2 columns
        sqlx::query(
            r#"
            CREATE TABLE test_table (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Add third column
        let new_column = ColumnDefinition::new("value", "REAL");
        SchemaSync::add_column(&pool, "test_table", &new_column)
            .await
            .unwrap();

        // Verify column was added
        let columns = SchemaIntrospector::introspect_table(&pool, "test_table")
            .await
            .unwrap();

        assert_eq!(columns.len(), 3);
        assert_eq!(columns[2].name, "value");
        assert_eq!(columns[2].type_name, "REAL");
    }

    #[tokio::test]
    async fn test_add_column_with_default() {
        let pool = setup_test_db().await;

        sqlx::query(
            r#"
            CREATE TABLE test_table (
                id INTEGER PRIMARY KEY
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Add column with default value
        let new_column = ColumnDefinition::new("status", "TEXT")
            .default("'pending'");

        SchemaSync::add_column(&pool, "test_table", &new_column)
            .await
            .unwrap();

        // Verify default value
        let columns = SchemaIntrospector::introspect_table(&pool, "test_table")
            .await
            .unwrap();

        assert_eq!(columns[1].name, "status");
        assert_eq!(columns[1].default_value, Some("'pending'".to_string()));
    }

    #[tokio::test]
    async fn test_table_exists() {
        let pool = setup_test_db().await;

        // Table doesn't exist
        assert!(!SchemaIntrospector::table_exists(&pool, "nonexistent")
            .await
            .unwrap());

        // Create table
        sqlx::query("CREATE TABLE test_table (id INTEGER)")
            .execute(&pool)
            .await
            .unwrap();

        // Table exists
        assert!(SchemaIntrospector::table_exists(&pool, "test_table")
            .await
            .unwrap());
    }
}
