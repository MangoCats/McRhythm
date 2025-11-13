//! Amplitude analysis API handlers
//!
//! **[IMPL008]** POST /analyze/amplitude

use axum::{
    extract::State,
    routing::post,
    Json, Router,
};

use crate::{
    error::ApiResult,
    models::{AmplitudeAnalysisRequest, AmplitudeAnalysisResponse},
    AppState,
};

/// **[IMPL008]** POST /analyze/amplitude
///
/// Analyze single file amplitude envelope.
pub async fn analyze_amplitude(
    State(_state): State<AppState>,
    Json(request): Json<AmplitudeAnalysisRequest>,
) -> ApiResult<Json<AmplitudeAnalysisResponse>> {
    use crate::services::AmplitudeAnalyzer;
    use std::path::Path;

    tracing::info!(
        file_path = %request.file_path,
        start = request.start_time,
        end = request.end_time.unwrap_or(0.0),
        "Amplitude analysis request"
    );

    // Get parameters (use request params or use Default trait)
    let params = request.parameters.unwrap_or_default();

    // Create analyzer
    let analyzer = AmplitudeAnalyzer::new(params);

    // Analyze file
    let file_path = Path::new(&request.file_path);
    let end_time = request.end_time.unwrap_or(f64::MAX);

    let result = analyzer
        .analyze_file(file_path, request.start_time, end_time)
        .await
        .map_err(|e| crate::error::ApiError::Internal(e.to_string()))?;

    // Convert to response (convert f32 RMS profile to f64)
    let rms_profile_f64: Vec<f64> = result
        .rms_profile
        .unwrap_or_default()
        .iter()
        .map(|&v| v as f64)
        .collect();

    let response = AmplitudeAnalysisResponse {
        file_path: request.file_path,
        peak_rms: result.peak_rms,
        lead_in_duration: result.lead_in_duration,
        lead_out_duration: result.lead_out_duration,
        quick_ramp_up: result.quick_ramp_up,
        quick_ramp_down: result.quick_ramp_down,
        rms_profile: rms_profile_f64,
        analyzed_at: chrono::Utc::now(),
    };

    Ok(Json(response))
}

/// Build amplitude analysis routes
pub fn amplitude_routes() -> Router<AppState> {
    Router::new().route("/analyze/amplitude", post(analyze_amplitude))
}
