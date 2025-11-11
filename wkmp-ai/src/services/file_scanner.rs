//! Audio file scanner
//!
//! **[AIA-COMP-010]** Recursive audio file discovery with format validation
//! **[AIA-PERF-030]** Two-phase parallel scanning (sequential traversal + parallel verification)
//!
//! Per [IMPL013](../../docs/IMPL013-file_scanner.md)

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use rayon::prelude::*;
use thiserror::Error;
use walkdir::{DirEntry, WalkDir};

/// Audio file scanner errors
#[derive(Debug, Error)]
pub enum ScanError {
    /// Specified path does not exist
    #[error("Path not found: {0}")]
    PathNotFound(PathBuf),

    /// Path exists but is not a directory
    #[error("Not a directory: {0}")]
    NotADirectory(PathBuf),

    /// Cannot access file
    #[error("File access error {0}: {1}")]
    FileAccessError(PathBuf, String),

    /// Permission denied when accessing path
    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),

    /// General I/O error
    #[error("I/O error: {0}")]
    IoError(String),
}

/// Scan result with statistics
#[derive(Debug, Clone)]
pub struct ScanResult {
    /// List of audio file paths found
    pub files: Vec<PathBuf>,
    /// Total size of all files in bytes
    pub total_size: u64,
    /// Count of files by audio format (extension)
    pub by_format: HashMap<String, usize>,
    /// Scan errors encountered
    pub errors: Vec<String>,
}

/// Audio file scanner
pub struct FileScanner {
    ignore_patterns: Vec<String>,
    max_depth: Option<usize>,
}

impl FileScanner {
    /// Create new file scanner with default ignore patterns
    ///
    /// Ignores system files like .DS_Store, Thumbs.db, .git, etc.
    pub fn new() -> Self {
        Self {
            ignore_patterns: vec![
                ".DS_Store".to_string(),
                "Thumbs.db".to_string(),
                ".git".to_string(),
                ".svn".to_string(),
                "node_modules".to_string(),
            ],
            max_depth: None,
        }
    }

    /// Scan directory for audio files
    ///
    /// **[AIA-PERF-030]** Two-phase parallel implementation:
    /// - Phase 1: Sequential directory traversal with symlink detection
    /// - Phase 2: Parallel magic byte verification (3-6x speedup on SSD)
    pub fn scan(&self, root_path: &Path) -> Result<Vec<PathBuf>, ScanError> {
        if !root_path.exists() {
            return Err(ScanError::PathNotFound(root_path.to_path_buf()));
        }

        if !root_path.is_dir() {
            return Err(ScanError::NotADirectory(root_path.to_path_buf()));
        }

        // Phase 1: Sequential directory traversal + symlink detection
        // This must be sequential because symlink_visited is mutable
        let mut candidate_files = Vec::new();
        let mut symlink_visited = HashSet::new();

        let walker = WalkDir::new(root_path)
            .follow_links(false) // Don't follow symlinks automatically
            .max_depth(self.max_depth.unwrap_or(usize::MAX))
            .into_iter()
            .filter_entry(|e| self.should_process_entry(e, &mut symlink_visited));

        for entry in walker {
            match entry {
                Ok(entry) => {
                    if entry.file_type().is_file() {
                        candidate_files.push(entry.path().to_path_buf());
                    }
                }
                Err(e) => {
                    tracing::warn!("Error accessing entry: {}", e);
                    // Continue scanning, don't abort
                }
            }
        }

        tracing::debug!(
            "Phase 1 complete: {} candidate files discovered",
            candidate_files.len()
        );

        // Phase 2: Parallel magic byte verification
        // Each thread reads different file independently (thread-safe I/O)
        let audio_files: Vec<PathBuf> = candidate_files
            .par_iter()
            .filter_map(|path| {
                match self.is_audio_file(path) {
                    Ok(true) => Some(path.clone()),
                    Ok(false) => None,
                    Err(e) => {
                        tracing::warn!("Error verifying {}: {}", path.display(), e);
                        None
                    }
                }
            })
            .collect();

        tracing::debug!(
            "Phase 2 complete: {} audio files verified from {} candidates",
            audio_files.len(),
            candidate_files.len()
        );

        Ok(audio_files)
    }

    /// Scan with statistics
    pub fn scan_with_stats(&self, root_path: &Path) -> Result<ScanResult, ScanError> {
        let files = self.scan(root_path)?;

        let mut total_size = 0u64;
        let mut by_format = HashMap::new();
        let mut errors = Vec::new();

        for file in &files {
            // Accumulate size
            match self.get_file_size(file) {
                Ok(size) => total_size += size,
                Err(e) => errors.push(e.to_string()),
            }

            // Count by extension
            if let Some(ext) = file.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                *by_format.entry(ext_str).or_insert(0) += 1;
            }
        }

