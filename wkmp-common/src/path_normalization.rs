//! Path normalization utilities for cross-platform database storage
//!
//! **Design Rule:** All file paths stored in the database MUST use forward slashes (/)
//! as separators, regardless of the host platform. This ensures:
//! - Path-based lookups work correctly across platforms
//! - Database UNIQUE constraint on `files.path` prevents duplicate processing
//! - Same file with different separator representations is treated as one file
//!
//! **Rationale:**
//! On Windows, the same file can be represented with either forward slashes or backslashes:
//! - `Artist/Album/Track.mp3`
//! - `Artist\Album\Track.mp3`
//!
//! Without normalization, these are treated as different files, causing:
//! - Duplicate database records (different `path` values)
//! - Duplicate processing (both enter the import pipeline)
//! - Wasted CPU/IO (same file processed twice)
//!
//! **Solution:** Convert all paths to forward-slash format before database operations.
//! Forward slashes work correctly on all platforms (Windows, Linux, macOS).

use std::path::Path;

/// Normalize file path to canonical database format
///
/// Converts all path separators to forward slashes and removes trailing slashes.
/// This ensures consistent path representation in the database regardless of platform.
///
/// # Arguments
///
/// * `path` - File path to normalize (can be absolute or relative)
///
/// # Returns
///
/// String with all backslashes replaced by forward slashes, no trailing slash
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use wkmp_common::path_normalization::normalize_path_for_db;
///
/// // Windows-style path
/// let windows_path = Path::new("Artist\\Album\\Track.mp3");
/// assert_eq!(normalize_path_for_db(windows_path), "Artist/Album/Track.mp3");
///
/// // Unix-style path (no change needed)
/// let unix_path = Path::new("Artist/Album/Track.mp3");
/// assert_eq!(normalize_path_for_db(unix_path), "Artist/Album/Track.mp3");
///
/// // Mixed separators (Windows permits this)
/// let mixed_path = Path::new("Artist/Album\\Track.mp3");
/// assert_eq!(normalize_path_for_db(mixed_path), "Artist/Album/Track.mp3");
///
/// // Trailing slash removed
/// let trailing_path = Path::new("Artist/Album/");
/// assert_eq!(normalize_path_for_db(trailing_path), "Artist/Album");
/// ```
///
/// # Platform Behavior
///
/// - **Windows:** Backslashes converted to forward slashes
/// - **Linux/macOS:** No change (already uses forward slashes)
/// - **All platforms:** Trailing slashes removed
pub fn normalize_path_for_db(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_backslashes() {
        let path = Path::new("Artist\\Album\\Track.mp3");
        assert_eq!(normalize_path_for_db(path), "Artist/Album/Track.mp3");
    }

    #[test]
    fn test_unix_forward_slashes() {
        let path = Path::new("Artist/Album/Track.mp3");
        assert_eq!(normalize_path_for_db(path), "Artist/Album/Track.mp3");
    }

    #[test]
    fn test_mixed_separators() {
        let path = Path::new("Artist/Album\\Track.mp3");
        assert_eq!(normalize_path_for_db(path), "Artist/Album/Track.mp3");
    }

    #[test]
    fn test_trailing_slash_removed() {
        let path = Path::new("Artist/Album/");
        assert_eq!(normalize_path_for_db(path), "Artist/Album");
    }

    #[test]
    fn test_single_file() {
        let path = Path::new("Track.mp3");
        assert_eq!(normalize_path_for_db(path), "Track.mp3");
    }

    #[test]
    fn test_deep_path() {
        let path = Path::new("Music\\Library\\Artist\\Album\\CD1\\Track01.mp3");
        assert_eq!(
            normalize_path_for_db(path),
            "Music/Library/Artist/Album/CD1/Track01.mp3"
        );
    }

    #[test]
    fn test_empty_path() {
        let path = Path::new("");
        assert_eq!(normalize_path_for_db(path), "");
    }

    #[test]
    fn test_root_path() {
        let path = Path::new("/");
        assert_eq!(normalize_path_for_db(path), "");
    }
}
