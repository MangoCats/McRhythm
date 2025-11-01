//! Custom search functionality
//!
//! [REQ-DR-F-060]: Search by MusicBrainz Work ID
//! [REQ-DR-F-070]: Search by file path pattern

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

use crate::AppState;

/// Query parameters for Work ID search
#[derive(Debug, Deserialize)]
pub struct WorkIdQuery {
    /// MusicBrainz Work ID (UUID format)
    pub work_id: String,

    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    pub page: i64,
}

/// Query parameters for file path search
#[derive(Debug, Deserialize)]
pub struct PathQuery {
    /// File path pattern (SQL LIKE syntax: % for wildcard)
    pub pattern: String,

    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    pub page: i64,
}

fn default_page() -> i64 {
    1
}

/// Search response with results and metadata
#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub search_type: String,
    pub query: String,
    pub total_results: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

/// GET /api/search/by-work-id?work_id=UUID
///
/// Search for songs by MusicBrainz Work ID.
/// Returns all passages associated with songs that reference the specified work.
/// [REQ-DR-F-060]
pub async fn search_by_work_id(
    State(state): State<AppState>,
    Query(query): Query<WorkIdQuery>,
) -> Result<Json<SearchResponse>, SearchError> {
    // Validate UUID format
    Uuid::parse_str(&query.work_id)
        .map_err(|_| SearchError::InvalidWorkId(query.work_id.clone()))?;

    const PAGE_SIZE: i64 = 100;

    // Get total count
    // Songs table has work_id column referencing MusicBrainz Work
    let total_results: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM passages p
         JOIN passage_songs ps ON p.guid = ps.passage_id
         JOIN songs s ON ps.song_id = s.guid
         WHERE s.work_id = ?"
    )
    .bind(&query.work_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| SearchError::DatabaseError(e.to_string()))?;

    // Calculate pagination
    let total_pages = (total_results + PAGE_SIZE - 1) / PAGE_SIZE;
    let page = query.page.max(1).min(total_pages.max(1));
    let offset = (page - 1) * PAGE_SIZE;

    // Query passages
    let rows = sqlx::query(
        "SELECT p.guid, p.file_id, p.start_time_ticks, p.end_time_ticks, s.recording_mbid, s.work_id
         FROM passages p
         JOIN passage_songs ps ON p.guid = ps.passage_id
         JOIN songs s ON ps.song_id = s.guid
         WHERE s.work_id = ?
         ORDER BY p.created_at DESC
         LIMIT ? OFFSET ?"
    )
    .bind(&query.work_id)
    .bind(PAGE_SIZE)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| SearchError::DatabaseError(e.to_string()))?;

    // Convert rows to JSON
    let columns = vec![
        "passage_guid".to_string(),
        "file_id".to_string(),
        "start_time_ticks".to_string(),
        "end_time_ticks".to_string(),
        "recording_mbid".to_string(),
        "work_id".to_string(),
    ];

    let json_rows: Vec<Vec<Value>> = rows
        .iter()
        .map(|row| {
            vec![
                row.get::<String, _>(0).into(),
                row.get::<String, _>(1).into(),
                json!(row.get::<i64, _>(2)),
                json!(row.get::<i64, _>(3)),
                row.get::<String, _>(4).into(),
                row.get::<String, _>(5).into(),
            ]
        })
        .collect();

    Ok(Json(SearchResponse {
        search_type: "by-work-id".to_string(),
        query: query.work_id,
        total_results,
        page,
        page_size: PAGE_SIZE,
        total_pages,
        columns,
        rows: json_rows,
    }))
}

/// GET /api/search/by-path?pattern=%.flac
///
/// Search for files by path pattern (SQL LIKE syntax).
/// Returns all files matching the pattern.
/// [REQ-DR-F-070]
pub async fn search_by_path(
    State(state): State<AppState>,
    Query(query): Query<PathQuery>,
) -> Result<Json<SearchResponse>, SearchError> {
    // Validate pattern is not empty
    if query.pattern.trim().is_empty() {
        return Err(SearchError::EmptyPattern);
    }

    const PAGE_SIZE: i64 = 100;

    // Get total count
    let total_results: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM files WHERE path LIKE ?"
    )
    .bind(&query.pattern)
    .fetch_one(&state.db)
    .await
    .map_err(|e| SearchError::DatabaseError(e.to_string()))?;

    // Calculate pagination
    let total_pages = (total_results + PAGE_SIZE - 1) / PAGE_SIZE;
    let page = query.page.max(1).min(total_pages.max(1));
    let offset = (page - 1) * PAGE_SIZE;

    // Query files
    let rows = sqlx::query(
        "SELECT guid, path, duration, hash, created_at
         FROM files
         WHERE path LIKE ?
         ORDER BY path ASC
         LIMIT ? OFFSET ?"
    )
    .bind(&query.pattern)
    .bind(PAGE_SIZE)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| SearchError::DatabaseError(e.to_string()))?;

    // Convert rows to JSON
    let columns = vec![
        "guid".to_string(),
        "path".to_string(),
        "duration".to_string(),
        "hash".to_string(),
        "created_at".to_string(),
    ];

    let json_rows: Vec<Vec<Value>> = rows
        .iter()
        .map(|row| {
            vec![
                row.get::<String, _>(0).into(),
                row.get::<String, _>(1).into(),
                row.try_get::<Option<f64>, _>(2)
                    .ok()
                    .flatten()
                    .map(|v| json!(v))
                    .unwrap_or(Value::Null),
                row.get::<String, _>(3).into(),
                row.get::<String, _>(4).into(),
            ]
        })
        .collect();

    Ok(Json(SearchResponse {
        search_type: "by-path".to_string(),
        query: query.pattern,
        total_results,
        page,
        page_size: PAGE_SIZE,
        total_pages,
        columns,
        rows: json_rows,
    }))
}

/// Search errors
#[derive(Debug)]
pub enum SearchError {
    InvalidWorkId(String),
    EmptyPattern,
    DatabaseError(String),
}

impl IntoResponse for SearchError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            SearchError::InvalidWorkId(id) => {
                (StatusCode::BAD_REQUEST, format!("Invalid Work ID (must be UUID): {}", id))
            }
            SearchError::EmptyPattern => {
                (StatusCode::BAD_REQUEST, "Empty search pattern".to_string())
            }
            SearchError::DatabaseError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", msg))
            }
        };

        let body = Json(json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}