        Ok(ScanResult {
            files,
            total_size,
            by_format,
            errors,
        })
    }

    /// Check if entry should be processed
    fn should_process_entry(
        &self,
        entry: &DirEntry,
        symlink_visited: &mut HashSet<PathBuf>,
    ) -> bool {
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy();

        // Skip ignored patterns
        for pattern in &self.ignore_patterns {
            if file_name.contains(pattern) {
                return false;
            }
        }

        // Detect symlink loops
        if entry.file_type().is_symlink() {
            if let Ok(canonical) = path.canonicalize() {
                if !symlink_visited.insert(canonical) {
                    tracing::warn!("Symlink loop detected: {}", path.display());
                    return false;
                }
            }
        }

        true
    }

    /// Check if file is audio format
    fn is_audio_file(&self, path: &Path) -> Result<bool, ScanError> {
        // 1. Check extension first (fast)
        if let Some(ext) = path.extension() {
            let ext_lower = ext.to_string_lossy().to_lowercase();
            if self.is_audio_extension(&ext_lower) {
                // 2. Verify with magic bytes (reliable)
                return self.verify_magic_bytes(path);
            }
        }

        Ok(false)
    }

    /// Check if extension is audio
    fn is_audio_extension(&self, ext: &str) -> bool {
        matches!(
            ext,
            "mp3" | "flac" | "ogg" | "oga" | "m4a" | "aac" | "mp4" | "wav" | "opus" | "wma"
        )
    }

    /// Verify file type using magic bytes
    fn verify_magic_bytes(&self, path: &Path) -> Result<bool, ScanError> {
        let mut file = File::open(path)
            .map_err(|e| ScanError::FileAccessError(path.to_path_buf(), e.to_string()))?;

        let mut buffer = [0u8; 12]; // Read first 12 bytes
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|e| ScanError::FileAccessError(path.to_path_buf(), e.to_string()))?;

        if bytes_read < 4 {
            return Ok(false); // Too small to be audio
        }

        let is_audio = match &buffer[..bytes_read.min(12)] {
            // MP3
            [0xFF, 0xFB, ..] | [0xFF, 0xF3, ..] | [0xFF, 0xF2, ..] => true,
            [b'I', b'D', b'3', ..] => true, // MP3 with ID3 tag

            // FLAC
            [b'f', b'L', b'a', b'C', ..] => true,

            // OGG (Vorbis/Opus)
            [b'O', b'g', b'g', b'S', ..] => true,

            // M4A/AAC (MP4 container)
            [_, _, _, _, b'f', b't', b'y', b'p', ..] => true,

            // WAV
            [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'A', b'V', b'E'] => true,

            _ => false,
        };

        Ok(is_audio)
    }

    /// Get file size
    pub fn get_file_size(&self, path: &Path) -> Result<u64, ScanError> {
        let metadata = std::fs::metadata(path)
            .map_err(|e| ScanError::FileAccessError(path.to_path_buf(), e.to_string()))?;
        Ok(metadata.len())
    }

    /// Validate path is within root folder (prevent directory traversal)
    pub fn validate_path(&self, path: &Path, root: &Path) -> Result<(), ScanError> {
        let canonical_path = path
            .canonicalize()
            .map_err(|e| ScanError::FileAccessError(path.to_path_buf(), e.to_string()))?;

        let canonical_root = root
            .canonicalize()
            .map_err(|e| ScanError::FileAccessError(root.to_path_buf(), e.to_string()))?;

        if !canonical_path.starts_with(&canonical_root) {
            return Err(ScanError::PermissionDenied(path.to_path_buf()));
        }

        Ok(())
    }

    /// Check file size is reasonable (<2GB)
    pub fn validate_file_size(&self, path: &Path) -> Result<(), ScanError> {
        let size = self.get_file_size(path)?;

        const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024 * 1024; // 2GB

        if size > MAX_FILE_SIZE {
            return Err(ScanError::IoError(format!(
                "File too large: {} bytes (max 2GB)",
                size
            )));
        }

        Ok(())
    }
}

impl Default for FileScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_audio_extension_detection() {
        let scanner = FileScanner::new();
        assert!(scanner.is_audio_extension("mp3"));
        assert!(scanner.is_audio_extension("flac"));
        assert!(scanner.is_audio_extension("ogg"));
        assert!(!scanner.is_audio_extension("txt"));
        assert!(!scanner.is_audio_extension("jpg"));
    }

    #[test]
    fn test_scan_nonexistent_path() {
        let scanner = FileScanner::new();
        let result = scanner.scan(Path::new("/nonexistent/path"));
        assert!(result.is_err());
        match result.unwrap_err() {
            ScanError::PathNotFound(_) => {}
            _ => panic!("Expected PathNotFound error"),
        }
    }

    #[test]
    fn test_scan_file_as_directory() {
        let scanner = FileScanner::new();
        // Use a known file path
        let result = scanner.scan(Path::new("/etc/hosts"));
        if result.is_err() {
            match result.unwrap_err() {
                ScanError::NotADirectory(_) | ScanError::PathNotFound(_) => {}
                _ => panic!("Expected NotADirectory or PathNotFound error"),
            }
        }
    }

    #[test]
    fn test_scan_empty_directory() {
        let temp_dir = std::env::temp_dir().join("wkmp_test_empty");
        fs::create_dir_all(&temp_dir).unwrap();

        let scanner = FileScanner::new();
        let result = scanner.scan(&temp_dir).unwrap();
        assert_eq!(result.len(), 0);

        fs::remove_dir(&temp_dir).unwrap();
    }
}
