# Increment 5: wkmp-ai Settings Sync with Write-Back

**Estimated Effort:** 2-3 hours
**Dependencies:** Increment 2 (TOML write utils), Increment 3 (DB accessors), Increment 4 (resolver)
**Risk:** MEDIUM (TOML write-back, best-effort error handling)

---

## Objectives

Implement sync_settings_to_toml() with HashMap interface and write-back behavior for ENV/UI sources.

---

## Requirements Addressed

- [APIK-WB-010] through [APIK-WB-040] - Write-back behavior
- [APIK-SYNC-010], [APIK-SYNC-020] - Generic sync mechanism
- [APIK-ERR-020] - TOML write failures logged as warnings
- [APIK-LOG-030], [APIK-LOG-040] - Migration and write failure logging

---

## Deliverables

### Code Changes

**File: wkmp-ai/src/config.rs** (extend)

```rust
use std::collections::HashMap;

/// Sync settings from database to TOML file
///
/// **Traceability:** APIK-SYNC-010, APIK-WB-040
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
/// **Traceability:** APIK-WB-010, APIK-WB-020
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
```

---

### Unit Tests

**File: wkmp-ai/tests/unit/config_tests.rs** (extend)

Tests for tc_u_wb_001-006 (6 write-back tests):

```rust
// tc_u_wb_001: ENV to database write-back
// tc_u_wb_002: ENV to TOML write-back
// tc_u_wb_003: TOML to database (no TOML write)
// tc_u_wb_004: UI update to database
// tc_u_wb_005: UI update to TOML (HashMap interface)
// tc_u_wb_006: TOML write failure graceful degradation
```

Tests for tc_u_sec_002 (security warning):

```rust
// tc_u_sec_002: Permission warning logged for loose permissions
```

---

## Acceptance Criteria

- [ ] sync_settings_to_toml() implemented (HashMap interface)
- [ ] migrate_key_to_database() implemented
- [ ] ENV → Database + TOML write-back works
- [ ] TOML → Database (no TOML write) works
- [ ] UI update → Database + TOML works
- [ ] TOML write failure logs warning (doesn't fail operation)
- [ ] All unit tests pass (7 tests)

---

## Test Traceability

- tc_u_wb_001-006: Write-back behavior
- tc_u_sec_002: Security warning

---

## Rollback Plan

Remove sync_settings_to_toml() and migrate_key_to_database() functions. Resolver still works (read-only).
