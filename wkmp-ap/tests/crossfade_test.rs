//! Integration test for crossfade functionality
//!
//! Tests three-passage playback with explicit lead-in, lead-out, fade-in, and fade-out durations.
//! Implements test specification: 20-second passages from middle of tracks with 8-second durations.

use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

const BASE_URL: &str = "http://localhost:5740";

/// Test passage enqueue, playback, and crossfade with explicit timing
#[tokio::test]
#[ignore] // Run with: cargo test crossfade_test -- --ignored --nocapture
async fn test_three_passage_crossfade() {
    // Wait for server to be ready (assumes server is already running)
    sleep(Duration::from_secs(1)).await;

    let client = Client::new();

    println!("\n=== Crossfade Integration Test ===");
    println!("Testing 20-second passages with 8-second lead/fade durations\n");

    // Test configuration from docs/crossfade.md:
    // - 20 second passages from middle of tracks
    // - 8 second lead-in duration
    // - 8 second lead-out duration
    // - 8 second fade-in duration
    // - 8 second fade-out duration
    //
    // Timing calculation for 20-second passage:
    // - Start = 0 (relative to passage)
    // - End = 20 seconds
    // - Fade-In Point = Start + 8 = 8 seconds (XFD-DUR-010)
    // - Lead-In Point = Start + 8 = 8 seconds (XFD-DUR-020)
    // - Lead-Out Point = End - 8 = 12 seconds (XFD-DUR-030)
    // - Fade-Out Point = End - 8 = 12 seconds (XFD-DUR-040)
    //
    // This creates the timing structure:
    // Start(0) < Fade-In(8) = Lead-In(8) < Fade-Out(12) = Lead-Out(12) < End(20)

    // Three test tracks with known durations (from previous testing):
    // Track 1 (Train): 222.5 seconds
    // Track 2 (Superfly): 277.5 seconds
    // Track 3 (What's Up): 295.5 seconds

    let passages = vec![
        (
            "Bigger,_Better,_Faster,_More/(4_Non_Blondes)Bigger,_Better,_Faster,_More-01-Train_.mp3",
            222.5, // duration in seconds
            "Train",
        ),
        (
            "Bigger,_Better,_Faster,_More/(4_Non_Blondes)Bigger,_Better,_Faster,_More-02-Superfly_.mp3",
            277.5,
            "Superfly",
        ),
        (
            "Bigger,_Better,_Faster,_More/(4_Non_Blondes)Bigger,_Better,_Faster,_More-03-What's_Up_.mp3",
            295.5,
            "What's Up",
        ),
    ];

    // Enqueue all three passages
    println!("Enqueueing 3 passages with crossfade timing:\n");

    for (idx, (file_path, total_duration, name)) in passages.iter().enumerate() {
        // Select 20 seconds from the middle of the track
        let middle_time = total_duration / 2.0;
        let start_time_sec = middle_time - 10.0; // 10 seconds before middle
        let end_time_sec = middle_time + 10.0;   // 10 seconds after middle

        // Convert to milliseconds for API (u32 as per EnqueueRequest)
        let start_time_ms = (start_time_sec * 1000.0) as u32;
        let end_time_ms = (end_time_sec * 1000.0) as u32;

        // Timing points (absolute positions in the file, in milliseconds)
        // These are the absolute times in the file where crossfade points occur
        let fade_in_point_ms = start_time_ms + 8000;   // 8 seconds after start
        let lead_in_point_ms = start_time_ms + 8000;   // 8 seconds after start
        let fade_out_point_ms = start_time_ms + 12000; // 12 seconds after start (8 before end)
        let lead_out_point_ms = start_time_ms + 12000; // 12 seconds after start (8 before end)

        let payload = json!({
            "file_path": file_path,
            "start_time_ms": start_time_ms,
            "end_time_ms": end_time_ms,
            "fade_in_point_ms": fade_in_point_ms,
            "lead_in_point_ms": lead_in_point_ms,
            "lead_out_point_ms": lead_out_point_ms,
            "fade_out_point_ms": fade_out_point_ms,
            "fade_in_curve": "exponential",
            "fade_out_curve": "logarithmic",
        });

        println!("Passage {}: {}", idx + 1, name);
        println!("  File: {}", file_path);
        println!("  Start: {:.1}s, End: {:.1}s (20.0s duration)", start_time_sec, end_time_sec);
        println!("  Fade-In Point: {:.1}s (8s fade-in duration)", fade_in_point_ms as f64 / 1000.0);
        println!("  Lead-In Point: {:.1}s (8s lead-in duration)", lead_in_point_ms as f64 / 1000.0);
        println!("  Lead-Out Point: {:.1}s (8s lead-out duration)", lead_out_point_ms as f64 / 1000.0);
        println!("  Fade-Out Point: {:.1}s (8s fade-out duration)", fade_out_point_ms as f64 / 1000.0);
        println!("  Crossfade curves: Exponential fade-in, Logarithmic fade-out\n");

        let response = client
            .post(&format!("{}/api/v1/playback/enqueue", BASE_URL))
            .json(&payload)
            .send()
            .await
            .expect("Failed to send enqueue request");

        assert_eq!(
            response.status(),
            200,
            "Enqueue failed for passage {}: {}",
            idx + 1,
            name
        );

        let body: serde_json::Value = response.json().await.expect("Failed to parse response");
        println!("✓ Enqueued passage {}: queue_entry_id = {}", idx + 1, body["queue_entry_id"]);
    }

    println!("\n=== Starting Playback ===\n");

    // Start playback
    let response = client
        .post(&format!("{}/api/v1/playback/play", BASE_URL))
        .send()
        .await
        .expect("Failed to send play request");

    assert_eq!(response.status(), 200, "Play request failed");
    println!("✓ Playback started\n");

    // Monitor playback and verify crossfades
    println!("=== Monitoring Playback ===\n");

    // Expected timeline for crossfading:
    // Passage 1: 20 seconds total
    //   - Plays from 0-8s with fade-in
    //   - Plays at full volume from 8-12s (4 seconds)
    //   - Lead-out begins at 12s
    //   - Passage 2 should start when Passage 1 reaches 12s (8s from end)
    //
    // Crossfade calculation (from docs/crossfade.md XFD-IMPL-020):
    //   - Passage A lead-out duration = 8 seconds (20s - 12s)
    //   - Passage B lead-in duration = 8 seconds
    //   - Crossfade duration = min(8, 8) = 8 seconds
    //   - Passage B starts when A has 8 seconds remaining
    //
    // Passage 1 → 2 transition:
    //   - P1 at 12s → P2 starts at 0s
    //   - Both play for 8 seconds (P1: 12s-20s, P2: 0s-8s)
    //   - P1 fades out, P2 fades in
    //
    // Passage 2 → 3 transition:
    //   - Same 8-second crossfade
    //
    // Total expected playback time:
    //   - P1 solo: 12 seconds (0-12s)
    //   - P1+P2 crossfade: 8 seconds (P1: 12-20s, P2: 0-8s)
    //   - P2 solo: 4 seconds (P2: 8-12s)
    //   - P2+P3 crossfade: 8 seconds (P2: 12-20s, P3: 0-8s)
    //   - P3 solo: 4 seconds (P3: 8-12s)
    //   - P3 fade-out: 8 seconds (P3: 12-20s)
    //   - Total: 12 + 8 + 4 + 8 + 4 + 8 = 44 seconds

    println!("\nExpected timeline:");
    println!("  0-12s:  Passage 1 solo (fade-in 0-8s, full volume 8-12s)");
    println!("  12-20s: Passage 1→2 crossfade (8s overlap)");
    println!("  20-24s: Passage 2 solo (full volume)");
    println!("  24-32s: Passage 2→3 crossfade (8s overlap)");
    println!("  32-36s: Passage 3 solo (full volume)");
    println!("  36-44s: Passage 3 fade-out");
    println!("  Total: ~44 seconds\n");

    // Record the start time for relative timing
    use std::time::Instant;
    let test_start = Instant::now();

    // Wait a moment for playback to stabilize
    sleep(Duration::from_millis(500)).await;

    // Sample at key points using elapsed time from test start
    let sample_times = vec![
        (5.0, "Passage 1 fading in"),
        (10.0, "Passage 1 at full volume"),
        (15.0, "Passage 1→2 crossfade active"),
        (22.0, "Passage 2 solo"),
        (27.0, "Passage 2→3 crossfade active"),
        (35.0, "Passage 3 solo"),
        (40.0, "Passage 3 fading out"),
    ];

    for (target_elapsed, description) in sample_times {
        // Calculate how long to wait from now
        let elapsed = test_start.elapsed().as_secs_f64();
        let wait_duration = target_elapsed - elapsed;

        if wait_duration > 0.0 {
            sleep(Duration::from_secs_f64(wait_duration)).await;
        }

        let response = client
            .get(&format!("{}/api/v1/playback/position", BASE_URL))
            .send()
            .await
            .expect("Failed to get position");

        let position: serde_json::Value = response.json().await.expect("Failed to parse position");
        let current_pos = position["position_ms"].as_f64().unwrap();
        let passage_id = position["passage_id"].as_str().unwrap_or("unknown");

        let actual_elapsed = test_start.elapsed().as_secs_f64();
        println!(
            "[Elapsed: {:.1}s, Position: {:.1}s] {} - passage_id: {}",
            actual_elapsed,
            current_pos / 1000.0,
            description,
            passage_id
        );
    }

    // Wait for playback to complete
    println!("\nWaiting for playback to complete...");
    sleep(Duration::from_secs(10)).await;

    // Verify final state
    let response = client
        .get(&format!("{}/api/v1/playback/status", BASE_URL))
        .send()
        .await
        .expect("Failed to get status");

    let status: serde_json::Value = response.json().await.expect("Failed to parse status");
    println!("\nFinal status: {}", serde_json::to_string_pretty(&status).unwrap());

    println!("\n=== Crossfade Test Complete ===\n");
}

