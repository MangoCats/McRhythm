//! API authentication via timestamp and hash validation
//!
//! Implements API-AUTH-025 through API-AUTH-031 per SPEC007
//!
//! # Architecture
//!
//! Per SPEC007 API-AUTH-025:
//! - All API requests include timestamp (i64 Unix epoch ms) and hash (SHA-256)
//! - Timestamp must be within ≤1000ms past and ≤1ms future
//! - Hash calculated from canonical JSON + shared secret
//! - Shared secret stored in database settings table
//! - Can be disabled by setting shared_secret = 0
//!
//! # Pure Functions
//!
//! This module contains ONLY pure functions and database operations.
//! No HTTP framework dependencies (Axum, etc.) - those are in module-specific code.

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "sqlx")]
use sqlx::SqlitePool;

// ========================================
// Error Types
// ========================================

/// Authentication error types
///
/// Per SPEC007 API-AUTH-029: Defines error conditions for auth validation
#[derive(Debug, Clone)]
pub enum ApiAuthError {
    /// Timestamp outside acceptable window
    InvalidTimestamp {
        timestamp: i64,
        now: i64,
        reason: String,
    },

    /// Hash does not match calculated value
    InvalidHash { provided: String, calculated: String },

    /// Timestamp field missing from request
    MissingTimestamp,

    /// Hash field missing from request
    MissingHash,

    /// Database error loading shared secret
    DatabaseError(String),

    /// Failed to parse request body
    ParseError(String),
}

impl std::fmt::Display for ApiAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiAuthError::InvalidTimestamp { reason, .. } => {
                write!(f, "Invalid timestamp: {}", reason)
            }
            ApiAuthError::InvalidHash { .. } => write!(f, "Invalid hash"),
            ApiAuthError::MissingTimestamp => write!(f, "Missing timestamp field"),
            ApiAuthError::MissingHash => write!(f, "Missing hash field"),
            ApiAuthError::DatabaseError(err) => write!(f, "Database error: {}", err),
            ApiAuthError::ParseError(err) => write!(f, "Parse error: {}", err),
        }
    }
}

impl std::error::Error for ApiAuthError {}

// ========================================
// Shared Secret Management
// ========================================

/// Load shared secret from database settings
///
/// Per API-AUTH-028:
/// - Key: `api_shared_secret`
/// - Value: i64
/// - Special value 0: Disables auth checking
///
/// # Examples
///
/// ```ignore
/// let secret = load_shared_secret(&db).await?;
/// if secret == 0 {
///     // Auth disabled
/// }
/// ```
#[cfg(feature = "sqlx")]
pub async fn load_shared_secret(db: &SqlitePool) -> Result<i64, ApiAuthError> {
    let result: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM settings WHERE key = 'api_shared_secret'",
    )
    .fetch_optional(db)
    .await
    .map_err(|e| ApiAuthError::DatabaseError(e.to_string()))?;

    match result {
        Some((value,)) => {
            // Parse value as i64
            value
                .parse::<i64>()
                .map_err(|e| ApiAuthError::DatabaseError(format!("Invalid i64: {}", e)))
        }
        None => {
            // Not found - generate and store new secret
            initialize_shared_secret(db).await
        }
    }
}

/// Initialize shared secret if not present
///
/// Per API-AUTH-028: Generate cryptographically random i64 (non-zero)
///
/// # Examples
///
/// ```ignore
/// let secret = initialize_shared_secret(&db).await?;
/// assert_ne!(secret, 0);
/// ```
#[cfg(feature = "sqlx")]
pub async fn initialize_shared_secret(db: &SqlitePool) -> Result<i64, ApiAuthError> {
    use rand::Rng;

    // Generate crypto-random i64 (non-zero)
    let mut rng = rand::thread_rng();
    let secret: i64 = loop {
        let val = rng.gen::<i64>();
        if val != 0 {
            break val;
        }
    };

    // Store in database
    sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES ('api_shared_secret', ?)")
        .bind(secret.to_string())
        .execute(db)
        .await
        .map_err(|e| ApiAuthError::DatabaseError(e.to_string()))?;

    Ok(secret)
}

// ========================================
// Timestamp Validation
// ========================================

