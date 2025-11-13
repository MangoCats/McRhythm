# WKMP File Scanner Implementation

**⚙️ TIER 3 - IMPLEMENTATION SPECIFICATION**

Defines Rust implementation for recursive audio file discovery. Derived from [SPEC032](SPEC032-audio_ingest_architecture.md) and [SPEC008](SPEC008-library_management.md).

> **Related:** [Audio Ingest Architecture](SPEC032-audio_ingest_architecture.md) | [Library Management](SPEC008-library_management.md)

---

## Overview

**Module:** `wkmp-ai/src/services/file_scanner.rs`
**Purpose:** Discover audio files in directory tree with format validation
**Dependencies:** walkdir, infer (file type detection)

---

## Supported Audio Formats

| Format | Extensions | Magic Bytes | Priority |
|--------|-----------|-------------|----------|
| **MP3** | .mp3 | `FF FB`, `FF F3`, `FF F2`, `ID3` | High |
| **FLAC** | .flac | `fLaC` | High |
| **OGG Vorbis** | .ogg, .oga | `OggS` | High |
| **AAC/M4A** | .m4a, .aac, .mp4 | `ftyp` | High |
| **WAV** | .wav | `RIFF` + `WAVE` | Medium |
| **Opus** | .opus | `OggS` + Opus header | Medium |
| **WMA** | .wma | ASF header | Low |

**Detection Strategy:** Magic bytes (primary) + extension (fallback)

---

## File Scanner Implementation

```rust
// wkmp-ai/src/services/file_scanner.rs

use std::path::{Path, PathBuf};
use walkdir::{WalkDir, DirEntry};
use std::fs::File;
use std::io::Read;

/// Audio file scanner
pub struct FileScanner {
    ignore_patterns: Vec<String>,
    max_depth: Option<usize>,
}

impl FileScanner {
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
    pub fn scan(&self, root_path: &Path) -> Result<Vec<PathBuf>, ScanError> {
        if !root_path.exists() {
            return Err(ScanError::PathNotFound(root_path.to_path_buf()));
        }

        if !root_path.is_dir() {
            return Err(ScanError::NotADirectory(root_path.to_path_buf()));
        }

        let mut audio_files = Vec::new();
        let mut symlink_visited = std::collections::HashSet::new();

        let walker = WalkDir::new(root_path)
            .follow_links(false)  // Don't follow symlinks automatically
            .max_depth(self.max_depth.unwrap_or(usize::MAX))
            .into_iter()
            .filter_entry(|e| self.should_process_entry(e, &mut symlink_visited));

        for entry in walker {
            match entry {
                Ok(entry) => {
                    if entry.file_type().is_file() {
                        if self.is_audio_file(entry.path())? {
                            audio_files.push(entry.path().to_path_buf());
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Error accessing entry: {}", e);
                    // Continue scanning, don't abort
                }
            }
        }

        Ok(audio_files)
    }

    /// Check if entry should be processed
    fn should_process_entry(
        &self,
        entry: &DirEntry,
        symlink_visited: &mut std::collections::HashSet<PathBuf>,
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
                    log::warn!("Symlink loop detected: {}", path.display());
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

        let mut buffer = [0u8; 12];  // Read first 12 bytes
        let bytes_read = file.read(&mut buffer)
            .map_err(|e| ScanError::FileAccessError(path.to_path_buf(), e.to_string()))?;

        if bytes_read < 4 {
            return Ok(false);  // Too small to be audio
        }

        let is_audio = match &buffer[..bytes_read.min(12)] {
            // MP3
            [0xFF, 0xFB, ..] | [0xFF, 0xF3, ..] | [0xFF, 0xF2, ..] => true,
            [b'I', b'D', b'3', ..] => true,  // MP3 with ID3 tag

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
}
```

---

## Scan Statistics

```rust
/// Scan result with statistics
#[derive(Debug, Clone)]
pub struct ScanResult {
    pub files: Vec<PathBuf>,
    pub total_size: u64,
    pub by_format: std::collections::HashMap<String, usize>,
    pub errors: Vec<ScanError>,
}

impl FileScanner {
    /// Scan with statistics
    pub fn scan_with_stats(&self, root_path: &Path) -> Result<ScanResult, ScanError> {
        let files = self.scan(root_path)?;

        let mut total_size = 0u64;
        let mut by_format = std::collections::HashMap::new();
        let mut errors = Vec::new();

        for file in &files {
            // Accumulate size
            match self.get_file_size(file) {
                Ok(size) => total_size += size,
                Err(e) => errors.push(e),
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
}
```

---

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("Path not found: {0}")]
    PathNotFound(PathBuf),

    #[error("Not a directory: {0}")]
    NotADirectory(PathBuf),

    #[error("File access error {0}: {1}")]
    FileAccessError(PathBuf, String),

    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),

    #[error("I/O error: {0}")]
    IoError(String),
}
```

---

## Usage Example

```rust
// In import workflow
let scanner = FileScanner::new();

