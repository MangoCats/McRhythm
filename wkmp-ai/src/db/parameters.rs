//! Parameter management database operations
//!
//! Load/save global import parameters from settings table

use anyhow::Result;
use sqlx::SqlitePool;

use crate::models::AmplitudeParameters;

/// Load global amplitude parameters from database
///
/// Returns default values if not set in database
pub async fn load_amplitude_parameters(pool: &SqlitePool) -> Result<AmplitudeParameters> {
    let mut params = AmplitudeParameters::default();
    let mut loaded_count = 0;

    // Load each parameter from settings table
    if let Some(val) = get_setting_u32(pool, "rms_window_ms").await? {
        params.rms_window_ms = val;
        loaded_count += 1;
    }
    if let Some(val) = get_setting_f64(pool, "lead_in_threshold_db").await? {
        params.lead_in_threshold_db = val;
        loaded_count += 1;
    }
    if let Some(val) = get_setting_f64(pool, "lead_out_threshold_db").await? {
        params.lead_out_threshold_db = val;
        loaded_count += 1;
    }
    if let Some(val) = get_setting_f64(pool, "quick_ramp_threshold").await? {
        params.quick_ramp_threshold = val;
        loaded_count += 1;
    }
    if let Some(val) = get_setting_f64(pool, "quick_ramp_duration_s").await? {
        params.quick_ramp_duration_s = val;
        loaded_count += 1;
    }
    if let Some(val) = get_setting_f64(pool, "max_lead_in_duration_s").await? {
        params.max_lead_in_duration_s = val;
        loaded_count += 1;
    }
    if let Some(val) = get_setting_f64(pool, "max_lead_out_duration_s").await? {
        params.max_lead_out_duration_s = val;
        loaded_count += 1;
    }
    if let Some(val) = get_setting_bool(pool, "apply_a_weighting").await? {
        params.apply_a_weighting = val;
        loaded_count += 1;
    }

    tracing::info!("Loaded {} amplitude parameters from database (8 total)", loaded_count);
    Ok(params)
}

/// Save global amplitude parameters to database
pub async fn save_amplitude_parameters(
    pool: &SqlitePool,
    params: &AmplitudeParameters,
) -> Result<()> {
    tracing::info!("Saving amplitude parameters to database: {:?}", params);

    set_setting_u32(pool, "rms_window_ms", params.rms_window_ms).await?;
    set_setting_f64(pool, "lead_in_threshold_db", params.lead_in_threshold_db).await?;
    set_setting_f64(pool, "lead_out_threshold_db", params.lead_out_threshold_db).await?;
    set_setting_f64(pool, "quick_ramp_threshold", params.quick_ramp_threshold).await?;
    set_setting_f64(pool, "quick_ramp_duration_s", params.quick_ramp_duration_s).await?;
    set_setting_f64(pool, "max_lead_in_duration_s", params.max_lead_in_duration_s).await?;
    set_setting_f64(pool, "max_lead_out_duration_s", params.max_lead_out_duration_s).await?;
    set_setting_bool(pool, "apply_a_weighting", params.apply_a_weighting).await?;

    tracing::info!("Successfully saved all 8 amplitude parameters");
    Ok(())
}

// Helper functions to get/set typed settings

async fn get_setting_u32(pool: &SqlitePool, key: &str) -> Result<Option<u32>> {
    let value: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;

    match value {
        Some(v) => Ok(Some(v.parse()?)),
        None => Ok(None),
    }
}

async fn get_setting_f64(pool: &SqlitePool, key: &str) -> Result<Option<f64>> {
    let value: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;

    match value {
        Some(v) => Ok(Some(v.parse()?)),
        None => Ok(None),
    }
}

async fn get_setting_bool(pool: &SqlitePool, key: &str) -> Result<Option<bool>> {
    let value: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;

    match value {
        Some(v) => Ok(Some(v == "true" || v == "1")),
        None => Ok(None),
    }
}

async fn set_setting_u32(pool: &SqlitePool, key: &str, value: u32) -> Result<()> {
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value.to_string())
    .execute(pool)
    .await?;

    Ok(())
}

async fn set_setting_f64(pool: &SqlitePool, key: &str, value: f64) -> Result<()> {
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value.to_string())
    .execute(pool)
    .await?;

    Ok(())
}

async fn set_setting_bool(pool: &SqlitePool, key: &str, value: bool) -> Result<()> {
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(if value { "true" } else { "false" })
    .execute(pool)
    .await?;

    Ok(())
}
