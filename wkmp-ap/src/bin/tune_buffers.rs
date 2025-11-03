//! Buffer Auto-Tuning Utility
//!
//! Automatically determines optimal buffer parameters for stable audio playback.
//!
//! **Usage:**
//! ```bash
//! wkmp-ap tune-buffers [--quick] [--thorough] [--apply] [--export <file>]
//! ```
//!
//! **Traceability:** TUNE-UI-010, TUNE-INT-010, TUNE-ALG-010

use clap::Parser;
use std::time::Instant;
use tracing::{error, info};
use wkmp_ap::tuning::{
    generate_recommendations, CliFormatter, CurvePoint, CurveStatus,
    SystemInfo, TestConfig, TestHarness, TestResult, TuningReport, Verdict,
};

/// Buffer auto-tuning utility
#[derive(Parser, Debug)]
#[clap(name = "tune-buffers")]
#[clap(about = "Automatically tune audio buffer parameters for optimal performance")]
struct Args {
    /// Quick tuning mode (fewer test points, ~5 minutes)
    #[clap(long)]
    quick: bool,

    /// Thorough tuning mode (more test points, ~15 minutes)
    #[clap(long)]
    thorough: bool,

    /// Automatically apply recommended values to database
    #[clap(long)]
    apply: bool,

    /// Export results to JSON file
    #[clap(long, value_name = "FILE")]
    export: Option<String>,

    /// Test duration per configuration (seconds)
    #[clap(long, default_value = "30")]
    test_duration: u64,
}

/// Tuning mode configuration
#[derive(Debug)]
struct TuningMode {
    /// Mixer intervals to test (milliseconds)
    intervals: Vec<u64>,
    /// Test duration per configuration (seconds)
    test_duration: u64,
}

impl TuningMode {
    /// Quick mode: Fewer test points, faster completion
    fn quick() -> Self {
        Self {
            intervals: vec![5, 10, 20, 50],
            test_duration: 20, // Shorter tests in quick mode
        }
    }

    /// Thorough mode: Comprehensive testing
    fn thorough() -> Self {
        Self {
            intervals: vec![1, 2, 5, 10, 20, 50, 100],
            test_duration: 30,
        }
    }

    /// Default mode: Balanced testing
    fn default_mode() -> Self {
        Self {
            intervals: vec![2, 5, 10, 20, 50],
            test_duration: 30,
        }
    }
}

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    // Determine tuning mode
    let mode = if args.thorough {
        TuningMode::thorough()
    } else if args.quick {
        TuningMode::quick()
    } else {
        TuningMode::default_mode()
    };

    // Override test duration if specified
    let mode = if args.test_duration != 30 {
        TuningMode {
            test_duration: args.test_duration,
            ..mode
        }
    } else {
        mode
    };

    info!("Starting buffer auto-tuning...");
    info!(
        "Mode: {} intervals, {}s per test",
        mode.intervals.len(),
        mode.test_duration
    );

    // Create tokio runtime for async operations
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    // Run tuning
    let start_time = Instant::now();
    match run_tuning(&mode, runtime.handle().clone()) {
        Ok(report) => {
            let elapsed = start_time.elapsed();

            // Display results
            display_results(&report);

            // Export if requested
            if let Some(export_path) = args.export {
                match report.export_json(&export_path) {
                    Ok(_) => {
                        println!("\n✓ Results exported to: {}", export_path);
                    }
                    Err(e) => {
                        error!("Failed to export results: {}", e);
                    }
                }
            }

            // Apply if requested
            if args.apply {
                if let Some(recs) = &report.recommendations {
                    println!("\nApplying recommended values...");
                    println!(
                        "  mixer_check_interval_ms = {}",
                        recs.primary.mixer_check_interval_ms
                    );
                    println!("  audio_buffer_size = {}", recs.primary.audio_buffer_size);
                    println!("\nNote: Database update not yet implemented.");
                    println!("Please manually update settings in database.");
                } else {
                    println!("\nNo recommendations available to apply.");
                }
            }

            info!("Tuning complete in {:.1} seconds", elapsed.as_secs_f64());
        }
        Err(e) => {
            error!("Tuning failed: {}", e);
            std::process::exit(1);
        }
    }
}

