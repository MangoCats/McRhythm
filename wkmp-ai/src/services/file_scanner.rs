//! Audio file scanner
//!
//! **[AIA-COMP-010]** Recursive audio file discovery with format validation
//! **[AIA-PERF-030]** Two-phase parallel scanning (sequential traversal + parallel verification)
//! **[AIA-CLASSIFY-010]** File classification during scanning (PLAN027)
//!
//! Per [IMPL013](../../docs/IMPL013-file_scanner.md)

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use rayon::prelude::*;
use thiserror::Error;
use walkdir::{DirEntry, WalkDir};

// **[AIA-CLASSIFY-020]** Audio format extensions (per REQ-PI-020)
const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "m4a", "aac", "opus", "wav"
];

// **[AIA-CLASSIFY-020]** Image format extensions (per REQ-ART-020)
const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif"
];

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

    /// Scan directory for audio files with progress callback
    ///
    /// **[AIA-PERF-030]** Two-phase parallel implementation:
    /// - Phase 1: Sequential directory traversal with symlink detection
    /// - Phase 2: Parallel magic byte verification (3-6x speedup on SSD)
    ///
    /// **Progress Callback:** Called periodically during Phase 1 with current count.
    /// Called at least once per 100 files discovered.
    pub fn scan_with_progress<F>(
        &self,
        root_path: &Path,
        progress_callback: &mut F,
    ) -> Result<Vec<PathBuf>, ScanError>
    where
        F: FnMut(usize),
    {
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
        const PROGRESS_INTERVAL: usize = 100;

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

                        // Call progress callback every 100 files
                        if candidate_files.len() % PROGRESS_INTERVAL == 0 {
                            progress_callback(candidate_files.len());
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Error accessing entry: {}", e);
                    // Continue scanning, don't abort
                }
            }
        }

        // Final progress update with total count
        progress_callback(candidate_files.len());

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

    /// Scan directory for audio files (legacy - no progress callback)
    ///
    /// **[AIA-PERF-030]** Two-phase parallel implementation:
    /// - Phase 1: Sequential directory traversal with symlink detection
    /// - Phase 2: Parallel magic byte verification (3-6x speedup on SSD)
    pub fn scan(&self, root_path: &Path) -> Result<Vec<PathBuf>, ScanError> {
        self.scan_with_progress(root_path, &mut |_| {})
    }

    /// Scan with statistics and progress callback
    ///
    /// **Progress Callback:** Called periodically during Phase 1 (file discovery)
    /// with current file count. Called at least once per 100 files discovered.
    pub fn scan_with_stats_and_progress<F>(
        &self,
        root_path: &Path,
        mut progress_callback: F,
    ) -> Result<ScanResult, ScanError>
    where
        F: FnMut(usize),
    {
        let files = self.scan_with_progress(root_path, &mut progress_callback)?;

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

    /// Scan with statistics (legacy - no progress callback)
    pub fn scan_with_stats(&self, root_path: &Path) -> Result<ScanResult, ScanError> {
        self.scan_with_stats_and_progress(root_path, |_| {})
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

    // ========================================
    // File Classification Methods (PLAN027)
    // **[AIA-CLASSIFY-010]** Classify all files during scan
    // ========================================

    /// **[AIA-CLASSIFY-010]** Scan and classify ALL files with progress callback
    ///
    /// Returns FileClassification with all files categorized into audio/image/other.
    /// Classification happens during directory traversal (no additional I/O overhead).
    ///
    /// **Progress Callback:** Called periodically with current file count (every 100 files)
    pub fn scan_and_classify_with_progress<F>(
        &self,
        root_path: &Path,
        progress_callback: &mut F,
    ) -> Result<crate::models::FileClassification, ScanError>
    where
        F: FnMut(usize),
    {
        use crate::models::{FileClassification, FileInfo};

        if !root_path.exists() {
            return Err(ScanError::PathNotFound(root_path.to_path_buf()));
        }

        if !root_path.is_dir() {
            return Err(ScanError::NotADirectory(root_path.to_path_buf()));
        }

        let mut classification = FileClassification::new();
        let mut file_count = 0usize;
        let mut symlink_visited = HashSet::new();
        const PROGRESS_INTERVAL: usize = 100;

        let walker = WalkDir::new(root_path)
            .follow_links(false)
            .max_depth(self.max_depth.unwrap_or(usize::MAX))
            .into_iter()
            .filter_entry(|e| self.should_process_entry(e, &mut symlink_visited));

        for entry in walker {
            match entry {
                Ok(entry) => {
                    if entry.file_type().is_file() {
                        let path = entry.path().to_path_buf();

                        // Get file metadata
                        match std::fs::metadata(&path) {
                            Ok(metadata) => {
                                let size_bytes = metadata.len();
                                let modified_at = metadata.modified().unwrap_or(SystemTime::now());

                                // Classify by extension
                                let file_info = FileInfo::new(path.clone(), size_bytes, modified_at);

                                if let Some(ext) = path.extension() {
                                    let ext_lower = ext.to_string_lossy().to_lowercase();

                                    if self.is_audio_extension_classify(&ext_lower) {
                                        classification.audio_files.push(file_info);
                                    } else if self.is_image_extension(&ext_lower) {
                                        classification.image_files.push(file_info);
                                    } else {
                                        classification.other_files.push(file_info);
                                    }
                                } else {
                                    // No extension → other
                                    classification.other_files.push(file_info);
                                }

                                file_count += 1;

                                // Call progress callback every 100 files
                                if file_count % PROGRESS_INTERVAL == 0 {
                                    progress_callback(file_count);
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Error getting metadata for {}: {}", path.display(), e);
                                // Continue scanning, don't abort
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Error accessing entry: {}", e);
                    // Continue scanning, don't abort
                }
            }
        }

        // Final progress update
        progress_callback(file_count);

        // Sort all categories alphabetically
        classification.sort_all();

        // Mark scan as completed
        classification.mark_completed();

        tracing::info!(
            "File classification complete: {} audio, {} image, {} other (total {})",
            classification.audio_files.len(),
            classification.image_files.len(),
            classification.other_files.len(),
            classification.total_count()
        );

        Ok(classification)
    }

    /// **[AIA-CLASSIFY-010]** Scan and classify ALL files (no progress callback)
    pub fn scan_and_classify(&self, root_path: &Path) -> Result<crate::models::FileClassification, ScanError> {
        self.scan_and_classify_with_progress(root_path, &mut |_| {})
    }

    /// **[AIA-CLASSIFY-020]** Check if extension is audio (for classification)
    fn is_audio_extension_classify(&self, ext: &str) -> bool {
        AUDIO_EXTENSIONS.contains(&ext)
    }

    /// **[AIA-CLASSIFY-020]** Check if extension is image
    fn is_image_extension(&self, ext: &str) -> bool {
        IMAGE_EXTENSIONS.contains(&ext)
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
