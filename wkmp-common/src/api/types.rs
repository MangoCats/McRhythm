//! Shared API request/response types
//!
//! Types used across all WKMP modules for authentication and error responses.
//!
//! # Architecture
//!
//! These types are used by all 5 WKMP microservices:
//! - wkmp-ap (Audio Player)
//! - wkmp-ui (User Interface)
//! - wkmp-pd (Program Director)
//! - wkmp-ai (Audio Ingest)
//! - wkmp-le (Lyric Editor)

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ========================================
// Authentication Types
// ========================================

/// Authentication parameters for GET requests (query parameters)
///
/// Per SPEC007 API-AUTH-025: All API requests include timestamp and hash
///
/// # Examples
///
/// ```
/// // GET /playback/queue?timestamp=1730000000000&hash=abc123...
/// use wkmp_common::api::types::AuthQuery;
///
/// let query = AuthQuery {
///     timestamp: 1730000000000,
///     hash: "abc123...".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthQuery {
    /// Unix epoch time in milliseconds (API-AUTH-025)
    pub timestamp: i64,

    /// SHA-256 hash (64 hex chars) (API-AUTH-025)
    pub hash: String,
}

/// Authentication-only request body for empty POST/PUT/DELETE
///
/// Per SPEC007 API-AUTH-025: Even "empty" requests must include auth
///
/// # Examples
///
/// ```
/// // POST /playback/play with just auth fields
/// use wkmp_common::api::types::AuthRequest;
///
/// let request = AuthRequest {
///     timestamp: 1730000000000,
///     hash: "abc123...".to_string(),
/// };
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthRequest {
    /// Unix epoch time in milliseconds (API-AUTH-025)
    pub timestamp: i64,

    /// SHA-256 hash (64 hex chars) (API-AUTH-025)
    pub hash: String,
}

// ========================================
// Error Response Types
// ========================================

/// Error response per API-AUTH-029
///
/// Returned as 401 Unauthorized when authentication fails
///
/// # Examples
///
/// ```
/// use wkmp_common::api::types::AuthErrorResponse;
///
/// let error = AuthErrorResponse {
///     error: "timestamp_invalid".to_string(),
///     message: "Request timestamp outside acceptable window".to_string(),
///     details: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct AuthErrorResponse {
    /// Error type identifier
    pub error: String,
    /// Human-readable error message
    pub message: String,
    /// Additional error details (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl AuthErrorResponse {
    /// Create new auth error response
    pub fn new(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Create auth error with details
    pub fn with_details(
        error: impl Into<String>,
        message: impl Into<String>,
        details: Value,
    ) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            details: Some(details),
        }
    }
}

// ========================================
// Tests
// ========================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_query_serialization() {
        let query = AuthQuery {
            timestamp: 1730000000000,
            hash: "abc123".to_string(),
        };

        let json = serde_json::to_string(&query).unwrap();
        assert!(json.contains("timestamp"));
        assert!(json.contains("1730000000000"));
        assert!(json.contains("abc123"));
    }

    #[test]
    fn test_auth_request_deserialization() {
        let json = r#"{"timestamp": 1730000000000, "hash": "abc123"}"#;
        let request: AuthRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.timestamp, 1730000000000);
        assert_eq!(request.hash, "abc123");
    }

    #[test]
    fn test_auth_error_response() {
        let error = AuthErrorResponse::new("timestamp_invalid", "Timestamp too old");

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("timestamp_invalid"));
        assert!(json.contains("Timestamp too old"));
    }

    #[test]
    fn test_auth_error_with_details() {
        let details = serde_json::json!({
            "timestamp": 1730000000000i64,
            "server_time": 1730000001500i64
        });

        let error =
            AuthErrorResponse::with_details("timestamp_invalid", "Timestamp too old", details);

        assert_eq!(error.error, "timestamp_invalid");
        assert!(error.details.is_some());
    }
}