/// Helper test to just enqueue the three passages without starting playback
/// Useful for manual testing and verification
#[tokio::test]
#[ignore]
async fn test_enqueue_only() {
    sleep(Duration::from_secs(1)).await;

    let client = Client::new();

    println!("\n=== Enqueue Test (No Playback) ===\n");

    let passages = vec![
        ("Bigger,_Better,_Faster,_More/(4_Non_Blondes)Bigger,_Better,_Faster,_More-01-Train_.mp3", 222.5),
        ("Bigger,_Better,_Faster,_More/(4_Non_Blondes)Bigger,_Better,_Faster,_More-02-Superfly_.mp3", 277.5),
        ("Bigger,_Better,_Faster,_More/(4_Non_Blondes)Bigger,_Better,_Faster,_More-03-What's_Up_.mp3", 295.5),
    ];

    for (idx, (file_path, total_duration)) in passages.iter().enumerate() {
        let middle_time = total_duration / 2.0;
        let start_time_sec = middle_time - 10.0;
        let end_time_sec = middle_time + 10.0;

        let start_time_ms = (start_time_sec * 1000.0) as u32;
        let end_time_ms = (end_time_sec * 1000.0) as u32;

        let payload = json!({
            "file_path": file_path,
            "start_time_ms": start_time_ms,
            "end_time_ms": end_time_ms,
            "fade_in_point_ms": start_time_ms + 8000,
            "lead_in_point_ms": start_time_ms + 8000,
            "lead_out_point_ms": start_time_ms + 12000,
            "fade_out_point_ms": start_time_ms + 12000,
            "fade_in_curve": "exponential",
            "fade_out_curve": "logarithmic",
        });

        let response = client
            .post(&format!("{}/api/v1/playback/enqueue", BASE_URL))
            .json(&payload)
            .send()
            .await
            .expect("Failed to send enqueue request");

        assert_eq!(response.status(), 200);

        let body: serde_json::Value = response.json().await.expect("Failed to parse response");
        println!("✓ Enqueued passage {}: {}", idx + 1, body["queue_entry_id"]);
    }

    println!("\n✓ All passages enqueued successfully");
    println!("Run: curl -X POST http://localhost:5740/api/v1/playback/play");
    println!("to start playback\n");
}
