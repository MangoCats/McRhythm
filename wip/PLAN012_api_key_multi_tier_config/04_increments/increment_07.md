# Increment 7: Web UI API Endpoint

**Estimated Effort:** 2-3 hours
**Dependencies:** Increment 3 (DB accessors), Increment 5 (sync)
**Risk:** LOW

---

## Objectives

Implement POST /api/settings/acoustid_api_key endpoint with validation and TOML sync.

---

## Requirements Addressed

- [APIK-UI-010], [APIK-UI-020], [APIK-UI-030] - Web UI endpoint
- [APIK-VAL-010] - Validation
- [APIK-WB-030] - UI update write-back

---

## Deliverables

### Code Changes

**File: wkmp-ai/src/api/handlers.rs** (extend)

```rust
use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct SetApiKeyRequest {
    pub api_key: String,
}

#[derive(Serialize)]
pub struct SetApiKeyResponse {
    pub success: bool,
    pub message: String,
}

/// POST /api/settings/acoustid_api_key
///
/// **Traceability:** APIK-UI-010, APIK-UI-020
pub async fn set_acoustid_api_key(
    State(db): State<Pool<Sqlite>>,
    State(toml_path): State<PathBuf>,
    Json(payload): Json<SetApiKeyRequest>,
) -> Json<SetApiKeyResponse> {
    // Validate
    if payload.api_key.trim().is_empty() {
        return Json(SetApiKeyResponse {
            success: false,
            message: "API key cannot be empty".to_string(),
        });
    }

    // Write to database
    match crate::db::settings::set_acoustid_api_key(&db, payload.api_key.clone()).await {
        Ok(()) => {
            // Sync to TOML (best-effort)
            let mut settings = HashMap::new();
            settings.insert("acoustid_api_key".to_string(), payload.api_key);
            let _ = crate::config::sync_settings_to_toml(settings, &toml_path).await;

            Json(SetApiKeyResponse {
                success: true,
                message: "API key saved successfully".to_string(),
            })
        }
        Err(e) => {
            Json(SetApiKeyResponse {
                success: false,
                message: format!("Failed to save API key: {}", e),
            })
        }
    }
}
```

**File: wkmp-ai/src/api/mod.rs** (extend router)

```rust
.route("/api/settings/acoustid_api_key", post(handlers::set_acoustid_api_key))
```

---

### Integration Tests

**File: wkmp-ai/tests/integration/ui_endpoint_tests.rs** (new)

Tests for tc_i_ui_001-003:

```rust
// tc_i_ui_001: POST success
// tc_i_ui_002: POST empty key error
// tc_i_ui_003: POST writes to database and TOML
```

---

## Acceptance Criteria

- [ ] POST /api/settings/acoustid_api_key endpoint implemented
- [ ] Validation rejects empty keys
- [ ] Success response: {success: true, message: "..."}
- [ ] Error response: {success: false, message: "..."}
- [ ] Writes to database and TOML
- [ ] All integration tests pass (3 tests)

---

## Test Traceability

- tc_i_ui_001-003: Web UI endpoint

---

## Rollback Plan

Remove route and handler. No impact on existing functionality.
