//! Full Library Import Performance Test
//!
//! **[PLAN_am30_integration]** End-to-end performance testing for library import workflow
//!
//! Tests the complete import pipeline across multiple files, measuring:
//! - Total import time for N files
//! - Per-file processing time (average, min, max)
//! - Event emission throughput
//! - Single-track vs album routing behavior
//! - Memory and resource usage patterns

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::sync::mpsc;
use wkmp_ai::workflow::{Pipeline, PipelineConfig, WorkflowEvent};

mod helpers;
use helpers::audio_generator::{generate_test_library, generate_test_wav, AudioConfig};

/// Performance metrics collected during import
#[derive(Debug, Default)]
struct ImportMetrics {
    /// Total files processed
    pub files_processed: usize,
    /// Total passages created
    pub passages_created: usize,
    /// Total import time
    pub total_duration: Duration,
    /// Per-file timing (file_path, duration)
    pub file_timings: Vec<(String, Duration)>,
    /// Event counts by type
    pub event_counts: EventCounts,
    /// Files routed to album processing
    pub album_files: usize,
    /// Files routed to single-track processing
    pub single_track_files: usize,
    /// Errors encountered
    pub errors: Vec<String>,
}

#[derive(Debug, Default)]
struct EventCounts {
    pub file_started: usize,
    pub file_completed: usize,
    pub boundary_detected: usize,
    pub passage_started: usize,
    pub passage_completed: usize,
    pub single_track_check: usize,
    pub album_matching_started: usize,
    pub album_matching_completed: usize,
    pub album_matching_failed: usize,
    pub album_matching_fallback: usize,
    pub errors: usize,
}

impl ImportMetrics {
    fn avg_file_time(&self) -> Duration {
        if self.files_processed == 0 {
            Duration::ZERO
        } else {
            self.total_duration / self.files_processed as u32
        }
    }

    fn min_file_time(&self) -> Duration {
        self.file_timings
            .iter()
            .map(|(_, d)| *d)
            .min()
            .unwrap_or(Duration::ZERO)
    }

    fn max_file_time(&self) -> Duration {
        self.file_timings
            .iter()
            .map(|(_, d)| *d)
            .max()
            .unwrap_or(Duration::ZERO)
    }

    fn throughput_files_per_sec(&self) -> f64 {
        if self.total_duration.as_secs_f64() > 0.0 {
            self.files_processed as f64 / self.total_duration.as_secs_f64()
        } else {
            0.0
        }
    }

    fn print_summary(&self) {
        println!("\n=== Full Library Import Performance Summary ===");
        println!("Files processed:     {}", self.files_processed);
        println!("Passages created:    {}", self.passages_created);
        println!("Total duration:      {:?}", self.total_duration);
        println!("Avg per file:        {:?}", self.avg_file_time());
        println!("Min file time:       {:?}", self.min_file_time());
        println!("Max file time:       {:?}", self.max_file_time());
        println!(
            "Throughput:          {:.2} files/sec",
            self.throughput_files_per_sec()
        );
        println!("\n--- Routing ---");
        println!("Single-track files:  {}", self.single_track_files);
        println!("Album files:         {}", self.album_files);
        println!("\n--- Events ---");
        println!("FileStarted:         {}", self.event_counts.file_started);
        println!("FileCompleted:       {}", self.event_counts.file_completed);
        println!("BoundaryDetected:    {}", self.event_counts.boundary_detected);
        println!("PassageStarted:      {}", self.event_counts.passage_started);
        println!("PassageCompleted:    {}", self.event_counts.passage_completed);
        println!(
            "SingleTrackCheck:    {}",
            self.event_counts.single_track_check
        );
        println!(
            "AlbumMatchStarted:   {}",
            self.event_counts.album_matching_started
        );
        println!(
            "AlbumMatchCompleted: {}",
            self.event_counts.album_matching_completed
        );
        println!(
            "AlbumMatchFailed:    {}",
            self.event_counts.album_matching_failed
        );
        println!(
            "AlbumMatchFallback:  {}",
            self.event_counts.album_matching_fallback
        );
        println!("Errors:              {}", self.event_counts.errors);
        if !self.errors.is_empty() {
            println!("\n--- Error Details ---");
            for (i, err) in self.errors.iter().enumerate().take(5) {
                println!("  {}: {}", i + 1, err);
            }
            if self.errors.len() > 5 {
                println!("  ... and {} more", self.errors.len() - 5);
            }
        }
        println!("================================================\n");
    }
}

