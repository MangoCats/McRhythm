//! File Classification API handlers
//!
//! **[AIA-CLASSIFY-UI-040]** GET /api/import/file-classification
//! **PLAN027** File classification report endpoints (IMPL008 v1.1)

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiResult},
    models::ImportState,
    AppState,
};

/// Query parameters for file classification endpoint
#[derive(Debug, Deserialize)]
pub struct FileClassificationQuery {
    /// Filter by category (audio|image|other)
    pub category: Option<String>,
    /// Pagination offset (default: 0)
    #[serde(default)]
    pub offset: usize,
    /// Pagination limit (default: 500, max: 1000)
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    500
}

/// File info for response
#[derive(Debug, Serialize, Clone)]
pub struct FileInfoResponse {
    /// Absolute path to file
    pub path: String,
    /// File size in bytes
    pub size_bytes: u64,
    /// File last modified timestamp (ISO 8601)
    pub modified_at: String,
}

/// Category statistics
#[derive(Debug, Serialize)]
pub struct CategoryStats {
    /// Number of files in this category
    pub count: usize,
    /// Total size of all files in this category (bytes)
    pub total_size_bytes: u64,
    /// List of files (paginated)
    pub files: Vec<FileInfoResponse>,
}

/// Response for all-categories view
#[derive(Debug, Serialize)]
pub struct AllCategoriesResponse {
    /// Session ID
    pub session_id: Uuid,
    /// Scanned folder path
    pub scanned_folder: String,
    /// When scan completed (ISO 8601 timestamp)
    pub scan_completed_at: String,
    /// Audio files category
    pub audio_files: CategoryStats,
    /// Image files category
    pub image_files: CategoryStats,
    /// Other files category
    pub other_files: CategoryStats,
}

/// Pagination info for single-category response
#[derive(Debug, Serialize)]
pub struct PaginationInfo {
    /// Current offset
    pub offset: usize,
    /// Current limit
    pub limit: usize,
    /// Total count in this category
    pub total_count: usize,
    /// Whether there are more files after this page
    pub has_more: bool,
}

/// Response for single-category view
#[derive(Debug, Serialize)]
pub struct SingleCategoryResponse {
    /// Session ID
    pub session_id: Uuid,
    /// Scanned folder path
    pub scanned_folder: String,
    /// When scan completed (ISO 8601 timestamp)
    pub scan_completed_at: String,
    /// Category name
    pub category: String,
    /// Pagination information
    pub pagination: PaginationInfo,
    /// Files in this category (paginated)
    pub files: Vec<FileInfoResponse>,
}

/// **[AIA-CLASSIFY-UI-030]** Format file size in human-readable format
///
/// - `< 1 KB`: "1,234 B"
/// - `< 1 MB`: "42.5 KB"
/// - `< 1 GB`: "15.3 MB"
/// - `>= 1 GB`: "2.1 GB"
pub fn format_file_size(size_bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size_bytes < KB {
        format!("{} B", size_bytes)
    } else if size_bytes < MB {
        format!("{:.1} KB", size_bytes as f64 / KB as f64)
    } else if size_bytes < GB {
        format!("{:.1} MB", size_bytes as f64 / MB as f64)
    } else {
        format!("{:.1} GB", size_bytes as f64 / GB as f64)
    }
}

/// Helper to convert SystemTime to ISO 8601 string
fn systemtime_to_iso8601(time: SystemTime) -> String {
    use std::time::UNIX_EPOCH;
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(duration.as_secs() as i64, 0)
                .unwrap_or_else(|| chrono::Utc::now());
            datetime.to_rfc3339()
        }
        Err(_) => chrono::Utc::now().to_rfc3339(),
    }
}

