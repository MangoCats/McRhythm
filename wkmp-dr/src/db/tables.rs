//! Table listing and metadata queries
//!
//! [REQ-DR-F-010]: Table-by-table content viewing
//! [REQ-DR-F-030]: Row count display

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Table metadata [REQ-DR-F-010, REQ-DR-F-030]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableInfo {
    /// Table name
    pub name: String,
    /// Number of rows in table
    pub row_count: i64,
}

/// List all tables with row counts [REQ-DR-F-010, REQ-DR-F-030]
///
/// Returns tables in alphabetical order, excluding SQLite internal tables.
/// Note: Not currently used by API layer, kept for potential future table enumeration endpoint
#[allow(dead_code)]
pub async fn list_tables(pool: &SqlitePool) -> Result<Vec<TableInfo>> {
    let tables = sqlx::query_as::<_, (String,)>(
        r#"
        SELECT name
        FROM sqlite_master
        WHERE type = 'table'
          AND name NOT LIKE 'sqlite_%'
        ORDER BY name ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut table_infos = Vec::new();

    for (table_name,) in tables {
        // Get row count for each table
        let row_count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", table_name))
            .fetch_one(pool)
            .await?;

        table_infos.push(TableInfo {
            name: table_name,
            row_count,
        });
    }

    Ok(table_infos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// TC-U-010-01: List all tables query
    #[tokio::test]
    async fn test_list_all_tables() {
        // This test requires a real wkmp.db to exist
        let db_path = PathBuf::from(env!("HOME")).join("Music/wkmp.db");
        if !db_path.exists() {
            eprintln!("Skipping test: database not found at {:?}", db_path);
            return;
        }

        let pool = crate::db::connect_readonly(&db_path)
            .await
            .expect("Should connect to database");

        let tables = list_tables(&pool)
            .await
            .expect("Should list tables");

        // Verify we got tables
        assert!(tables.len() > 0, "Should have at least one table");

        // Verify tables are alphabetically sorted
        for i in 1..tables.len() {
            assert!(
                tables[i - 1].name <= tables[i].name,
                "Tables should be in alphabetical order"
            );
        }

        // Verify no SQLite internal tables
        for table in &tables {
            assert!(
                !table.name.starts_with("sqlite_"),
                "Should not include SQLite internal tables"
            );
        }

        // Expected WKMP tables (based on actual schema)
        let expected_tables = [
            "acoustid_cache",
            "albums",
            "artists",
            "files",
            "images",
            "import_sessions",
            "passage_albums",
            "passage_songs",
            "passages",
            "schema_version",
            "settings",
            "song_artists",
            "songs",
            "temp_file_albums",
            "temp_file_songs",
            "users",
            "works",
        ];

        for expected in &expected_tables {
            assert!(
                tables.iter().any(|t| &t.name == expected),
                "Should have table: {}",
                expected
            );
        }

        // Verify row counts are non-negative
        for table in &tables {
            assert!(
                table.row_count >= 0,
                "Row count should be non-negative for table: {}",
                table.name
            );
        }

        println!("✓ Found {} tables", tables.len());
        for table in &tables {
            println!("  - {} ({} rows)", table.name, table.row_count);
        }
    }

    /// TC-U-030-01: Row count query
    #[tokio::test]
    async fn test_row_counts() {
        let db_path = PathBuf::from(env!("HOME")).join("Music/wkmp.db");
        if !db_path.exists() {
            eprintln!("Skipping test: database not found");
            return;
        }

        let pool = crate::db::connect_readonly(&db_path)
            .await
            .expect("Should connect to database");

        let tables = list_tables(&pool)
            .await
            .expect("Should list tables");

        // Row counts should match manual COUNT(*) queries
        for table in &tables {
            let manual_count: i64 =
                sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", table.name))
                    .fetch_one(&pool)
                    .await
                    .expect("Manual count should succeed");

            assert_eq!(
                table.row_count, manual_count,
                "Row count mismatch for table: {}",
                table.name
            );
        }
    }
}