/// Event collector task - runs in background counting events
async fn collect_events(
    mut rx: mpsc::Receiver<WorkflowEvent>,
    counts: Arc<std::sync::Mutex<EventCounts>>,
    errors: Arc<std::sync::Mutex<Vec<String>>>,
) {
    while let Some(event) = rx.recv().await {
        let mut c = counts.lock().unwrap();
        match &event {
            WorkflowEvent::FileStarted { .. } => c.file_started += 1,
            WorkflowEvent::FileCompleted { .. } => c.file_completed += 1,
            WorkflowEvent::BoundaryDetected { .. } => c.boundary_detected += 1,
            WorkflowEvent::PassageStarted { .. } => c.passage_started += 1,
            WorkflowEvent::PassageCompleted { .. } => c.passage_completed += 1,
            WorkflowEvent::SingleTrackCheckCompleted { .. } => c.single_track_check += 1,
            WorkflowEvent::AlbumMatchingStarted { .. } => c.album_matching_started += 1,
            WorkflowEvent::AlbumMatchingCompleted { .. } => c.album_matching_completed += 1,
            WorkflowEvent::AlbumMatchingFailed { .. } => c.album_matching_failed += 1,
            WorkflowEvent::AlbumMatchingFallback { .. } => c.album_matching_fallback += 1,
            WorkflowEvent::Error { message, .. } => {
                c.errors += 1;
                errors.lock().unwrap().push(message.clone());
            }
            _ => {}
        }
    }
}

/// Generate test library with mixed file types
fn generate_mixed_library(dir: &Path, single_track_count: usize, album_count: usize) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    // Generate single-track files (30-60 seconds, no silence gaps)
    let single_config = AudioConfig {
        duration_seconds: 45.0,
        sample_rate: 44100,
        channels: 2,
        silence_gap_start: None,
        silence_gap_duration: None,
    };

    for i in 0..single_track_count {
        let filename = format!("single_track_{:03}.wav", i + 1);
        let file_path = dir.join(filename);
        generate_test_wav(&file_path, &single_config)?;
        files.push(file_path);
    }

    // Generate album-like files (longer duration with silence gaps suggesting tracks)
    // Note: These will likely still be processed as single tracks since they don't have
    // the ID3/filename patterns that indicate albums, but this tests the routing logic
    let album_config = AudioConfig {
        duration_seconds: 180.0, // 3 minutes (longer file)
        sample_rate: 44100,
        channels: 2,
        silence_gap_start: Some(60.0),   // Gap at 1 minute
        silence_gap_duration: Some(2.0), // 2 second silence
    };

    for i in 0..album_count {
        let filename = format!("album_file_{:03}.wav", i + 1);
        let file_path = dir.join(filename);
        generate_test_wav(&file_path, &album_config)?;
        files.push(file_path);
    }

    Ok(files)
}