/// Validate timestamp per API-AUTH-029, API-AUTH-030
///
/// # Rules
///
/// - Timestamp must be ≤1000ms in the past
/// - Timestamp must be ≤1ms in the future
/// - Per API-AUTH-030: Asymmetry is intentional
///   - Past tolerance: Allows processing delay
///   - Future tolerance: Minimal (clock drift only)
///
/// # Examples
///
/// ```
/// use wkmp_common::api::auth::validate_timestamp;
/// use std::time::{SystemTime, UNIX_EPOCH};
///
/// let now = SystemTime::now()
///     .duration_since(UNIX_EPOCH)
///     .unwrap()
///     .as_millis() as i64;
///
/// // Current time is valid
/// assert!(validate_timestamp(now).is_ok());
///
/// // 500ms ago is valid
/// assert!(validate_timestamp(now - 500).is_ok());
///
/// // 2000ms ago is invalid
/// assert!(validate_timestamp(now - 2000).is_err());
/// ```
pub fn validate_timestamp(timestamp: i64) -> Result<(), ApiAuthError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let diff = now - timestamp;

    // Check if too far in the past (>1000ms)
    if diff > 1000 {
        return Err(ApiAuthError::InvalidTimestamp {
            timestamp,
            now,
            reason: format!("Timestamp {}ms too old (max 1000ms past)", diff),
        });
    }

    // Check if in the future (>1ms)
    if diff < -1 {
        return Err(ApiAuthError::InvalidTimestamp {
            timestamp,
            now,
            reason: format!("Timestamp {}ms in future (max 1ms future)", diff.abs()),
        });
    }

    Ok(())
}

// ========================================
// Hash Calculation and Validation
// ========================================

/// Calculate hash per API-AUTH-027
///
/// # Algorithm
///
/// 1. Replace hash field with dummy hash (64 zeros)
/// 2. Convert to canonical JSON (sorted keys, no whitespace)
/// 3. Append shared secret as decimal i64 string
/// 4. Calculate SHA-256 of concatenated string
/// 5. Return as 64 hex characters
///
/// # Examples
///
/// ```
/// use wkmp_common::api::auth::calculate_hash;
/// use serde_json::json;
///
/// let json = json!({
///     "file_path": "music.mp3",
///     "timestamp": 1730000000000i64,
///     "hash": "dummy"
/// });
///
/// let hash = calculate_hash(&json, 123456789);
/// assert_eq!(hash.len(), 64); // SHA-256 is 64 hex chars
/// ```
pub fn calculate_hash(json_value: &Value, shared_secret: i64) -> String {
    // Step 1: Replace hash with dummy hash
    let mut value = json_value.clone();
    if let Some(obj) = value.as_object_mut() {
        obj.insert(
            "hash".to_string(),
            Value::String(
                "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            ),
        );
    }

    // Step 2: Canonical JSON (sorted keys, no whitespace)
    let canonical = to_canonical_json(&value);

    // Step 3: Append shared secret as decimal string
    let to_hash = format!("{}{}", canonical, shared_secret);

    // Step 4: Calculate SHA-256
    let mut hasher = Sha256::new();
    hasher.update(to_hash.as_bytes());
    let result = hasher.finalize();

    // Step 5: Convert to 64 hex characters
    format!("{:x}", result)
}

/// Convert JSON to canonical form (sorted keys, no whitespace)
///
/// Per API-AUTH-027: Keys must be sorted alphabetically
///
/// # Examples
///
/// ```
/// use wkmp_common::api::auth::to_canonical_json;
/// use serde_json::json;
///
/// let json = json!({"z": 3, "a": 1, "m": 2});
/// let canonical = to_canonical_json(&json);
///
/// // Keys are alphabetically sorted
/// assert!(canonical.starts_with("{\"a\":"));
/// assert!(canonical.contains("\"m\":"));
/// assert!(canonical.contains("\"z\":"));
/// ```
pub fn to_canonical_json(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut pairs: Vec<_> = map.iter().collect();
            pairs.sort_by_key(|(k, _)| *k);
            let items: Vec<String> = pairs
                .into_iter()
                .map(|(k, v)| format!("\"{}\":{}", k, to_canonical_json(v)))
                .collect();
            format!("{{{}}}", items.join(","))
        }
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(|v| to_canonical_json(v)).collect();
            format!("[{}]", items.join(","))
        }
        Value::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
    }
}