/// Run the complete tuning process
fn run_tuning(mode: &TuningMode, runtime: tokio::runtime::Handle) -> Result<TuningReport, String> {
    let start_time = Instant::now();

    // Detect system information
    let system_info = SystemInfo::detect();
    println!("\n{}", CliFormatter::format_system_info(&system_info));

    // Phase 1: Coarse sweep - test all intervals with adaptive buffer sizing
    println!("{}", CliFormatter::format_phase_header(1, "Testing mixer intervals (adaptive buffer sizing)"));

    // Try progressively larger buffers until we find viable configurations
    let buffer_attempts = vec![512, 1024, 2048, 4096];
    let mut phase1_results = Vec::new();
    let mut viable_intervals: Vec<u64> = Vec::new();
    let mut working_buffer = 512;

    for &test_buffer in &buffer_attempts {
        println!("\nTrying with buffer size: {} frames ({:.1}ms @ 44.1kHz)",
                 test_buffer, (test_buffer as f64 / 44100.0) * 1000.0);

        let mut attempt_results = Vec::new();

        for &interval in &mode.intervals {
            let config = TestConfig::new(interval, test_buffer)
                .with_duration(mode.test_duration);

            let harness = TestHarness::new(config, runtime.clone());

            match harness.run_test() {
                Ok(result) => {
                    println!("{}", CliFormatter::format_test_progress(&result));
                    attempt_results.push(result);
                }
                Err(e) => {
                    error!("Test failed for interval {}ms: {}", interval, e);
                    // Continue with other tests
                }
            }
        }

        // Check if we found any viable intervals
        viable_intervals = attempt_results
            .iter()
            .filter(|r| matches!(r.verdict, Verdict::Stable | Verdict::Warning))
            .map(|r| r.mixer_check_interval_ms)
            .collect();

        phase1_results = attempt_results;

        if !viable_intervals.is_empty() {
            working_buffer = test_buffer;
            println!("\n✓ Found {} viable intervals with buffer size {}",
                     viable_intervals.len(), test_buffer);
            break;
        } else {
            println!("\n✗ No viable intervals found with buffer size {}", test_buffer);

            // Show diagnostic info about why tests failed
            if !phase1_results.is_empty() {
                let min_rate = phase1_results.iter()
                    .map(|r| r.underrun_rate())
                    .min_by(|a, b| a.partial_cmp(b).unwrap())
                    .unwrap_or(0.0);
                println!("   (Best result: {:.2}% underruns, need <0.1% for Stable or <1% for Warning)",
                         min_rate);
            }
        }
    }

    if viable_intervals.is_empty() {
        // Provide detailed diagnostic information
        println!("\n✗ TUNING FAILED: Could not find stable configuration even with 4096 frame buffer");
        println!("\nDiagnostic information:");
        println!("  Tests completed: {}", phase1_results.len());

        if !phase1_results.is_empty() {
            let best = phase1_results.iter()
                .min_by(|a, b| a.underrun_rate().partial_cmp(&b.underrun_rate()).unwrap())
                .unwrap();

            println!("  Best result: {}ms interval, {} buffer, {:.2}% underruns",
                     best.mixer_check_interval_ms,
                     best.audio_buffer_size,
                     best.underrun_rate());
        }

        println!("\nPossible causes:");
        println!("  1. Audio device not available or inaccessible");
        println!("  2. System audio configuration issue");
        println!("  3. Audio backend (cpal/ALSA) initialization failure");
        println!("  4. Permissions issue accessing audio device");

        println!("\nTroubleshooting steps:");
        println!("  1. Check audio device: aplay -l (Linux) or system audio settings");
        println!("  2. Test audio: speaker-test -c 2 -t wav");
        println!("  3. Check permissions: groups | grep audio");
        println!("  4. Run with debug logging: RUST_LOG=debug cargo run --bin tune-buffers");

        return Err("No stable intervals found even with maximum buffer (4096). Audio system may not be functional.".to_string());
    }

    println!("\nViable intervals: {:?} (using buffer size: {} frames)",
             viable_intervals, working_buffer);

    // Phase 2: Binary search for minimum stable buffer per viable interval
    println!("{}", CliFormatter::format_phase_header(2, "Finding minimum buffer sizes"));

    let mut all_results = phase1_results.clone();
    let mut curve_points = Vec::new();

    for &interval in &viable_intervals {
        print!("[...] {}ms interval: Searching... ", interval);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        // Binary search for minimum stable buffer
        // Start search from slightly below the working buffer from Phase 1
        let search_start = (working_buffer / 2).max(64);
        let min_buffer = binary_search_for_interval(
            interval,
            mode.test_duration,
            &runtime,
            &mut all_results,
            search_start,
            working_buffer * 2, // Search up to 2x the working buffer
        )?;

        if let Some(buffer) = min_buffer {
            println!("\r[✓] {}ms interval: Min buffer = {} frames", interval, buffer);

            curve_points.push(CurvePoint {
                interval_ms: interval,
                min_stable_buffer: Some(buffer),
                status: CurveStatus::Stable,
            });
        } else {
            println!("\r[✗] {}ms interval: Could not find stable buffer", interval);

            curve_points.push(CurvePoint {
                interval_ms: interval,
                min_stable_buffer: None,
                status: CurveStatus::Unstable,
            });
        }
    }

    // Add unstable intervals to curve
    for &interval in &mode.intervals {
        if !viable_intervals.contains(&interval) {
            curve_points.push(CurvePoint {
                interval_ms: interval,
                min_stable_buffer: None,
                status: CurveStatus::Unstable,
            });
        }
    }

    // Sort curve by interval
    curve_points.sort_by_key(|p| p.interval_ms);

    // Generate recommendations
    let recommendations = generate_recommendations(&curve_points);

    let duration = start_time.elapsed().as_secs();

    Ok(TuningReport::new(
        system_info,
        all_results,
        curve_points,
        recommendations,
        duration,
    ))
}