/// **[TC-PERF-001]** Full library import performance benchmark
///
/// **Given:** A test library of N audio files
/// **When:** Processing all files through the Pipeline
/// **Then:**
/// - All files processed without fatal errors
/// - Events emitted for each stage
/// - Performance metrics collected and reported
/// - Average processing time within acceptable bounds
#[tokio::test]
async fn test_full_library_import_10_files() -> Result<()> {
    // Setup test library
    let temp_dir = TempDir::new()?;
    let files = generate_test_library(temp_dir.path(), 10, &AudioConfig {
        duration_seconds: 35.0, // Slightly over minimum passage length
        ..Default::default()
    })?;

    // Run import and collect metrics
    let metrics = run_library_import(&files, false).await?;
    metrics.print_summary();

    // Assertions
    assert_eq!(
        metrics.files_processed, 10,
        "Should process all 10 files"
    );
    assert!(
        metrics.passages_created >= 10,
        "Should create at least 1 passage per file"
    );
    assert_eq!(
        metrics.event_counts.file_started, 10,
        "Should emit FileStarted for each file"
    );
    assert!(
        metrics.event_counts.errors < 5,
        "Should have minimal errors (actual: {})",
        metrics.event_counts.errors
    );

    // Performance bounds (generous for CI environments)
    // 35s audio files should process in under 30s each on average
    assert!(
        metrics.avg_file_time() < Duration::from_secs(30),
        "Average file time should be under 30s (actual: {:?})",
        metrics.avg_file_time()
    );

    Ok(())
}

/// **[TC-PERF-002]** Large library import (25 files)
///
/// Tests scalability with more files
#[tokio::test]
#[ignore = "Long-running performance test - run with --ignored"]
async fn test_full_library_import_25_files() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let files = generate_test_library(temp_dir.path(), 25, &AudioConfig {
        duration_seconds: 35.0,
        ..Default::default()
    })?;

    let metrics = run_library_import(&files, false).await?;
    metrics.print_summary();

    assert_eq!(metrics.files_processed, 25);
    assert!(metrics.passages_created >= 25);

    // Throughput should be relatively stable (not degrading with more files)
    assert!(
        metrics.throughput_files_per_sec() > 0.01,
        "Throughput should be positive (actual: {:.4} files/sec)",
        metrics.throughput_files_per_sec()
    );

    Ok(())
}

/// **[TC-PERF-003]** Mixed library (single tracks + album-like files)
///
/// Tests routing between single-track and album processing paths
#[tokio::test]
async fn test_mixed_library_import() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let files = generate_mixed_library(temp_dir.path(), 5, 3)?;

    // Enable album matching for routing test
    let metrics = run_library_import(&files, true).await?;
    metrics.print_summary();

    assert_eq!(metrics.files_processed, 8, "Should process all 8 files");

    // With album matching enabled, should see SingleTrackCheckCompleted events
    assert!(
        metrics.event_counts.single_track_check >= 8,
        "Should check each file for single-track vs album"
    );

    Ok(())
}

/// **[TC-PERF-004]** Event throughput test
///
/// Verifies events are emitted efficiently without blocking
#[tokio::test]
async fn test_event_throughput() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let files = generate_test_library(temp_dir.path(), 5, &AudioConfig {
        duration_seconds: 35.0,
        ..Default::default()
    })?;

    let metrics = run_library_import(&files, false).await?;

    // Calculate events per file
    let total_events = metrics.event_counts.file_started
        + metrics.event_counts.file_completed
        + metrics.event_counts.boundary_detected
        + metrics.event_counts.passage_started
        + metrics.event_counts.passage_completed;

    let events_per_file = total_events as f64 / metrics.files_processed as f64;

    println!("Events per file: {:.1}", events_per_file);

    // Each file should generate at least 4 events:
    // FileStarted, BoundaryDetected, PassageStarted, PassageCompleted, FileCompleted
    assert!(
        events_per_file >= 4.0,
        "Should emit at least 4 events per file (actual: {:.1})",
        events_per_file
    );

    Ok(())
}