/// Validate hash matches calculated value
///
/// # Examples
///
/// ```
/// use wkmp_common::api::auth::{calculate_hash, validate_hash};
/// use serde_json::json;
///
/// let json = json!({
///     "file_path": "music.mp3",
///     "timestamp": 1730000000000i64,
///     "hash": "dummy"
/// });
///
/// let secret = 123456789i64;
/// let calculated = calculate_hash(&json, secret);
///
/// // Validation succeeds with correct hash
/// assert!(validate_hash(&calculated, &json, secret).is_ok());
///
/// // Validation fails with wrong hash
/// let wrong = "0000000000000000000000000000000000000000000000000000000000000000";
/// assert!(validate_hash(wrong, &json, secret).is_err());
/// ```
pub fn validate_hash(
    provided_hash: &str,
    json_value: &Value,
    shared_secret: i64,
) -> Result<(), ApiAuthError> {
    let calculated = calculate_hash(json_value, shared_secret);

    if provided_hash != calculated {
        return Err(ApiAuthError::InvalidHash {
            provided: provided_hash.to_string(),
            calculated,
        });
    }

    Ok(())
}

// ========================================
// Tests
// ========================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_timestamp_accepted() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // Current time should be valid
        assert!(validate_timestamp(now).is_ok());

        // 500ms ago should be valid (≤1000ms)
        assert!(validate_timestamp(now - 500).is_ok());

        // 1000ms ago should be valid (boundary)
        assert!(validate_timestamp(now - 1000).is_ok());
    }

    #[test]
    fn test_timestamp_too_old_rejected() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // 1001ms ago should be rejected (>1000ms)
        assert!(validate_timestamp(now - 1001).is_err());

        // 2000ms ago should be rejected
        assert!(validate_timestamp(now - 2000).is_err());
    }

    #[test]
    fn test_timestamp_future_rejected() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // 1ms future should be valid (boundary)
        assert!(validate_timestamp(now + 1).is_ok());

        // 2ms future should be rejected (>1ms)
        assert!(validate_timestamp(now + 2).is_err());

        // 100ms future should be rejected
        assert!(validate_timestamp(now + 100).is_err());
    }

    #[test]
    fn test_hash_calculation_algorithm() {
        let json = serde_json::json!({
            "file_path": "music.mp3",
            "timestamp": 1730000000000i64,
            "hash": "0000000000000000000000000000000000000000000000000000000000000000"
        });

        let shared_secret = 123456789i64;
        let hash = calculate_hash(&json, shared_secret);

        // Hash should be 64 hex characters
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));

        // Same input should produce same hash
        let hash2 = calculate_hash(&json, shared_secret);
        assert_eq!(hash, hash2);

        // Different secret should produce different hash
        let hash3 = calculate_hash(&json, 987654321);
        assert_ne!(hash, hash3);
    }

    #[test]
    fn test_canonical_json_sorting() {
        // Keys should be sorted alphabetically
        let json = serde_json::json!({
            "z_field": "last",
            "a_field": "first",
            "m_field": "middle"
        });

        let canonical = to_canonical_json(&json);

        // Should have fields in alphabetical order
        assert!(canonical.contains("\"a_field\":"));
        assert!(canonical.contains("\"m_field\":"));
        assert!(canonical.contains("\"z_field\":"));

        // a_field should come before m_field
        let a_pos = canonical.find("\"a_field\"").unwrap();
        let m_pos = canonical.find("\"m_field\"").unwrap();
        let z_pos = canonical.find("\"z_field\"").unwrap();
        assert!(a_pos < m_pos);
        assert!(m_pos < z_pos);
    }

    #[test]
    fn test_valid_hash_accepted() {
        let json = serde_json::json!({
            "file_path": "music.mp3",
            "timestamp": 1730000000000i64,
            "hash": "dummy"
        });

        let shared_secret = 123456789i64;
        let calculated = calculate_hash(&json, shared_secret);

        // Validation should succeed with correct hash
        assert!(validate_hash(&calculated, &json, shared_secret).is_ok());
    }

    #[test]
    fn test_invalid_hash_rejected() {
        let json = serde_json::json!({
            "file_path": "music.mp3",
            "timestamp": 1730000000000i64,
            "hash": "dummy"
        });

        let shared_secret = 123456789i64;
        let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";

        // Validation should fail with wrong hash
        assert!(validate_hash(wrong_hash, &json, shared_secret).is_err());
    }

    #[test]
    fn test_canonical_json_no_whitespace() {
        let json = serde_json::json!({
            "field1": "value1",
            "field2": 42
        });

        let canonical = to_canonical_json(&json);

        // Should have no whitespace
        assert!(!canonical.contains(' '));
        assert!(!canonical.contains('\n'));
        assert!(!canonical.contains('\t'));
    }
}