match scanner.scan_with_stats(&root_folder) {
    Ok(result) => {
        log::info!("Found {} audio files", result.files.len());
        log::info!("Total size: {} MB", result.total_size / 1_000_000);

        for (format, count) in &result.by_format {
            log::info!("  {}: {} files", format.to_uppercase(), count);
        }

        if !result.errors.is_empty() {
            log::warn!("{} files had errors during scan", result.errors.len());
        }

        // Process files
        for file_path in result.files {
            // Continue to metadata extraction
        }
    }
    Err(e) => {
        return Err(ImportError::ScanFailed(e));
    }
}
```

---

## Security Considerations

### Path Validation

```rust
impl FileScanner {
    /// Validate path is within root folder (prevent directory traversal)
    pub fn validate_path(&self, path: &Path, root: &Path) -> Result<(), ScanError> {
        let canonical_path = path.canonicalize()
            .map_err(|e| ScanError::FileAccessError(path.to_path_buf(), e.to_string()))?;

        let canonical_root = root.canonicalize()
            .map_err(|e| ScanError::FileAccessError(root.to_path_buf(), e.to_string()))?;

        if !canonical_path.starts_with(&canonical_root) {
            return Err(ScanError::PermissionDenied(path.to_path_buf()));
        }

        Ok(())
    }
}
```

### File Size Limits

```rust
impl FileScanner {
    /// Check file size is reasonable (<2GB)
    pub fn validate_file_size(&self, path: &Path) -> Result<(), ScanError> {
        let size = self.get_file_size(path)?;

        const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024 * 1024;  // 2GB

        if size > MAX_FILE_SIZE {
            return Err(ScanError::IoError(
                format!("File too large: {} bytes (max 2GB)", size)
            ));
        }

        Ok(())
    }
}
```

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_finds_audio_files() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        fs::write(root.join("song.mp3"), b"\xFF\xFB\x90\x00").unwrap();
        fs::write(root.join("album.flac"), b"fLaC\x00\x00\x00\x22").unwrap();
        fs::write(root.join("readme.txt"), b"not audio").unwrap();

        let scanner = FileScanner::new();
        let files = scanner.scan(root).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.file_name().unwrap() == "song.mp3"));
        assert!(files.iter().any(|f| f.file_name().unwrap() == "album.flac"));
    }

    #[test]
    fn test_ignore_patterns() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join("song.mp3"), b"\xFF\xFB\x90\x00").unwrap();
        fs::write(root.join(".DS_Store"), b"metadata").unwrap();

        let scanner = FileScanner::new();
        let files = scanner.scan(root).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name().unwrap(), "song.mp3");
    }

    #[test]
    fn test_magic_byte_verification() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // MP3 extension but not MP3 content
        let fake_mp3 = root.join("fake.mp3");
        fs::write(&fake_mp3, b"This is not MP3 data").unwrap();

        let scanner = FileScanner::new();
        let is_audio = scanner.verify_magic_bytes(&fake_mp3).unwrap();

        assert!(!is_audio);  // Rejected despite .mp3 extension
    }

    #[test]
    fn test_nested_directories() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let nested = root.join("artist").join("album");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("track.mp3"), b"\xFF\xFB\x90\x00").unwrap();

        let scanner = FileScanner::new();
        let files = scanner.scan(root).unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].to_string_lossy().contains("artist"));
        assert!(files[0].to_string_lossy().contains("album"));
    }

    #[test]
    fn test_symlink_loop_detection() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let dir_a = root.join("a");
        let dir_b = root.join("b");
        fs::create_dir(&dir_a).unwrap();
        fs::create_dir(&dir_b).unwrap();

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&dir_b, dir_a.join("link_to_b")).unwrap();
            std::os::unix::fs::symlink(&dir_a, dir_b.join("link_to_a")).unwrap();
        }

        let scanner = FileScanner::new();
        // Should not hang or panic
        let result = scanner.scan(root);
        assert!(result.is_ok());
    }
}
```

---

## Performance Considerations

### Parallel Scanning (Future Enhancement)

```rust
// Currently sequential, can be parallelized for very large libraries
use rayon::prelude::*;

impl FileScanner {
    pub fn scan_parallel(&self, root_path: &Path) -> Result<Vec<PathBuf>, ScanError> {
        // Collect all paths first
        let entries: Vec<_> = WalkDir::new(root_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .collect();

        // Parallel audio detection
        let audio_files: Vec<PathBuf> = entries.par_iter()
            .filter_map(|entry| {
                if self.is_audio_file(entry.path()).unwrap_or(false) {
                    Some(entry.path().to_path_buf())
                } else {
                    None
                }
            })
            .collect();

        Ok(audio_files)
    }
}
```

### Scan Performance Targets

| Library Size | Expected Scan Time | Bottleneck |
|--------------|-------------------|------------|
| 100 files | <1 second | Disk I/O (magic bytes) |
| 1,000 files | 1-3 seconds | Disk I/O |
| 10,000 files | 10-30 seconds | Directory traversal |

**Optimization:** Cache directory structure between scans (future enhancement)

---

## Integration with Import Workflow

```rust
// In import_workflow.rs

async fn scanning_phase(
    session_id: Uuid,
    root_folder: PathBuf,
    tx: broadcast::Sender<ImportEvent>,
) -> Result<Vec<PathBuf>, ImportError> {
    // Update state
    tx.send(ImportEvent::StateChanged {
        session_id,
        old_state: ImportState::Started,
        new_state: ImportState::Scanning,
    }).ok();

    // Scan files
    let scanner = FileScanner::new();
    let result = scanner.scan_with_stats(&root_folder)?;

    // Send progress event
    tx.send(ImportEvent::Progress {
        session_id,
        current: result.files.len(),
        total: result.files.len(),
        operation: format!("Found {} audio files", result.files.len()),
    }).ok();

    log::info!("Scan complete: {} files, {} MB",
        result.files.len(),
        result.total_size / 1_000_000
    );

    Ok(result.files)
}
```

---

**Document Version:** 1.0
**Last Updated:** 2025-10-27
**Status:** Implementation specification (ready for coding)