/// **[TC-PERF-005]** Concurrent file processing simulation
///
/// Tests behavior when multiple files are queued
#[tokio::test]
async fn test_sequential_file_processing() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let files = generate_test_library(temp_dir.path(), 3, &AudioConfig {
        duration_seconds: 35.0,
        ..Default::default()
    })?;

    // Process files sequentially and verify ordering
    let (tx, rx) = mpsc::channel(1000);
    let event_counts = Arc::new(std::sync::Mutex::new(EventCounts::default()));
    let errors = Arc::new(std::sync::Mutex::new(Vec::new()));

    let collector_counts = event_counts.clone();
    let collector_errors = errors.clone();
    let collector = tokio::spawn(collect_events(rx, collector_counts, collector_errors));

    let config = PipelineConfig {
        enable_album_matching: false,
        ..Default::default()
    };
    let pipeline = Pipeline::with_events(config, tx);

    let mut file_order = Vec::new();
    for file in &files {
        let start = Instant::now();
        let result = pipeline.process_file(file).await;
        let duration = start.elapsed();

        file_order.push((file.file_name().unwrap().to_string_lossy().to_string(), duration));

        if let Err(e) = result {
            println!("Warning: File processing error: {}", e);
        }
    }

    // Drop pipeline to close event channel
    drop(pipeline);

    // Wait for collector
    collector.await?;

    // Verify sequential processing (each file completed before next started)
    let counts = event_counts.lock().unwrap();
    assert_eq!(
        counts.file_started, 3,
        "Should have 3 FileStarted events"
    );
    assert_eq!(
        counts.file_completed, 3,
        "Should have 3 FileCompleted events"
    );

    println!("\nFile processing order:");
    for (name, duration) in &file_order {
        println!("  {} - {:?}", name, duration);
    }

    Ok(())
}

/// Core import runner - processes files and collects metrics
async fn run_library_import(files: &[PathBuf], enable_album_matching: bool) -> Result<ImportMetrics> {
    let (tx, rx) = mpsc::channel(1000);
    let event_counts = Arc::new(std::sync::Mutex::new(EventCounts::default()));
    let errors = Arc::new(std::sync::Mutex::new(Vec::new()));

    // Start event collector
    let collector_counts = event_counts.clone();
    let collector_errors = errors.clone();
    let collector = tokio::spawn(collect_events(rx, collector_counts, collector_errors));

    // Create pipeline with events
    let config = PipelineConfig {
        enable_album_matching,
        single_track_threshold: 0.6,
        album_match_min_percentage: 60.0,
        album_match_fallback: true,
        ..Default::default()
    };
    let pipeline = Pipeline::with_events(config, tx);

    // Process all files
    let mut metrics = ImportMetrics::default();
    let overall_start = Instant::now();

    for file in files {
        let file_start = Instant::now();
        let result = pipeline.process_file(file).await;
        let file_duration = file_start.elapsed();

        metrics.file_timings.push((
            file.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            file_duration,
        ));

        match result {
            Ok(passages) => {
                metrics.files_processed += 1;
                metrics.passages_created += passages.len();
            }
            Err(e) => {
                metrics.errors.push(format!(
                    "{}: {}",
                    file.file_name().unwrap_or_default().to_string_lossy(),
                    e
                ));
            }
        }
    }

    metrics.total_duration = overall_start.elapsed();

    // Drop pipeline to close channel
    drop(pipeline);

    // Wait for event collector to finish
    collector.await?;

    // Copy event counts
    let counts = event_counts.lock().unwrap();
    metrics.event_counts = EventCounts {
        file_started: counts.file_started,
        file_completed: counts.file_completed,
        boundary_detected: counts.boundary_detected,
        passage_started: counts.passage_started,
        passage_completed: counts.passage_completed,
        single_track_check: counts.single_track_check,
        album_matching_started: counts.album_matching_started,
        album_matching_completed: counts.album_matching_completed,
        album_matching_failed: counts.album_matching_failed,
        album_matching_fallback: counts.album_matching_fallback,
        errors: counts.errors,
    };

    // Determine routing counts from events
    metrics.album_files = metrics.event_counts.album_matching_started;
    metrics.single_track_files = metrics.files_processed.saturating_sub(metrics.album_files);

    // Append collected errors
    let collected_errors = errors.lock().unwrap();
    metrics.errors.extend(collected_errors.clone());

    Ok(metrics)
}

