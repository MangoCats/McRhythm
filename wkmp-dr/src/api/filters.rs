//! Predefined filters for common database queries
//!
//! [REQ-DR-F-040]: Filter passages lacking MusicBrainz ID
//! [REQ-DR-F-050]: Filter files without passages

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;

use crate::{pagination::{calculate_pagination, PAGE_SIZE}, AppState};

/// Query parameters for filters with pagination
#[derive(Debug, Deserialize)]
pub struct FilterQuery {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    pub page: i64,
}

fn default_page() -> i64 {
    1
}

/// Filter response with results and metadata
#[derive(Debug, Serialize)]
pub struct FilterResponse {
    pub filter_name: String,
    pub description: String,
    pub total_results: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

/// GET /api/filters/passages-without-mbid
///
/// Returns passages that lack MusicBrainz recording ID.
/// [REQ-DR-F-040]
pub async fn passages_without_mbid(
    State(state): State<AppState>,
    Query(query): Query<FilterQuery>,
) -> Result<Json<FilterResponse>, FilterError> {
    // Get total count - passages not linked to any song
    let total_results: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM passages
         WHERE guid NOT IN (SELECT DISTINCT passage_id FROM passage_songs)"
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| FilterError::DatabaseError(e.to_string()))?;

    // Calculate pagination
    let p = calculate_pagination(total_results, query.page);

    // Query passages not linked to songs
    let rows = sqlx::query(
        "SELECT guid, file_id, start_time_ticks, end_time_ticks, title, created_at
         FROM passages
         WHERE guid NOT IN (SELECT DISTINCT passage_id FROM passage_songs)
         ORDER BY created_at DESC
         LIMIT ? OFFSET ?"
    )
    .bind(PAGE_SIZE)
    .bind(p.offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| FilterError::DatabaseError(e.to_string()))?;

    // Convert rows to JSON
    let columns = vec![
        "guid".to_string(),
        "file_id".to_string(),
        "start_time_ticks".to_string(),
        "end_time_ticks".to_string(),
        "title".to_string(),
        "created_at".to_string(),
    ];

    let json_rows: Vec<Vec<Value>> = rows
        .iter()
        .map(|row| {
            vec![
                row.get::<String, _>(0).into(),
                row.get::<String, _>(1).into(),
                json!(row.get::<i64, _>(2)),
                json!(row.get::<i64, _>(3)),
                row.try_get::<Option<String>, _>(4)
                    .ok()
                    .flatten()
                    .map(Value::String)
                    .unwrap_or(Value::Null),
                row.get::<String, _>(5).into(),
            ]
        })
        .collect();

    Ok(Json(FilterResponse {
        filter_name: "passages-without-mbid".to_string(),
        description: "Passages lacking MusicBrainz recording ID".to_string(),
        total_results,
        page: p.page,
        page_size: PAGE_SIZE,
        total_pages: p.total_pages,
        columns,
        rows: json_rows,
    }))
}

/// GET /api/filters/files-without-passages
///
/// Returns audio files that have been imported but not yet segmented into passages.
/// [REQ-DR-F-050]
pub async fn files_without_passages(
    State(state): State<AppState>,
    Query(query): Query<FilterQuery>,
) -> Result<Json<FilterResponse>, FilterError> {
    // Get total count
    let total_results: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM files
         WHERE guid NOT IN (SELECT DISTINCT file_id FROM passages)"
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| FilterError::DatabaseError(e.to_string()))?;

    // Calculate pagination
    let p = calculate_pagination(total_results, query.page);

    // Query files
    let rows = sqlx::query(
        "SELECT guid, path, duration_ticks, hash, created_at
         FROM files
         WHERE guid NOT IN (SELECT DISTINCT file_id FROM passages)
         ORDER BY created_at DESC
         LIMIT ? OFFSET ?"
    )
    .bind(PAGE_SIZE)
    .bind(p.offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| FilterError::DatabaseError(e.to_string()))?;

    // Convert rows to JSON
    let columns = vec![
        "guid".to_string(),
        "path".to_string(),
        "duration_ticks".to_string(),
        "hash".to_string(),
        "created_at".to_string(),
    ];

    let json_rows: Vec<Vec<Value>> = rows
        .iter()
        .map(|row| {
            vec![
                row.get::<String, _>(0).into(),
                row.get::<String, _>(1).into(),
                row.try_get::<Option<i64>, _>(2)
                    .ok()
                    .flatten()
                    .map(|v| json!(v))
                    .unwrap_or(Value::Null),
                row.get::<String, _>(3).into(),
                row.get::<String, _>(4).into(),
            ]
        })
        .collect();

    Ok(Json(FilterResponse {
        filter_name: "files-without-passages".to_string(),
        description: "Audio files not yet segmented into passages".to_string(),
        total_results,
        page: p.page,
        page_size: PAGE_SIZE,
        total_pages: p.total_pages,
        columns,
        rows: json_rows,
    }))
}

/// Filter errors
#[derive(Debug)]
pub enum FilterError {
    DatabaseError(String),
}

impl IntoResponse for FilterError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            FilterError::DatabaseError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", msg))
            }
        };

        let body = Json(json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}
