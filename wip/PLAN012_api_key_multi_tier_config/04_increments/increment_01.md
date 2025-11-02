# Increment 1: wkmp-common TOML Schema Extension

**Estimated Effort:** 1-2 hours
**Dependencies:** None (foundation increment)
**Risk:** LOW

---

## Objectives

Extend wkmp-common TomlConfig struct to support acoustid_api_key field with proper serde attributes.

---

## Requirements Addressed

- [APIK-TOML-SCHEMA-010] - Extend TomlConfig struct
- [APIK-TOML-SCHEMA-020] - Maintain backward compatibility
- [APIK-ARCH-020] - wkmp-common provides TOML schema

---

## Deliverables

### Code Changes

**File: wkmp-common/src/config.rs**

Add field to TomlConfig struct:
```rust
#[derive(Debug, Deserialize, Serialize)]
pub struct TomlConfig {
    pub root_folder: Option<PathBuf>,

    #[serde(default)]
    pub logging: LoggingConfig,

    pub static_assets: Option<PathBuf>,

    // NEW FIELD
    /// AcoustID API key for audio fingerprinting
    pub acoustid_api_key: Option<String>,
}
```

**Important:** Add `Serialize` to derive macro (currently only has `Deserialize`)

---

### Unit Tests

**File: wkmp-common/tests/config_tests.rs** (extend existing or create)

**Test: Round-trip serialization preserves acoustid_api_key**
```rust
#[test]
fn test_toml_roundtrip_with_acoustid_key() {
    let config = TomlConfig {
        root_folder: Some(PathBuf::from("/music")),
        logging: LoggingConfig::default(),
        static_assets: None,
        acoustid_api_key: Some("test-key-123".to_string()),
    };

    let toml_str = toml::to_string(&config).unwrap();
    let parsed: TomlConfig = toml::from_str(&toml_str).unwrap();

    assert_eq!(parsed.acoustid_api_key, Some("test-key-123".to_string()));
}
```

**Test: Missing acoustid_api_key field deserializes as None**
```rust
#[test]
fn test_backward_compatible_missing_field() {
    let toml_str = r#"
        root_folder = "/music"
        [logging]
        level = "info"
    "#;

    let config: TomlConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.acoustid_api_key, None);
}
```

---

## Verification Steps

1. Cargo check passes (no compiler errors)
2. Existing wkmp-common tests still pass (backward compatibility)
3. New unit tests pass (roundtrip, missing field)
4. TOML files without acoustid_api_key still parse correctly

---

## Acceptance Criteria

- [ ] TomlConfig struct has acoustid_api_key: Option<String> field
- [ ] Derive includes Serialize trait
- [ ] Backward compatibility maintained (existing TOML files parse without error)
- [ ] Unit tests verify roundtrip serialization
- [ ] Unit tests verify missing field defaults to None
- [ ] All existing wkmp-common tests pass

---

## Test Traceability

- tc_u_toml_006 (roundtrip serialization)
- tc_u_toml_003 (field preservation - backward compatibility)

---

## Implementation Notes

**Why Option<String>?**
- Allows TOML files without key to parse successfully
- None indicates key not configured in TOML (fallback to ENV or database)
- Consistent with other optional fields (root_folder, static_assets)

**Serialize Trait:**
- Currently TomlConfig only derives Deserialize (read-only)
- Must add Serialize to support writing TOML files
- This enables write-back behavior (database → TOML sync)

---

## Rollback Plan

If increment fails:
- Revert wkmp-common/src/config.rs changes
- Remove test file if created
- No downstream impact (no other modules depend on this field yet)