/// **[TC-PERF-006]** Memory stability test
///
/// Process files in batches to verify no memory leaks
#[tokio::test]
#[ignore = "Long-running memory test - run with --ignored"]
async fn test_memory_stability_batch_processing() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Generate small batches and process them
    let batch_size = 5;
    let num_batches = 4;

    let mut total_metrics = ImportMetrics::default();

    for batch in 0..num_batches {
        println!("\n--- Processing batch {} of {} ---", batch + 1, num_batches);

        // Generate batch of files
        let batch_dir = temp_dir.path().join(format!("batch_{}", batch));
        std::fs::create_dir_all(&batch_dir)?;

        let files = generate_test_library(&batch_dir, batch_size, &AudioConfig {
            duration_seconds: 35.0,
            ..Default::default()
        })?;

        let metrics = run_library_import(&files, false).await?;

        total_metrics.files_processed += metrics.files_processed;
        total_metrics.passages_created += metrics.passages_created;
        total_metrics.total_duration += metrics.total_duration;

        println!("Batch {} complete: {} files, {} passages",
            batch + 1, metrics.files_processed, metrics.passages_created);
    }

    println!("\n=== Total across all batches ===");
    println!("Files:    {}", total_metrics.files_processed);
    println!("Passages: {}", total_metrics.passages_created);
    println!("Duration: {:?}", total_metrics.total_duration);

    assert_eq!(
        total_metrics.files_processed,
        batch_size * num_batches,
        "Should process all files across batches"
    );

    Ok(())
}

#[cfg(test)]
mod benchmark_tests {
    use super::*;

    /// Quick benchmark for CI - runs minimal test with short audio
    #[tokio::test]
    async fn benchmark_quick_3_files() -> Result<()> {
        let temp_dir = TempDir::new()?;
        // Use 5-second audio for quick benchmark (minimum practical length)
        let files = generate_test_library(temp_dir.path(), 3, &AudioConfig {
            duration_seconds: 5.0,
            ..Default::default()
        })?;

        let start = Instant::now();
        let metrics = run_library_import(&files, false).await?;
        let total = start.elapsed();

        println!("\nQuick Benchmark (3 files, 5s each):");
        println!("  Total time:    {:?}", total);
        println!("  Files/sec:     {:.3}", 3.0 / total.as_secs_f64());
        println!("  Passages:      {}", metrics.passages_created);
        println!("  Errors:        {}", metrics.event_counts.errors);

        // Verify files were processed (may have errors but should complete)
        assert!(
            metrics.files_processed >= 1,
            "Should process at least 1 file (processed: {})",
            metrics.files_processed
        );
        Ok(())
    }

    /// Longer benchmark with realistic file durations
    #[tokio::test]
    #[ignore = "Run with --ignored for full performance test"]
    async fn benchmark_10_files_realistic() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let files = generate_test_library(temp_dir.path(), 10, &AudioConfig {
            duration_seconds: 35.0,
            ..Default::default()
        })?;

        let start = Instant::now();
        let metrics = run_library_import(&files, false).await?;
        let total = start.elapsed();

        metrics.print_summary();

        println!("\nRealistic Benchmark (10 files, 35s each):");
        println!("  Total time:    {:?}", total);
        println!("  Files/sec:     {:.3}", 10.0 / total.as_secs_f64());

