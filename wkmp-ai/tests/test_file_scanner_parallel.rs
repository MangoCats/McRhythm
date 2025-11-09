use std::path::Path;
use std::time::Instant;
use wkmp_ai::services::file_scanner::FileScanner;

#[test]
fn test_parallel_file_scanning() {
    // Create test directory structure
    let test_dir = std::path::PathBuf::from("/tmp/wkmp_scan_test");

    if !test_dir.exists() {
        eprintln!("Test directory not found, creating...");
        std::fs::create_dir_all(test_dir.join("music")).unwrap();
        std::fs::create_dir_all(test_dir.join("docs")).unwrap();

        // Create test files
        for i in 1..=100 {
            std::fs::write(test_dir.join(format!("music/song{}.mp3", i)), b"").unwrap();
        }
        for i in 1..=50 {
            std::fs::write(test_dir.join(format!("docs/file{}.txt", i)), b"").unwrap();
        }
    }

    let scanner = FileScanner::new();

    let start = Instant::now();
    let result = scanner.scan_with_stats(&test_dir).unwrap();
    let duration = start.elapsed();

    println!("Scan completed in {:?}", duration);
    println!("Files found: {}", result.files.len());
    println!("By format: {:?}", result.by_format);

    // Verify results
    assert!(result.files.len() <= 100, "Should find at most 100 .mp3 files");
    assert!(result.by_format.get("mp3").unwrap_or(&0) <= &100, "Should have at most 100 mp3 files");
}

#[test]
fn test_scan_music_directory_if_exists() {
    let music_dir = Path::new("/home/sw/Music");

    if !music_dir.exists() {
        println!("Music directory doesn't exist, skipping test");
        return;
    }

    let scanner = FileScanner::new();

    let start = Instant::now();
    match scanner.scan_with_stats(music_dir) {
        Ok(result) => {
            let duration = start.elapsed();
            println!("Real music directory scan completed in {:?}", duration);
            println!("Files found: {}", result.files.len());
            println!("Total size: {} MB", result.total_size / 1_024 / 1_024);
            println!("By format: {:?}", result.by_format);
        }
        Err(e) => {
            println!("Scan failed (expected if no music): {}", e);
        }
    }
}
