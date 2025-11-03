//! Configuration resolution for wkmp-ai
//!
//! **Traceability:** [APIK-RES-010] through [APIK-RES-050], [APIK-VAL-010]
//!
//! Provides multi-tier configuration resolution with Database → ENV → TOML priority.

use wkmp_common::{Error, Result};
use wkmp_common::config::TomlConfig;
use sqlx::{Pool, Sqlite};
use std::collections::HashMap;
use std::path::Path;
use tracing::{info, warn};

/// Resolve AcoustID API key from 3-tier configuration
///
/// **Priority:** Database → ENV → TOML
///
/// **Traceability:** [APIK-RES-010], [APIK-ACID-010]
pub async fn resolve_acoustid_api_key(
    db: &Pool<Sqlite>,
    toml_config: &TomlConfig,
) -> Result<String> {
    let mut sources = Vec::new();

    // Tier 1: Database (authoritative)
    let db_key = crate::db::settings::get_acoustid_api_key(db).await?;
    if let Some(key) = &db_key {
        if is_valid_key(key) {
            sources.push("database");
        }
    }

    // Tier 2: Environment variable
    let env_key = std::env::var("WKMP_ACOUSTID_API_KEY").ok();
    if let Some(key) = &env_key {
        if is_valid_key(key) {
            sources.push("environment");
        }
    }

    // Tier 3: TOML config
    let toml_key = toml_config.acoustid_api_key.as_ref();
    if let Some(key) = toml_key {
        if is_valid_key(key) {
            sources.push("TOML");
        }
    }

    // Warn if multiple sources (potential misconfiguration)
    if sources.len() > 1 {
        warn!(
            "AcoustID API key found in multiple sources: {}. Using database (highest priority).",
            sources.join(", ")
        );
    }

    // Resolution priority
    if let Some(key) = db_key {
        if is_valid_key(&key) {
            info!("AcoustID API key loaded from database");
            return Ok(key);
        }
    }

    if let Some(key) = env_key {
        if is_valid_key(&key) {
            info!("AcoustID API key loaded from environment variable");
            return Ok(key);
        }
    }

    if let Some(key) = toml_key {
        if is_valid_key(key) {
            info!("AcoustID API key loaded from TOML config");
            return Ok(key.clone());
        }
    }

    // No valid key found
    Err(Error::Config("AcoustID API key not configured. Please configure using one of:\n\
         1. Web UI: http://localhost:5723/settings\n\
         2. Environment: WKMP_ACOUSTID_API_KEY=your-key-here\n\
         3. TOML config: ~/.config/wkmp/wkmp-ai.toml (acoustid_api_key = \"your-key\")\n\
         \n\
         Obtain API key at: https://acoustid.org/api-key".to_string()))
}

/// Validate API key (non-empty, non-whitespace)
///
/// **Traceability:** [APIK-VAL-010]
pub fn is_valid_key(key: &str) -> bool {
    !key.trim().is_empty()
}

// ============================================================================
// Settings Sync and Write-Back
// ============================================================================

/// Sync settings from database to TOML file
///
/// **Traceability:** [APIK-SYNC-010], [APIK-WB-040]
///
/// HashMap keys: "acoustid_api_key", etc. (future: "musicbrainz_token")
pub async fn sync_settings_to_toml(
    settings: HashMap<String, String>,
    toml_path: &Path,
) -> Result<()> {
    // Read existing TOML (or use defaults)
    let mut config = if toml_path.exists() {
        let content = std::fs::read_to_string(toml_path)
            .map_err(|e| Error::Config(format!("Read TOML failed: {}", e)))?;
        toml::from_str(&content)
            .map_err(|e| Error::Config(format!("Parse TOML failed: {}", e)))?
    } else {
        TomlConfig {
            root_folder: None,
            logging: Default::default(),
            static_assets: None,
            acoustid_api_key: None,
        }
    };

    // Update fields from HashMap
    if let Some(key) = settings.get("acoustid_api_key") {
        config.acoustid_api_key = Some(key.clone());
    }

    // Write atomically (best-effort)
    match wkmp_common::config::write_toml_config(&config, toml_path) {
        Ok(()) => {
            info!("Settings synced to TOML: {}", toml_path.display());
            Ok(())
        }
        Err(e) => {
            warn!("TOML write failed (database write succeeded): {}", e);
            Ok(()) // Graceful degradation
        }
    }
}

/// Perform auto-migration from ENV/TOML to database + TOML
///
/// **Traceability:** [APIK-WB-010], [APIK-WB-020]
pub async fn migrate_key_to_database(
    key: String,
    source: &str,
    db: &Pool<Sqlite>,
    toml_path: &Path,
) -> Result<()> {
    // Write to database (authoritative)
    crate::db::settings::set_acoustid_api_key(db, key.clone()).await?;

    // Write to TOML if source was ENV (backup)
    if source == "environment" {
        let mut settings = HashMap::new();
        settings.insert("acoustid_api_key".to_string(), key);
        sync_settings_to_toml(settings, toml_path).await?;
    }

    info!("AcoustID API key migrated from {} to database", source);
    Ok(())
}
