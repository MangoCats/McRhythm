# Increment 4: wkmp-ai Multi-Tier Resolver

**Estimated Effort:** 3-4 hours
**Dependencies:** Increment 1 (TomlConfig), Increment 3 (DB accessors)
**Risk:** MEDIUM (multi-tier logic complexity)

---

## Objectives

Implement resolve_acoustid_api_key() with 3-tier priority resolution and validation.

---

## Requirements Addressed

- [APIK-RES-010] through [APIK-RES-050] - Multi-tier resolution
- [APIK-VAL-010] - Validation (empty, whitespace, NULL)
- [APIK-ERR-010] - Comprehensive error message
- [APIK-LOG-010], [APIK-LOG-020] - Logging
- [APIK-ACID-010], [APIK-ACID-020], [APIK-ACID-030] - AcoustID specifics

---

## Deliverables

### Code Changes

**File: wkmp-ai/src/config.rs** (new)

```rust
use wkmp_common::{Error, Result};
use wkmp_common::config::TomlConfig;
use sqlx::{Pool, Sqlite};
use tracing::{info, warn};
use std::path::Path;

/// Resolve AcoustID API key from 3-tier configuration
///
/// **Priority:** Database → ENV → TOML
///
/// **Traceability:** APIK-RES-010, APIK-ACID-010
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
    Err(Error::Config(format!(
        "AcoustID API key not configured. Please configure using one of:\n\
         1. Web UI: http://localhost:5723/settings\n\
         2. Environment: WKMP_ACOUSTID_API_KEY=your-key-here\n\
         3. TOML config: ~/.config/wkmp/wkmp-ai.toml (acoustid_api_key = \"your-key\")\n\
         \n\
         Obtain API key at: https://acoustid.org/api-key"
    )))
}

/// Validate API key (non-empty, non-whitespace)
///
/// **Traceability:** APIK-VAL-010
fn is_valid_key(key: &str) -> bool {
    !key.trim().is_empty()
}
```

---

### Unit Tests

**File: wkmp-ai/tests/unit/config_tests.rs** (new)

Tests for tc_u_res_001-008 (8 resolution tests):

```rust
// tc_u_res_001: Database priority
#[tokio::test]
async fn test_database_overrides_env_and_toml() {
    // Setup: DB="db-key", ENV="env-key", TOML="toml-key"
    // Expected: "db-key"
}

// tc_u_res_002: ENV fallback
#[tokio::test]
async fn test_env_fallback_when_database_empty() {
    // Setup: DB=None, ENV="env-key", TOML="toml-key"
    // Expected: "env-key"
}

// tc_u_res_003: TOML fallback
#[tokio::test]
async fn test_toml_fallback_when_db_and_env_empty() {
    // Setup: DB=None, ENV=None, TOML="toml-key"
    // Expected: "toml-key"
}

// tc_u_res_004: Error on no key
#[tokio::test]
async fn test_error_when_no_key_found() {
    // Setup: DB=None, ENV=None, TOML=None
    // Expected: Err with helpful message
}

// tc_u_res_005: Database ignores ENV when present
#[tokio::test]
async fn test_database_ignores_env() {
    // Setup: DB="db-key", ENV="env-key"
    // Expected: "db-key" (no ENV check)
}

// tc_u_res_006: Database ignores TOML when present
#[tokio::test]
async fn test_database_ignores_toml() {
    // Setup: DB="db-key", TOML="toml-key"
    // Expected: "db-key" (no TOML check)
}

// tc_u_res_007: ENV ignores TOML when present
#[tokio::test]
async fn test_env_ignores_toml() {
    // Setup: DB=None, ENV="env-key", TOML="toml-key"
    // Expected: "env-key" (no TOML check)
}

// tc_u_res_008: Multiple sources warning logged
#[tokio::test]
async fn test_multiple_sources_warning() {
    // Setup: DB="db-key", ENV="env-key", TOML="toml-key"
    // Expected: Warning logged, returns "db-key"
}
```

Tests for tc_u_val_001-003 (3 validation tests):

```rust
// tc_u_val_001: Empty key rejected
#[test]
fn test_empty_key_rejected() {
    assert!(!is_valid_key(""));
}

// tc_u_val_002: Whitespace-only key rejected
#[test]
fn test_whitespace_key_rejected() {
    assert!(!is_valid_key("   \t\n"));
}

// tc_u_val_003: Valid key accepted
#[test]
fn test_valid_key_accepted() {
    assert!(is_valid_key("valid-key-123"));
}
```

---

## Verification Steps

1. All unit tests pass (11 tests: 8 resolution + 3 validation)
2. Verify tier priority (database > env > toml)
3. Verify error message includes all 3 methods
4. Verify logging shows resolution source

---

## Acceptance Criteria

- [ ] resolve_acoustid_api_key() implemented
- [ ] 3-tier priority works (database → env → toml)
- [ ] Validation rejects empty/whitespace keys
- [ ] Error message lists all configuration methods
- [ ] Logging shows resolution source
- [ ] Multiple sources warning logged
- [ ] All unit tests pass (11 tests)

---

## Test Traceability

- tc_u_res_001-008: Multi-tier resolution
- tc_u_val_001-003: Validation

---

## Rollback Plan

If increment fails:
- Remove wkmp-ai/src/config.rs
- No downstream impact (not integrated with startup yet)