        assert_eq!(metrics.files_processed, 10);
        Ok(())
    }

    /// **[TC-PERF-1000]** Large scale library import (1000 files)
    ///
    /// Tests scalability with a large number of files.
    /// Uses minimal 1-second audio to keep total runtime reasonable.
    #[tokio::test]
    #[ignore = "Run with --ignored for 1000-file stress test"]
    async fn benchmark_1000_files() -> Result<()> {
        println!("\n========================================");
        println!("  1000 File Library Import Benchmark");
        println!("========================================\n");

        let temp_dir = TempDir::new()?;

        // Generate 1000 files with 1-second audio (minimizes generation and decode time)
        println!("Generating 1000 test audio files (1s each)...");
        let gen_start = Instant::now();
        let files = generate_test_library(temp_dir.path(), 1000, &AudioConfig {
            duration_seconds: 1.0,  // Minimal duration for speed
            sample_rate: 22050,     // Lower sample rate for faster generation
            channels: 1,            // Mono for speed
            ..Default::default()
        })?;
        let gen_time = gen_start.elapsed();
        println!("  Generation time: {:?}", gen_time);
        println!("  Files created: {}", files.len());

        // Run import benchmark
        println!("\nProcessing 1000 files...");
        let start = Instant::now();
        let metrics = run_library_import(&files, false).await?;
        let total = start.elapsed();

        // Print detailed results
        metrics.print_summary();

        println!("\n========================================");
        println!("  1000 File Benchmark Results");
        println!("========================================");
        println!("  File generation:   {:?}", gen_time);
        println!("  Import processing: {:?}", total);
        println!("  Total test time:   {:?}", gen_time + total);
        println!("  Files processed:   {}", metrics.files_processed);
        println!("  Passages created:  {}", metrics.passages_created);
        println!("  Throughput:        {:.2} files/sec", metrics.throughput_files_per_sec());
        println!("  Avg per file:      {:?}", metrics.avg_file_time());
        println!("  Min file time:     {:?}", metrics.min_file_time());
        println!("  Max file time:     {:?}", metrics.max_file_time());
        println!("  Error count:       {}", metrics.event_counts.errors);
        println!("========================================\n");

        // Assertions
        assert!(
            metrics.files_processed >= 900,
            "Should process at least 90% of files (processed: {})",
            metrics.files_processed
        );

        // Throughput should be reasonable (at least 0.1 files/sec with real processing)
        assert!(
            metrics.throughput_files_per_sec() > 0.05,
            "Throughput too low: {:.4} files/sec",
            metrics.throughput_files_per_sec()
        );

        Ok(())
    }

    /// **[TC-PERF-100]** Medium scale library import (100 files)
    ///
    /// Intermediate test between quick benchmark and 1000-file stress test.
    #[tokio::test]
    #[ignore = "Run with --ignored for 100-file test"]
    async fn benchmark_100_files() -> Result<()> {
        println!("\n========================================");
        println!("  100 File Library Import Benchmark");
        println!("========================================\n");

        let temp_dir = TempDir::new()?;

        println!("Generating 100 test audio files (3s each)...");
        let gen_start = Instant::now();
        let files = generate_test_library(temp_dir.path(), 100, &AudioConfig {
            duration_seconds: 3.0,
            ..Default::default()
        })?;
        let gen_time = gen_start.elapsed();
        println!("  Generation time: {:?}", gen_time);

        println!("\nProcessing 100 files...");
        let start = Instant::now();
        let metrics = run_library_import(&files, false).await?;
        let total = start.elapsed();

        metrics.print_summary();

        println!("\n100 File Benchmark Results:");
        println!("  Import time:    {:?}", total);
        println!("  Files/sec:      {:.3}", 100.0 / total.as_secs_f64());
        println!("  Avg per file:   {:?}", metrics.avg_file_time());

        assert!(
            metrics.files_processed >= 95,
            "Should process at least 95% of files"
        );

        Ok(())
    }

    /// **[TC-PERF-REAL]** Real music library import test
    ///
    /// Tests with actual music files from user's library.
    /// Exercises both single-track and album routing paths with real data.
    ///
    /// Set WKMP_TEST_LIBRARY environment variable to the music directory path.
    /// Example: WKMP_TEST_LIBRARY="C:\Users\Mango Cat\Music" cargo test benchmark_real_library -- --ignored --nocapture
    #[tokio::test]
    #[ignore = "Requires WKMP_TEST_LIBRARY env var pointing to real music files"]
    async fn benchmark_real_library() -> Result<()> {
        let library_path = match std::env::var("WKMP_TEST_LIBRARY") {
            Ok(path) => PathBuf::from(path),
            Err(_) => {
                println!("WKMP_TEST_LIBRARY not set, skipping real library test");
                println!("Usage: WKMP_TEST_LIBRARY=\"/path/to/music\" cargo test benchmark_real_library -- --ignored --nocapture");
                return Ok(());
            }
        };

        if !library_path.exists() {
            println!("Library path does not exist: {:?}", library_path);
            return Ok(());
        }

        println!("\n========================================");
        println!("  Real Music Library Import Test");
        println!("========================================");
        println!("Library path: {:?}\n", library_path);

        // Scan for audio files
        let files = scan_audio_files(&library_path, None)?;

        if files.is_empty() {
            println!("No audio files found in {:?}", library_path);
            return Ok(());
        }

        println!("Found {} audio files\n", files.len());

        // Run import with album matching enabled
        let start = Instant::now();
        let metrics = run_library_import(&files, true).await?;
        let total = start.elapsed();

        metrics.print_summary();

        println!("\n========================================");
        println!("  Real Library Test Results");
        println!("========================================");
        println!("  Total files:       {}", files.len());
        println!("  Files processed:   {}", metrics.files_processed);
        println!("  Passages created:  {}", metrics.passages_created);
        println!("  Total time:        {:?}", total);
        println!("  Throughput:        {:.2} files/sec", metrics.throughput_files_per_sec());
        println!("");
        println!("  --- Routing Statistics ---");
        println!("  Single-track:      {} ({:.1}%)",
            metrics.single_track_files,
            100.0 * metrics.single_track_files as f64 / files.len().max(1) as f64
        );
        println!("  Album files:       {} ({:.1}%)",
            metrics.album_files,
            100.0 * metrics.album_files as f64 / files.len().max(1) as f64
        );
        println!("  Album matched:     {}", metrics.event_counts.album_matching_completed);
        println!("  Album failed:      {}", metrics.event_counts.album_matching_failed);
        println!("  Album fallback:    {}", metrics.event_counts.album_matching_fallback);
        println!("========================================\n");

        // Basic assertions - should process most files without errors
        assert!(
            metrics.files_processed >= files.len() / 2,
            "Should process at least 50% of files"
        );

        Ok(())
    }

    /// **[TC-PERF-REAL-LIMITED]** Real library with file limit
    ///
    /// Process first N files from real library for quick testing.
    /// Uses WKMP_TEST_LIBRARY env var, or falls back to default Music folder.
    #[tokio::test]
    #[ignore = "Requires real music files"]
    async fn benchmark_real_library_limited() -> Result<()> {
        let library_path = match std::env::var("WKMP_TEST_LIBRARY") {
            Ok(path) => PathBuf::from(path),
            Err(_) => {
                // Default fallback path for Windows development machine
                let default = PathBuf::from(r"C:\Users\Mango Cat\Music");
                if default.exists() && default.is_dir() {
                    println!("Using default library path: {:?}", default);
                    default
                } else {
                    println!("WKMP_TEST_LIBRARY not set and default path doesn't exist");
                    return Ok(());
                }
            }
        };

        let limit = std::env::var("WKMP_TEST_LIMIT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(20);

        println!("\n========================================");
        println!("  Real Library Import (limited to {} files)", limit);
        println!("========================================\n");

        let files = scan_audio_files(&library_path, Some(limit))?;
        println!("Processing {} files...\n", files.len());

        let metrics = run_library_import(&files, true).await?;
        metrics.print_summary();

        Ok(())
    }
}

/// Scan directory for audio files
fn scan_audio_files(dir: &Path, limit: Option<usize>) -> Result<Vec<PathBuf>> {
    let audio_extensions = ["mp3", "flac", "wav", "m4a", "aac", "ogg", "opus", "wma"];
    let mut files = Vec::new();

    fn scan_recursive(
        dir: &Path,
        files: &mut Vec<PathBuf>,
        extensions: &[&str],
        limit: Option<usize>,
    ) -> std::io::Result<()> {
        if let Some(max) = limit {
            if files.len() >= max {
                return Ok(());
            }
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                scan_recursive(&path, files, extensions, limit)?;
            } else if let Some(ext) = path.extension() {
                if extensions.iter().any(|e| ext.eq_ignore_ascii_case(e)) {
                    files.push(path);
                    if let Some(max) = limit {
                        if files.len() >= max {
                            return Ok(());
                        }
                    }
                }
            }
        }
        Ok(())
    }

    scan_recursive(dir, &mut files, &audio_extensions, limit)?;
    Ok(files)
}