/// Binary search for minimum stable buffer size for a given interval
fn binary_search_for_interval(
    interval_ms: u64,
    test_duration: u64,
    runtime: &tokio::runtime::Handle,
    results: &mut Vec<TestResult>,
    min_buffer: u32,
    max_buffer: u32,
) -> Result<Option<u32>, String> {

    // Helper function to test a specific buffer size
    let mut test_buffer = |buffer: u32| -> Result<Verdict, String> {
        let config = TestConfig::new(interval_ms, buffer).with_duration(test_duration);
        let harness = TestHarness::new(config, runtime.clone());

        match harness.run_test() {
            Ok(result) => {
                let verdict = result.verdict;
                results.push(result);
                Ok(verdict)
            }
            Err(e) => Err(format!("Test failed: {}", e)),
        }
    };

    // Binary search implementation
    let mut low = min_buffer;
    let mut high = max_buffer;
    let mut best_stable: Option<u32> = None;

    while high - low > 128 {
        // Convergence threshold: 128 frames
        let mid = (low + high) / 2;

        match test_buffer(mid) {
            Ok(Verdict::Stable) => {
                // Success! Try smaller buffer
                best_stable = Some(mid);
                high = mid;
            }
            Ok(Verdict::Warning) | Ok(Verdict::Unstable) => {
                // Failed, need larger buffer
                low = mid + 1;
            }
            Err(e) => {
                error!("Test error at buffer {}: {}", mid, e);
                return Err(e);
            }
        }
    }

    // If we found a stable configuration, verify with one more test at that size
    if let Some(buffer) = best_stable {
        match test_buffer(buffer) {
            Ok(Verdict::Stable) => Ok(Some(buffer)),
            _ => {
                // Verification failed, use slightly larger buffer
                let safe_buffer = (buffer + 128).min(max_buffer);
                Ok(Some(safe_buffer))
            }
        }
    } else {
        // Could not find stable buffer
        Ok(None)
    }
}

/// Display tuning results to CLI
fn display_results(report: &TuningReport) {
    // Session summary
    println!("{}", CliFormatter::format_session_summary(report));

    // Curve summary
    println!("{}", CliFormatter::format_curve_summary(&report.curve_data));

    // Recommendations
    if let Some(recs) = &report.recommendations {
        println!("{}", CliFormatter::format_recommendations(recs));
    } else {
        println!("\n⚠ Warning: No stable configurations found.");
        println!("The system may be overloaded or hardware may be insufficient.");
        println!("Consider:");
        println!("  - Closing background applications");
        println!("  - Using a faster CPU");
        println!("  - Increasing audio buffer sizes manually");
    }
}
