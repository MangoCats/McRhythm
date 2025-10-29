//! Parameter management API handlers
//!
//! **[IMPL008]** GET /parameters/global, POST /parameters/global

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::{error::ApiResult, models::AmplitudeParameters, AppState};

/// GET /parameters/global response
#[derive(Debug, Serialize)]
pub struct GlobalParametersResponse {
    #[serde(flatten)]
    pub parameters: AmplitudeParameters,
}

/// POST /parameters/global request (partial updates)
#[derive(Debug, Deserialize)]
pub struct UpdateParametersRequest {
    pub rms_window_ms: Option<u32>,
    pub lead_in_threshold_db: Option<f64>,
    pub lead_out_threshold_db: Option<f64>,
    pub quick_ramp_threshold: Option<f64>,
    pub quick_ramp_duration_s: Option<f64>,
    pub max_lead_in_duration_s: Option<f64>,
    pub max_lead_out_duration_s: Option<f64>,
    pub apply_a_weighting: Option<bool>,
}

/// POST /parameters/global response
#[derive(Debug, Serialize)]
pub struct UpdateParametersResponse {
    pub status: String,
    pub parameters: AmplitudeParameters,
}

/// **[IMPL008]** GET /parameters/global
///
/// Get global import parameters from database.
pub async fn get_global_parameters(
    State(state): State<AppState>,
) -> ApiResult<Json<GlobalParametersResponse>> {
    // Load from database settings table
    let parameters = crate::db::parameters::load_amplitude_parameters(&state.db).await?;

    tracing::debug!(?parameters, "Get global parameters from database");

    Ok(Json(GlobalParametersResponse { parameters }))
}

/// **[IMPL008]** POST /parameters/global
///
/// Update global parameters (partial update).
pub async fn update_global_parameters(
    State(state): State<AppState>,
    Json(request): Json<UpdateParametersRequest>,
) -> ApiResult<Json<UpdateParametersResponse>> {
    tracing::info!(?request, "Update global parameters");

    // Load current parameters from database
    let mut parameters = crate::db::parameters::load_amplitude_parameters(&state.db).await?;

    // Apply partial updates
    if let Some(val) = request.rms_window_ms {
        parameters.rms_window_ms = val;
    }
    if let Some(val) = request.lead_in_threshold_db {
        parameters.lead_in_threshold_db = val;
    }
    if let Some(val) = request.lead_out_threshold_db {
        parameters.lead_out_threshold_db = val;
    }
    if let Some(val) = request.quick_ramp_threshold {
        parameters.quick_ramp_threshold = val;
    }
    if let Some(val) = request.quick_ramp_duration_s {
        parameters.quick_ramp_duration_s = val;
    }
    if let Some(val) = request.max_lead_in_duration_s {
        parameters.max_lead_in_duration_s = val;
    }
    if let Some(val) = request.max_lead_out_duration_s {
        parameters.max_lead_out_duration_s = val;
    }
    if let Some(val) = request.apply_a_weighting {
        parameters.apply_a_weighting = val;
    }

    // Save updated parameters to database
    crate::db::parameters::save_amplitude_parameters(&state.db, &parameters).await?;

    Ok(Json(UpdateParametersResponse {
        status: "updated".to_string(),
        parameters,
    }))
}

/// Build parameter management routes
pub fn parameter_routes() -> Router<AppState> {
    Router::new()
        .route("/parameters/global", get(get_global_parameters))
        .route("/parameters/global", post(update_global_parameters))
}
