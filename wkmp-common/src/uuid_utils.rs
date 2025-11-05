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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_returns_valid_uuid() {
        let uuid = generate();
        // UUID should be valid (non-nil)
        assert_ne!(uuid, Uuid::nil());
    }

    #[test]
    fn test_generate_returns_version_4() {
        let uuid = generate();
        // UUIDv4 has version field = 4
        assert_eq!(uuid.get_version_num(), 4);
    }

    #[test]
    fn test_generate_returns_unique_uuids() {
        let uuid1 = generate();
        let uuid2 = generate();
        // Successive calls should produce different UUIDs
        assert_ne!(uuid1, uuid2);
    }

    #[test]
    fn test_parse_valid_uuid_string() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let result = parse(uuid_str);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_string(), uuid_str);
    }

    #[test]
    fn test_parse_uppercase_uuid() {
        let uuid_str = "550E8400-E29B-41D4-A716-446655440000";
        let result = parse(uuid_str);
        assert!(result.is_ok());
        // parse should handle uppercase (normalized to lowercase)
        assert_eq!(result.unwrap().to_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_parse_hyphenless_uuid() {
        let uuid_str = "550e8400e29b41d4a716446655440000";
        let result = parse(uuid_str);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_string(), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_parse_invalid_uuid_too_short() {
        let result = parse("550e8400-e29b-41d4");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_uuid_bad_characters() {
        let result = parse("xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_string() {
        let result = parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_roundtrip() {
        let original = generate();
        let parsed = parse(&original.to_string()).unwrap();
        assert_eq!(original, parsed);
    }
}
