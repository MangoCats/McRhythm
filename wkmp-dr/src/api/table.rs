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

use crate::AppState;

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
    const PAGE_SIZE: i64 = 100;
    let total_pages = (total_rows + PAGE_SIZE - 1) / PAGE_SIZE;
    let page = query.page.max(1).min(total_pages.max(1));
    let offset = (page - 1) * PAGE_SIZE;

    // Build query with optional sorting
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

        sql.push_str(&format!(" ORDER BY {} {}", sort_column, order));
    }

    sql.push_str(&format!(" LIMIT {} OFFSET {}", PAGE_SIZE, offset));

    // Execute query
    let rows = sqlx::query(&sql)
        .fetch_all(&state.db)
        .await
        .map_err(|e| TableError::DatabaseError(e.to_string()))?;

    // Get column names
    let columns = if let Some(first_row) = rows.first() {
        first_row
            .columns()
            .iter()
            .map(|col| col.name().to_string())
            .collect()
    } else {
        // Empty table - get columns from schema
        get_table_columns(&state, &table_name).await?
    };

    // Convert rows to JSON values
    let json_rows: Vec<Vec<serde_json::Value>> = rows
        .iter()
        .map(|row| {
            (0..row.len())
                .map(|i| {
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
        page,
        page_size: PAGE_SIZE,
        total_pages,
        columns,
        rows: json_rows,
    }))
}

/// Validate table name to prevent SQL injection
fn is_valid_table_name(name: &str) -> bool {
    // Only allow alphanumeric, underscore, and hyphen
    name.chars().all(|c| c.is_alphanumeric() || c == '_')
        && !name.is_empty()
        && name.len() < 100
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
