//! Table viewing API with pagination and sorting
//!
//! [REQ-DR-F-010]: Table-by-table content viewing
//! [REQ-DR-F-020]: Paginated browsing (100 rows/page)
//! [REQ-DR-F-080]: Sort columns (ascending/descending)

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Column, Row, ValueRef};

use crate::{pagination::{calculate_pagination, PAGE_SIZE}, AppState};

/// Query parameters for table viewing
#[derive(Debug, Deserialize)]
pub struct TableQuery {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    pub page: i64,

    /// Column to sort by (optional)
    pub sort: Option<String>,

    /// Sort order: "asc" or "desc"
    #[serde(default = "default_order")]
    pub order: String,
}

fn default_page() -> i64 {
    1
}

fn default_order() -> String {
    "asc".to_string()
}

/// Table data response [REQ-DR-F-020, REQ-DR-F-030]
#[derive(Debug, Serialize)]
pub struct TableDataResponse {
    pub table_name: String,
    pub total_rows: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
}

/// Reorder columns for logical display: human-readable first, UUIDs last
///
/// Returns all columns from db_columns in a more logical order:
/// - Human-readable columns (title, name, path) first
/// - Data columns in the middle
/// - Unknown columns (not in priority list)
/// - UUID/ID/GUID columns last (all together on right side)
fn get_column_order(table_name: &str, db_columns: &[String]) -> Vec<String> {
    // Define priority columns for each table (human-readable first)
    // ID/UUID/GUID/MBID columns are identified by pattern, not listed here
    let priority_cols: Vec<&str> = match table_name {
        "songs" => vec!["title", "lyrics", "related_songs", "base_probability", "min_cooldown",
                       "ramping_cooldown", "last_played_at", "created_at", "updated_at"],
        "passages" => vec!["title", "passage_number", "start_sample", "end_sample",
                          "crossfade_start_sample", "crossfade_end_sample",
                          "fade_in_curve", "fade_out_curve", "musical_flavor",
                          "base_probability", "last_played_at", "created_at", "updated_at"],
        "files" => vec!["path", "format", "sample_rate", "channels",
                       "duration_samples", "file_hash", "file_size_bytes",
                       "created_at", "updated_at"],
        "artists" => vec!["name", "sort_name", "created_at", "updated_at"],
        "albums" => vec!["title", "artist_credit", "release_date"],
        "works" => vec!["title", "composer_credit"],
        "passage_songs" => vec![],
        "album_songs" => vec!["track_number"],
        "settings" => vec!["key", "value"],
        "timeslots" => vec!["hour", "target_flavor"],
        _ => vec![],
    };

    // Helper function to check if column is a UUID/ID/GUID/MBID column
    let is_id_column = |col: &str| -> bool {
        col == "id"
            || col == "guid"
            || col == "mbid"
            || col.ends_with("_id")
            || col.ends_with("_guid")
            || col.ends_with("_mbid")
            || col == "musicbrainz_id"
            || col == "recording_mbid"
            || col.contains("uuid")
    };

    let mut ordered: Vec<String> = Vec::new();

    // Step 1: Add priority columns that exist in db_columns
    for &col in &priority_cols {
        if db_columns.contains(&col.to_string()) {
            ordered.push(col.to_string());
        }
    }

    // Step 2: Add unknown columns (not in priority list, not ID/UUID columns)
    for col in db_columns {
        let col_str = col.as_str();
        if !priority_cols.contains(&col_str) && !is_id_column(col_str) {
            ordered.push(col.clone());
        }
    }

    // Step 3: Add all ID/UUID/GUID columns last (together on right side)
    for col in db_columns {
        if is_id_column(col.as_str()) {
            ordered.push(col.clone());
        }
    }

    ordered
}

