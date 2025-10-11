//! UUID utilities

use uuid::Uuid;

/// Generate a new UUIDv4
pub fn generate() -> Uuid {
    Uuid::new_v4()
}

/// Parse UUID from string
pub fn parse(s: &str) -> Result<Uuid, uuid::Error> {
    Uuid::parse_str(s)
}
