# Increment 6: wkmp-ai Startup Integration

**Estimated Effort:** 2-3 hours
**Dependencies:** Increments 1-5 (resolver + write-back)
**Risk:** MEDIUM (integration with existing startup)

---

## Objectives

Integrate resolve_acoustid_api_key() into wkmp-ai startup sequence with auto-migration.

---

## Requirements Addressed

- [APIK-ARCH-030] - Module startup integration
- [APIK-MIG-010] - ENV deployments auto-migrate
- [APIK-LOG-010], [APIK-LOG-030] - Logging

---

## Deliverables

### Code Changes

**File: wkmp-ai/src/main.rs** (modify)

```rust
// After database initialization, before AcoustID client creation

// Resolve API key with auto-migration
let toml_path = /* derive from root folder resolver */;
let api_key = match config::resolve_acoustid_api_key(&db, &toml_config).await {
    Ok(key) => {
        // Check if migration needed (ENV or TOML source, database empty)
        let db_key = db::settings::get_acoustid_api_key(&db).await?;
        if db_key.is_none() {
            // Auto-migrate to database
            let source = if std::env::var("WKMP_ACOUSTID_API_KEY").is_ok() {
                "environment"
            } else {
                "TOML"
            };
            config::migrate_key_to_database(key.clone(), source, &db, &toml_path).await?;
        }
        key
    }
    Err(e) => {
        error!("Failed to resolve AcoustID API key: {}", e);
        return Err(e);
    }
};

// Check TOML permissions (security warning)
if let Err(e) = check_toml_permissions(&toml_path) {
    warn!("TOML permission check: {}", e);
}

// Create AcoustID client with resolved key
let acoustid_client = AcoustidClient::new(api_key);
```

---

### Integration Tests

**File: wkmp-ai/tests/integration/startup_tests.rs** (new)

Tests for tc_i_e2e_001-004 (4 end-to-end tests):

```rust
// tc_i_e2e_001: Startup with database key
// tc_i_e2e_002: Startup with ENV migration
// tc_i_e2e_003: Startup with TOML migration
// tc_i_e2e_004: Startup error on no key
```

---

## Acceptance Criteria

- [ ] resolve_acoustid_api_key() called at startup
- [ ] Auto-migration from ENV to database + TOML
- [ ] Auto-migration from TOML to database
- [ ] Startup fails with clear error if no key
- [ ] TOML permission warnings logged
- [ ] All integration tests pass (4 tests)

---

## Test Traceability

- tc_i_e2e_001-004: End-to-end startup

---

## Rollback Plan

Revert main.rs changes. Return to hardcoded key or ENV-only approach.
