//! Database access layer for wkmp-dr
//!
//! [REQ-DR-NF-020]: All connections are read-only
//! [REQ-DR-F-010]: Table-by-table content viewing
//! [REQ-DR-F-030]: Row count display

use anyhow::{Context, Result};
use sqlx::SqlitePool;
use std::path::Path;

mod tables;
pub use tables::list_tables;
#[allow(unused_imports)]
pub use tables::TableInfo;

/// Connect to database with read-only mode [REQ-DR-NF-020]
///
/// Safety: Uses SQLite mode=ro to prevent any write operations
pub async fn connect_readonly(db_path: &Path) -> Result<SqlitePool> {
    if !db_path.exists() {
        anyhow::bail!(
            "Database not found: {}\nPlease run wkmp-ui first to initialize the database.",
            db_path.display()
        );
    }

    // mode=ro: Read-only mode [REQ-DR-NF-020]
    // immutable=1: Additional safety (SQLite won't write even for internal operations)
    let db_url = format!("sqlite://{}?mode=ro&immutable=1", db_path.display());

    let pool = SqlitePool::connect(&db_url)
        .await
        .context("Failed to connect to database in read-only mode")?;

    // Verify read-only by attempting a write (should fail)
    #[cfg(debug_assertions)]
    {
        let write_test = sqlx::query("CREATE TABLE _test_write (id INTEGER)")
            .execute(&pool)
            .await;
        if write_test.is_ok() {
            panic!("SAFETY VIOLATION: Database connection is not read-only!");
        }
    }

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// TC-U-NF020-01: Read-only connection mode
    #[tokio::test]
    async fn test_readonly_connection() {
        // This test requires a real wkmp.db to exist
        // Skip if database doesn't exist (CI environment)
        let db_path = PathBuf::from(env!("HOME")).join("Music/wkmp.db");
        if !db_path.exists() {
            eprintln!("Skipping test: database not found at {:?}", db_path);
            return;
        }

        let pool = connect_readonly(&db_path)
            .await
            .expect("Should connect in read-only mode");

        // Attempt write operation - should fail
        let result = sqlx::query("CREATE TABLE _test (id INTEGER)")
            .execute(&pool)
            .await;

        assert!(result.is_err(), "Write operation should fail in read-only mode");
    }
}