/// GET /api/table/:name
///
/// Returns paginated table data with optional sorting.
/// [REQ-DR-F-010, REQ-DR-F-020, REQ-DR-F-080]
pub async fn get_table_data(
    State(state): State<AppState>,
    Path(table_name): Path<String>,
    Query(query): Query<TableQuery>,
) -> Result<Json<TableDataResponse>, TableError> {
    // Validate table name (prevent SQL injection)
    if !is_valid_table_name(&table_name) {
        return Err(TableError::InvalidTableName(table_name));
    }

    // Get total row count
    let total_rows: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {}", table_name))
        .fetch_one(&state.db)
        .await
        .map_err(|e| TableError::DatabaseError(e.to_string()))?;

    // Calculate pagination
    let p = calculate_pagination(total_rows, query.page);

    // Build query with optional sorting
    // [DR-SEC-060] Table name is whitelisted, safe to use directly
    let mut sql = format!("SELECT * FROM {}", table_name);

    if let Some(sort_column) = &query.sort {
        // Validate sort column exists
        if !is_valid_column(&state, &table_name, sort_column).await? {
            return Err(TableError::InvalidColumn(sort_column.clone()));
        }

        let order = if query.order.to_lowercase() == "desc" {
            "DESC"
        } else {
            "ASC"
        };

        // [DR-SEC-060] Quote column identifier for safety
        sql.push_str(&format!(" ORDER BY \"{}\" {}", escape_identifier(sort_column), order));
    }

    sql.push_str(&format!(" LIMIT {} OFFSET {}", PAGE_SIZE, p.offset));

    // Execute query
    let rows = sqlx::query(&sql)
        .fetch_all(&state.db)
        .await
        .map_err(|e| TableError::DatabaseError(e.to_string()))?;

    // Get column names from database
    let db_columns: Vec<String> = if let Some(first_row) = rows.first() {
        first_row
            .columns()
            .iter()
            .map(|col| col.name().to_string())
            .collect()
    } else {
        // Empty table - get columns from schema
        get_table_columns(&state, &table_name).await?
    };

    // Reorder columns: human-readable first, UUIDs last
    let column_order = get_column_order(&table_name, &db_columns);

    // Create index mapping from display order to database order
    let column_indices: Vec<usize> = column_order
        .iter()
        .map(|col_name| {
            db_columns
                .iter()
                .position(|db_col| db_col == col_name)
                .unwrap_or(0)
        })
        .collect();

    // Convert rows to JSON values with reordered columns
    let json_rows: Vec<Vec<serde_json::Value>> = rows
        .iter()
        .map(|row| {
            column_indices
                .iter()
                .map(|&i| {
                    // Convert SQLite value to JSON
                    row.try_get_raw(i)
                        .ok()
                        .and_then(|val| {
                            // Handle different SQLite types
                            if val.is_null() {
                                Some(serde_json::Value::Null)
                            } else {
                                // Try common types
                                row.try_get::<String, _>(i)
                                    .ok()
                                    .map(serde_json::Value::String)
                                    .or_else(|| {
                                        row.try_get::<i64, _>(i)
                                            .ok()
                                            .map(|v| json!(v))
                                    })
                                    .or_else(|| {
                                        row.try_get::<f64, _>(i)
                                            .ok()
                                            .map(|v| json!(v))
                                    })
                            }
                        })
                        .unwrap_or(serde_json::Value::Null)
                })
                .collect()
        })
        .collect();

    Ok(Json(TableDataResponse {
        table_name,
        total_rows,
        page: p.page,
        page_size: PAGE_SIZE,
        total_pages: p.total_pages,
        columns: column_order,
        rows: json_rows,
    }))
}

/// Escape SQL identifier (column name) for safe use in queries
/// [DR-SEC-060] Escapes double quotes by doubling them per SQLite spec
fn escape_identifier(identifier: &str) -> String {
    // In SQLite, double quotes in identifiers are escaped by doubling them
    // e.g., column"name becomes column""name
    identifier.replace('"', "\"\"")
}

/// Validate table name to prevent SQL injection
/// [DR-SEC-060] Uses whitelist approach for maximum security
fn is_valid_table_name(name: &str) -> bool {
    // Whitelist of known WKMP tables per IMPL001-database_schema.md
    const ALLOWED_TABLES: &[&str] = &[
        "songs",
        "passages",
        "files",
        "artists",
        "albums",
        "works",
        "passage_songs",
        "album_songs",
        "settings",
        "timeslots",
    ];
    ALLOWED_TABLES.contains(&name)
}

/// Check if column exists in table
async fn is_valid_column(
    state: &AppState,
    table_name: &str,
    column_name: &str,
) -> Result<bool, TableError> {
    let columns = get_table_columns(state, table_name).await?;
    Ok(columns.contains(&column_name.to_string()))
}

/// Get column names for a table
async fn get_table_columns(state: &AppState, table_name: &str) -> Result<Vec<String>, TableError> {
    let rows = sqlx::query(&format!("PRAGMA table_info({})", table_name))
        .fetch_all(&state.db)
        .await
        .map_err(|e| TableError::DatabaseError(e.to_string()))?;

    // PRAGMA table_info returns: (cid, name, type, notnull, dflt_value, pk)
    // We need column 1 (name)
    Ok(rows
        .iter()
        .map(|row| row.get::<String, _>(1))
        .collect())
}

/// Table API errors
#[derive(Debug)]
pub enum TableError {
    InvalidTableName(String),
    InvalidColumn(String),
    DatabaseError(String),
}

impl IntoResponse for TableError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            TableError::InvalidTableName(name) => {
                (StatusCode::BAD_REQUEST, format!("Invalid table name: {}", name))
            }
            TableError::InvalidColumn(col) => {
                (StatusCode::BAD_REQUEST, format!("Invalid column: {}", col))
            }
            TableError::DatabaseError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", msg))
            }
        };

        let body = Json(json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}
