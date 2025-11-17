//! Unit tests for file classification logic
//!
//! **[PLAN027]** File classification during SCANNING phase

use std::path::PathBuf;
use std::time::SystemTime;
use wkmp_ai::models::{FileClassification, FileInfo};

/// **[TC-CLASSIFY-001]** Test audio file classification by extension
#[test]
fn test_audio_file_extensions() {
    let audio_extensions = ["mp3", "flac", "ogg", "m4a", "aac", "opus", "wav", "MP3", "FLAC"];

    for ext in &audio_extensions {
        let path = PathBuf::from(format!("test.{}", ext));
        assert!(is_audio_extension(&path), "Extension {} should be classified as audio", ext);
    }
}

/// **[TC-CLASSIFY-002]** Test image file classification by extension
#[test]
fn test_image_file_extensions() {
    let image_extensions = ["jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif", "JPG", "PNG"];

    for ext in &image_extensions {
        let path = PathBuf::from(format!("test.{}", ext));
        assert!(is_image_extension(&path), "Extension {} should be classified as image", ext);
    }
}

/// **[TC-CLASSIFY-003]** Test other files classification
#[test]
fn test_other_file_extensions() {
    let other_extensions = ["txt", "pdf", "doc", "zip", "exe", "dll", "md", "rs"];

    for ext in &other_extensions {
        let path = PathBuf::from(format!("test.{}", ext));
        assert!(!is_audio_extension(&path), "Extension {} should not be audio", ext);
        assert!(!is_image_extension(&path), "Extension {} should not be image", ext);
    }
}

/// **[TC-CLASSIFY-004]** Test case-insensitive extension matching
#[test]
fn test_case_insensitive_extensions() {
    let test_cases = vec![
        ("test.MP3", true, false),
        ("test.FlAc", true, false),
        ("test.JPG", false, true),
        ("test.PnG", false, true),
        ("test.TXT", false, false),
    ];

    for (filename, should_be_audio, should_be_image) in test_cases {
        let path = PathBuf::from(filename);
        assert_eq!(is_audio_extension(&path), should_be_audio, "Failed for {}", filename);
        assert_eq!(is_image_extension(&path), should_be_image, "Failed for {}", filename);
    }
}

/// **[TC-CLASSIFY-005]** Test files without extensions
#[test]
fn test_files_without_extension() {
    let paths = vec![
        PathBuf::from("no_extension"),
        PathBuf::from("README"),
        PathBuf::from("Makefile"),
    ];

    for path in paths {
        assert!(!is_audio_extension(&path), "File without extension should not be audio");
        assert!(!is_image_extension(&path), "File without extension should not be image");
    }
}

/// **[TC-CLASSIFY-006]** Test FileClassification::new() creates empty categories
#[test]
fn test_file_classification_new() {
    let classification = FileClassification::new();

    assert_eq!(classification.audio_files.len(), 0);
    assert_eq!(classification.image_files.len(), 0);
    assert_eq!(classification.other_files.len(), 0);
    assert!(classification.scan_completed_at.is_none());
}

/// **[TC-CLASSIFY-007]** Test FileClassification total size calculations
#[test]
fn test_file_classification_total_sizes() {
    let mut classification = FileClassification::new();

    // Add some audio files
    classification.audio_files.push(FileInfo {
        path: PathBuf::from("song1.mp3"),
        size_bytes: 1000,
        modified_at: SystemTime::now(),
    });
    classification.audio_files.push(FileInfo {
        path: PathBuf::from("song2.flac"),
        size_bytes: 2000,
        modified_at: SystemTime::now(),
    });

    // Add some image files
    classification.image_files.push(FileInfo {
        path: PathBuf::from("cover.jpg"),
        size_bytes: 500,
        modified_at: SystemTime::now(),
    });

    // Add some other files
    classification.other_files.push(FileInfo {
        path: PathBuf::from("notes.txt"),
        size_bytes: 100,
        modified_at: SystemTime::now(),
    });

    assert_eq!(classification.audio_total_size(), 3000);
    assert_eq!(classification.image_total_size(), 500);
    assert_eq!(classification.other_total_size(), 100);
}

/// **[TC-CLASSIFY-008]** Test FileClassification sorting
#[test]
fn test_file_classification_sorting() {
    let mut classification = FileClassification::new();

    // Add files in random order
    classification.audio_files.push(FileInfo {
        path: PathBuf::from("z_last.mp3"),
        size_bytes: 1000,
        modified_at: SystemTime::now(),
    });
    classification.audio_files.push(FileInfo {
        path: PathBuf::from("a_first.mp3"),
        size_bytes: 2000,
        modified_at: SystemTime::now(),
    });
    classification.audio_files.push(FileInfo {
        path: PathBuf::from("m_middle.mp3"),
        size_bytes: 1500,
        modified_at: SystemTime::now(),
    });

    // Sort all categories
    classification.sort_all();

    // Verify audio files are sorted by path
    assert_eq!(classification.audio_files[0].path, PathBuf::from("a_first.mp3"));
    assert_eq!(classification.audio_files[1].path, PathBuf::from("m_middle.mp3"));
    assert_eq!(classification.audio_files[2].path, PathBuf::from("z_last.mp3"));
}

// Helper functions mirroring the implementation in file_scanner.rs

const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "m4a", "aac", "opus", "wav",
];

const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif",
];

fn is_audio_extension(path: &PathBuf) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

fn is_image_extension(path: &PathBuf) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}