/// **[AIA-CLASSIFY-UI-040]** GET /api/import/file-classification
///
/// Returns file classification results after SCANNING phase completes.
/// Supports pagination and category filtering.
///
/// **Query Parameters:**
/// - `category` (optional): Filter by category (`audio`, `image`, `other`)
/// - `offset` (optional): Pagination offset (default: 0)
/// - `limit` (optional): Maximum files to return (default: 500, max: 1000)
///
/// **Error Responses:**
/// - `404 Not Found`: Session not found or SCANNING not yet completed
/// - `409 Conflict`: Import still in SCANNING phase (report not ready)
/// - `400 Bad Request`: Invalid category parameter
pub async fn get_file_classification(
    State(state): State<AppState>,
    Query(query): Query<FileClassificationQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    // Validate limit
    let limit = query.limit.min(1000);

    // Validate category (if provided)
    if let Some(ref cat) = query.category {
        if !matches!(cat.as_str(), "audio" | "image" | "other") {
            return Err(ApiError::BadRequest(format!(
                "Invalid category '{}', must be 'audio', 'image', or 'other'",
                cat
            )));
        }
    }

    // **[PLAN027]** Get most recent session (active or completed) since file_classification is now persisted
    let session = sqlx::query(
        r#"
        SELECT session_id, state, root_folder, parameters,
               progress_current, progress_total, progress_percentage,
               current_operation, errors, started_at, ended_at, file_classification_data
        FROM import_sessions
        ORDER BY started_at DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?
    .ok_or_else(|| ApiError::NotFound("No import session found".to_string()))?;

    // Parse session fields we need
    use sqlx::Row;
    let session_id_str: String = session.get("session_id");
    let session_id = Uuid::parse_str(&session_id_str)
        .map_err(|e| ApiError::Internal(format!("Failed to parse session_id: {}", e)))?;

    let state_json: String = session.get("state");
    let import_state: ImportState = serde_json::from_str(&state_json)
        .map_err(|e| ApiError::Internal(format!("Failed to deserialize state: {}", e)))?;

    let root_folder: String = session.get("root_folder");

    let file_classification_data: Option<String> = session.get("file_classification_data");
    let file_classification = if let Some(data) = file_classification_data {
        serde_json::from_str::<crate::models::FileClassification>(&data)
            .map_err(|e| ApiError::Internal(format!("Failed to deserialize file_classification: {}", e)))?
    } else {
        crate::models::FileClassification::new()
    };

    // Check if SCANNING has completed
    if import_state == ImportState::Scanning {
        return Err(ApiError::Conflict(
            "Import still in SCANNING phase, classification report not yet available".to_string()
        ));
    }

    // Check if classification data is available
    if file_classification.scan_completed_at.is_none() {
        return Err(ApiError::NotFound(
            "File classification data not available for this session".to_string()
        ));
    }

    let scan_completed_at = file_classification.scan_completed_at
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

    // Helper to convert FileInfo to FileInfoResponse
    let convert_file_info = |file_info: &crate::models::FileInfo| -> FileInfoResponse {
        FileInfoResponse {
            path: file_info.path.to_string_lossy().to_string(),
            size_bytes: file_info.size_bytes,
            modified_at: systemtime_to_iso8601(file_info.modified_at),
        }
    };

    // If category filter specified, return single-category response
    if let Some(category) = query.category {
        let (files, total_count) = match category.as_str() {
            "audio" => (
                &file_classification.audio_files,
                file_classification.audio_files.len(),
            ),
            "image" => (
                &file_classification.image_files,
                file_classification.image_files.len(),
            ),
            "other" => (
                &file_classification.other_files,
                file_classification.other_files.len(),
            ),
            _ => unreachable!(), // Already validated above
        };

        // Paginate files
        let start = query.offset.min(files.len());
        let end = (start + limit).min(files.len());
        let has_more = end < files.len();

        let paginated_files: Vec<FileInfoResponse> = files[start..end]
            .iter()
            .map(convert_file_info)
            .collect();

        let response = SingleCategoryResponse {
            session_id,
            scanned_folder: root_folder.clone(),
            scan_completed_at,
            category,
            pagination: PaginationInfo {
                offset: query.offset,
                limit,
                total_count,
                has_more,
            },
            files: paginated_files,
        };

        Ok(Json(serde_json::to_value(response).map_err(|e| {
            ApiError::Internal(format!("JSON serialization error: {}", e))
        })?))
    } else {
        // Return all categories (first 500 files each, per spec)
        let audio_files: Vec<FileInfoResponse> = file_classification.audio_files
            .iter()
            .take(limit)
            .map(convert_file_info)
            .collect();

        let image_files: Vec<FileInfoResponse> = file_classification.image_files
            .iter()
            .take(limit)
            .map(convert_file_info)
            .collect();

        let other_files: Vec<FileInfoResponse> = file_classification.other_files
            .iter()
            .take(limit)
            .map(convert_file_info)
            .collect();

        let response = AllCategoriesResponse {
            session_id,
            scanned_folder: root_folder.clone(),
            scan_completed_at,
            audio_files: CategoryStats {
                count: file_classification.audio_files.len(),
                total_size_bytes: file_classification.audio_total_size(),
                files: audio_files,
            },
            image_files: CategoryStats {
                count: file_classification.image_files.len(),
                total_size_bytes: file_classification.image_total_size(),
                files: image_files,
            },
            other_files: CategoryStats {
                count: file_classification.other_files.len(),
                total_size_bytes: file_classification.other_total_size(),
                files: other_files,
            },
        };

        Ok(Json(serde_json::to_value(response).map_err(|e| {
            ApiError::Internal(format!("JSON serialization error: {}", e))
        })?))
    }
}

/// Build file classification routes
pub fn file_classification_routes() -> Router<AppState> {
    Router::new()
        .route("/api/import/file-classification", get(get_file_classification))
}
