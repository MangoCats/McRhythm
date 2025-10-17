//! Database models

use serde::{Deserialize, Serialize};

#[cfg(feature = "sqlx")]
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(FromRow))]
pub struct Setting {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(FromRow))]
pub struct ModuleConfig {
    pub module_name: String,
    pub host: String,
    pub port: i64,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(FromRow))]
pub struct QueueEntry {
    pub guid: String,
    pub file_path: String,
    pub passage_guid: Option<String>,
    pub play_order: i64,
    pub start_time_ms: Option<i64>,
    pub end_time_ms: Option<i64>,
    pub lead_in_point_ms: Option<i64>,
    pub lead_out_point_ms: Option<i64>,
    pub fade_in_point_ms: Option<i64>,
    pub fade_out_point_ms: Option<i64>,
    pub fade_in_curve: Option<String>,
    pub fade_out_curve: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(FromRow))]
pub struct File {
    pub guid: String,
    pub path: String,
    pub hash: String,
    pub duration: Option<f64>,
}
