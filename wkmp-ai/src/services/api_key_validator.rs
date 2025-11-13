//! AcoustID API Key Validation
//!
//! **Traceability:** [REQ-SPEC032-004] AcoustID API Key Validation (Step 1)
//!
//! Validates AcoustID API key at workflow start before file scanning.

use sqlx::{Pool, Sqlite};
use wkmp_common::{Error, Result};

use crate::db::settings;

/// API key validation outcome
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationResult {
    /// Valid API key
    Valid,
    /// Invalid API key (HTTP 401 or other validation failure)
    Invalid,
    /// Missing API key (NULL in database)
    Missing,
}

/// User choice after validation failure
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserChoice {
    /// User provided a new API key to validate
    ProvideKey(String),
    /// User acknowledged lack of key, skip fingerprinting for this session
    SkipFingerprinting,
}

/// API Key Validator
///
/// **Traceability:** [REQ-SPEC032-004] (Step 1: API Key Validation)
pub struct ApiKeyValidator {
    db: Pool<Sqlite>,
}

impl ApiKeyValidator {
    /// Create new API key validator
    pub fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }

    /// Validate stored AcoustID API key
    ///
    /// **Algorithm:**
    /// 1. Read `acoustid_api_key` from settings table
    /// 2. If NULL: Return ValidationResult::Missing
    /// 3. If present: Perform test lookup to validate key
    /// 4. Return ValidationResult::Valid or ValidationResult::Invalid
    ///
    /// **Traceability:** [REQ-SPEC032-004]
    pub async fn validate_stored_key(&self) -> Result<ValidationResult> {
        tracing::debug!("Validating stored AcoustID API key");

        // Read stored API key
        let api_key = settings::get_acoustid_api_key(&self.db).await?;

        match api_key {
            None => {
                tracing::debug!("AcoustID API key is NULL in database");
                Ok(ValidationResult::Missing)
            }
            Some(key) => {
                tracing::debug!(key_len = key.len(), "Found stored API key, validating");
                self.validate_key(&key).await
            }
        }
    }

    /// Validate a specific API key by performing test lookup
    ///
    /// **Implementation:** Perform a test AcoustID lookup with a known-valid fingerprint
    /// to check if the API key is accepted by the AcoustID service.
    ///
    /// **Note:** This is a placeholder implementation. The actual implementation should:
    /// - Use a test fingerprint (could be a short, known-valid fingerprint)
    /// - Call AcoustID API with the test fingerprint
    /// - Check for HTTP 401 (invalid key) vs. HTTP 200 (valid key)
    ///
    /// **Traceability:** [REQ-SPEC032-004]
    pub async fn validate_key(&self, key: &str) -> Result<ValidationResult> {
        tracing::debug!(key_len = key.len(), "Validating API key via test lookup");

        // Test fingerprint (short, valid Chromaprint fingerprint for testing)
        // This is a minimal valid fingerprint format for AcoustID
        let test_fingerprint = "AQAAA0mUaEkSRYnGQ_wW";
        let test_duration = 30; // 30 seconds

        // Build test request
        let client = reqwest::Client::new();
        let params = [
            ("client", key),
            ("meta", "recordings"),
            ("duration", &test_duration.to_string()),
            ("fingerprint", test_fingerprint),
        ];

        let response = client
            .post("https://api.acoustid.org/v2/lookup")
            .form(&params)
            .send()
            .await
            .map_err(|e| Error::Internal(format!("AcoustID API test request failed: {}", e)))?;

        let status = response.status();

        tracing::debug!(
            status_code = status.as_u16(),
            "AcoustID API validation response"
        );

        if status == 401 {
            tracing::warn!("AcoustID API key is invalid (HTTP 401)");
            Ok(ValidationResult::Invalid)
        } else if status.is_success() {
            tracing::info!("AcoustID API key is valid");
            Ok(ValidationResult::Valid)
        } else {
            // Other error codes (e.g., 500, 503) treated as validation failure
            tracing::warn!(
                status_code = status.as_u16(),
                "AcoustID API validation failed with non-401 error"
            );
            Ok(ValidationResult::Invalid)
        }
    }

    /// Store new API key in database
    ///
    /// **Traceability:** [REQ-SPEC032-004]
    pub async fn store_key(&self, key: String) -> Result<()> {
        tracing::debug!(key_len = key.len(), "Storing new AcoustID API key");
        settings::set_acoustid_api_key(&self.db, key).await
    }

    /// Handle user choice after validation failure
    ///
    /// **Algorithm:**
    /// - If ProvideKey: Validate new key, store if valid, return validation result
    /// - If SkipFingerprinting: Log warning, return Ok (workflow continues without fingerprinting)
    ///
    /// **Traceability:** [REQ-SPEC032-004]
    pub async fn handle_user_choice(&self, choice: UserChoice) -> Result<ValidationResult> {
        match choice {
            UserChoice::ProvideKey(new_key) => {
                tracing::debug!("User provided new API key, validating");
                let result = self.validate_key(&new_key).await?;

                if result == ValidationResult::Valid {
                    self.store_key(new_key).await?;
                    tracing::info!("New API key validated and stored");
                } else {
                    tracing::warn!("User-provided API key is invalid");
                }

                Ok(result)
            }
            UserChoice::SkipFingerprinting => {
                tracing::warn!(
                    "User chose to skip fingerprinting for this session. \
                     Identification accuracy will be reduced (metadata-only matching)."
                );
                Ok(ValidationResult::Missing)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    /// Setup in-memory test database with settings table
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create settings table matching production schema
        sqlx::query(
            "CREATE TABLE settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_validate_stored_key_missing() {
        let pool = setup_test_db().await;
        let validator = ApiKeyValidator::new(pool);

        let result = validator.validate_stored_key().await.unwrap();

        assert_eq!(result, ValidationResult::Missing);
    }

    #[tokio::test]
    async fn test_store_and_validate_key() {
        let pool = setup_test_db().await;
        let validator = ApiKeyValidator::new(pool.clone());

        // Store a test key
        let test_key = "test_key_123".to_string();
        validator.store_key(test_key.clone()).await.unwrap();

        // Verify key was stored
        let stored_key = settings::get_acoustid_api_key(&pool).await.unwrap();
        assert_eq!(stored_key, Some(test_key));
    }

    #[tokio::test]
    async fn test_handle_user_choice_skip() {
        let pool = setup_test_db().await;
        let validator = ApiKeyValidator::new(pool);

        let result = validator
            .handle_user_choice(UserChoice::SkipFingerprinting)
            .await
            .unwrap();

        assert_eq!(result, ValidationResult::Missing);
    }
}
