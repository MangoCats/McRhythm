//! Integration Tests for Edition Selection (PLAN027)
//!
//! Tests multi-factor scoring and edition selection with realistic
//! candidate scenarios.

use wkmp_ai::matching::editions::{
    calculate_edition_score, calculate_total_duration_score, calculate_track_count_penalty,
    calculate_track_quality_score, select_best_edition, EditionCandidate,
};

/// TC-I-092-01: 5-Edition Ranking Scenario
///
/// **Test Type:** Integration Test
/// **Scope:** Multi-factor scoring with realistic edition candidates
///
/// **Given:** 5 editions with varying characteristics:
/// - Standard Edition: 11 tracks, exact match
/// - Deluxe Edition: 17 tracks, bonus tracks
/// - Box Set: 147 tracks, massive compilation
/// - Japanese Edition: 12 tracks, 1 bonus track
/// - Remaster: 11 tracks, slight duration differences
///
/// **When:** Multi-factor scoring applied to all candidates
///
/// **Then:** Standard edition wins with correct score ordering
///
/// **Verify:**
/// - Standard edition has highest score
/// - Box set has lowest score
/// - Ordering is deterministic and sensible
#[test]
fn test_tc_i_092_01_five_edition_ranking() {
    // Detected durations: 11 tracks (simulated from silence detection)
    let detected_durations = vec![
        180.0, 210.0, 195.0, 220.0, 185.0, 200.0, 215.0, 190.0, 205.0, 198.0, 225.0,
    ];
    let detected_total_ms: u64 = detected_durations.iter().map(|&d| (d * 1000.0) as u64).sum();
    let tolerance_secs = 1.5;

    // Edition 1: Standard Edition (11 tracks, exact match)
    let standard_durations = vec![
        180.2, 210.1, 195.3, 219.8, 185.1, 200.0, 215.2, 189.9, 205.1, 198.2, 225.0,
    ];
    let standard_total_ms: u64 = standard_durations.iter().map(|&d| (d * 1000.0) as u64).sum();

    let duration_score_std = calculate_total_duration_score(detected_total_ms, standard_total_ms);
    let quality_score_std =
        calculate_track_quality_score(&detected_durations, &standard_durations, tolerance_secs);
    let name_score_std = 0.85; // High name similarity
    let track_penalty_std = calculate_track_count_penalty(11, 11);

    // Match score: 11/11 tracks match (100%)
    let match_score_std = 1.0;
    let score_std = calculate_edition_score(
        duration_score_std,
        match_score_std,
        quality_score_std,
        name_score_std,
        track_penalty_std,
    );

    // Edition 2: Deluxe Edition (17 tracks, 6 bonus tracks)
    let deluxe_durations = vec![
        180.2, 210.1, 195.3, 219.8, 185.1, 200.0, 215.2, 189.9, 205.1, 198.2, 225.0, 240.0,
        235.0, 220.0, 250.0, 210.0, 230.0,
    ];
    let deluxe_total_ms: u64 = deluxe_durations.iter().map(|&d| (d * 1000.0) as u64).sum();

    let duration_score_dlx = calculate_total_duration_score(detected_total_ms, deluxe_total_ms);
    let quality_score_dlx =
        calculate_track_quality_score(&detected_durations, &deluxe_durations, tolerance_secs);
    let name_score_dlx = 0.80; // Slightly lower name similarity
    let track_penalty_dlx = calculate_track_count_penalty(11, 17);

    // Match score: 0/17 (track count mismatch - detected 11, edition has 17)
    let match_score_dlx = 0.0;
    let score_dlx = calculate_edition_score(
        duration_score_dlx,
        match_score_dlx,
        quality_score_dlx,
        name_score_dlx,
        track_penalty_dlx,
    );

    // Edition 3: Box Set (147 tracks, massive compilation)
    let mut box_set_durations = standard_durations.clone();
    box_set_durations.extend(vec![200.0; 136]); // Add 136 more tracks
    let box_set_total_ms: u64 = box_set_durations.iter().map(|&d| (d * 1000.0) as u64).sum();

    let duration_score_box = calculate_total_duration_score(detected_total_ms, box_set_total_ms);
    let quality_score_box =
        calculate_track_quality_score(&detected_durations, &box_set_durations, tolerance_secs);
    let name_score_box = 0.85; // Same name similarity as standard
    let track_penalty_box = calculate_track_count_penalty(11, 147);

    // Match score: 0/147 (track count mismatch - detected 11, edition has 147)
    let match_score_box = 0.0;
    let score_box = calculate_edition_score(
        duration_score_box,
        match_score_box,
        quality_score_box,
        name_score_box,
        track_penalty_box,
    );

    // Edition 4: Japanese Edition (12 tracks, 1 bonus)
    let mut japanese_durations = standard_durations.clone();
    japanese_durations.push(240.0); // One bonus track
    let japanese_total_ms: u64 = japanese_durations.iter().map(|&d| (d * 1000.0) as u64).sum();

    let duration_score_jpn = calculate_total_duration_score(detected_total_ms, japanese_total_ms);
    let quality_score_jpn =
        calculate_track_quality_score(&detected_durations, &japanese_durations, tolerance_secs);
    let name_score_jpn = 0.82; // Slightly different name
    let track_penalty_jpn = calculate_track_count_penalty(11, 12);

    // Match score: 0/12 (track count mismatch - detected 11, edition has 12)
    let match_score_jpn = 0.0;
    let score_jpn = calculate_edition_score(
        duration_score_jpn,
        match_score_jpn,
        quality_score_jpn,
        name_score_jpn,
        track_penalty_jpn,
    );

    // Edition 5: Remaster (11 tracks, slight duration differences)
    let remaster_durations = vec![
        181.5, 211.0, 196.0, 221.0, 186.0, 201.0, 216.0, 191.0, 206.0, 199.0, 226.0,
    ];
    let remaster_total_ms: u64 = remaster_durations.iter().map(|&d| (d * 1000.0) as u64).sum();

    let duration_score_rem = calculate_total_duration_score(detected_total_ms, remaster_total_ms);
    let quality_score_rem =
        calculate_track_quality_score(&detected_durations, &remaster_durations, tolerance_secs);
    let name_score_rem = 0.78; // Lower name similarity (includes "Remaster")
    let track_penalty_rem = calculate_track_count_penalty(11, 11);

    // Match score: 11/11 tracks match (100% - remaster has same track count)
    let match_score_rem = 1.0;
    let score_rem = calculate_edition_score(
        duration_score_rem,
        match_score_rem,
        quality_score_rem,
        name_score_rem,
        track_penalty_rem,
    );

    // Create candidates
    let candidates = vec![
        EditionCandidate {
            mbid: "std-001".to_string(),
            title: "Album Title".to_string(),
            track_count: 11,
            total_duration_ms: standard_total_ms,
            track_durations_secs: standard_durations,
            name_similarity: name_score_std,
            score: score_std,
        },
        EditionCandidate {
            mbid: "dlx-002".to_string(),
            title: "Album Title (Deluxe Edition)".to_string(),
            track_count: 17,
            total_duration_ms: deluxe_total_ms,
            track_durations_secs: deluxe_durations,
            name_similarity: name_score_dlx,
            score: score_dlx,
        },
        EditionCandidate {
            mbid: "box-003".to_string(),
            title: "Complete Box Set".to_string(),
            track_count: 147,
            total_duration_ms: box_set_total_ms,
            track_durations_secs: box_set_durations,
            name_similarity: name_score_box,
            score: score_box,
        },
        EditionCandidate {
            mbid: "jpn-004".to_string(),
            title: "Album Title (Japan)".to_string(),
            track_count: 12,
            total_duration_ms: japanese_total_ms,
            track_durations_secs: japanese_durations,
            name_similarity: name_score_jpn,
            score: score_jpn,
        },
        EditionCandidate {
            mbid: "rem-005".to_string(),
            title: "Album Title (Remastered)".to_string(),
            track_count: 11,
            total_duration_ms: remaster_total_ms,
            track_durations_secs: remaster_durations,
            name_similarity: name_score_rem,
            score: score_rem,
        },
    ];

    // Select best edition
    let best = select_best_edition(&candidates).expect("Should select best edition");

    // Verify: Standard edition should win
    assert_eq!(best.mbid, "std-001", "Standard edition should win");
    assert_eq!(best.track_count, 11, "Winner should have 11 tracks");

    // Verify: Box set has lowest score
    assert!(
        score_box < score_std,
        "Box set score ({}) should be lower than standard ({})",
        score_box,
        score_std
    );
    assert!(
        score_box < score_dlx,
        "Box set score should be lower than deluxe"
    );
    assert!(
        score_box < score_jpn,
        "Box set score should be lower than Japanese"
    );
    assert!(
        score_box < score_rem,
        "Box set score should be lower than remaster"
    );

    // Verify: Japanese edition should be competitive (±1 track penalty minimal)
    let score_diff_std_jpn = (score_std - score_jpn).abs();
    assert!(
        score_diff_std_jpn < 0.15,
        "Japanese edition should be within ~15% of standard (diff: {})",
        score_diff_std_jpn
    );

    // Verify: Deluxe edition penalized more than Japanese (±6 tracks)
    assert!(
        score_jpn > score_dlx,
        "Japanese (+1 track) should score higher than deluxe (+6 tracks)"
    );

    println!("5-Edition Ranking Results:");
    println!("  1. Standard:  {:.4}", score_std);
    println!("  2. Japanese:  {:.4}", score_jpn);
    println!("  3. Remaster:  {:.4}", score_rem);
    println!("  4. Deluxe:    {:.4}", score_dlx);
    println!("  5. Box Set:   {:.4}", score_box);
}

/// TC-I-092-01 Variant: Verify deterministic tie-breaking
///
/// Additional test to verify that when standard and remaster have identical
/// scores, tie-breaking is deterministic based on name_similarity and MBID.
#[test]
fn test_tc_i_092_01_deterministic_tie_breaking() {
    let detected_durations = vec![180.0, 210.0, 195.0];
    let edition_durations = vec![180.0, 210.0, 195.0];
    let detected_total_ms: u64 = detected_durations.iter().map(|&d| (d * 1000.0) as u64).sum();
    let edition_total_ms: u64 = edition_durations.iter().map(|&d| (d * 1000.0) as u64).sum();
    let tolerance_secs = 1.5;

    let duration_score = calculate_total_duration_score(detected_total_ms, edition_total_ms);
    let quality_score =
        calculate_track_quality_score(&detected_durations, &edition_durations, tolerance_secs);
    let track_penalty = calculate_track_count_penalty(3, 3);

    // Three editions with identical track counts and durations
    // All have 100% match (3/3 tracks)
    let match_score = 1.0;
    let score_a = calculate_edition_score(duration_score, match_score, quality_score, 0.85, track_penalty);
    let score_b = calculate_edition_score(duration_score, match_score, quality_score, 0.85, track_penalty);
    let score_c = calculate_edition_score(duration_score, match_score, quality_score, 0.90, track_penalty);

    let candidates = vec![
        EditionCandidate {
            mbid: "zzz-999".to_string(),
            title: "Edition A".to_string(),
            track_count: 3,
            total_duration_ms: edition_total_ms,
            track_durations_secs: edition_durations.clone(),
            name_similarity: 0.85,
            score: score_a,
        },
        EditionCandidate {
            mbid: "aaa-111".to_string(),
            title: "Edition B".to_string(),
            track_count: 3,
            total_duration_ms: edition_total_ms,
            track_durations_secs: edition_durations.clone(),
            name_similarity: 0.85,
            score: score_b,
        },
        EditionCandidate {
            mbid: "mmm-555".to_string(),
            title: "Edition C".to_string(),
            track_count: 3,
            total_duration_ms: edition_total_ms,
            track_durations_secs: edition_durations,
            name_similarity: 0.90,
            score: score_c,
        },
    ];

    let best = select_best_edition(&candidates).expect("Should select best edition");

    // C should win (highest name_similarity)
    assert_eq!(best.mbid, "mmm-555", "Edition C should win (highest name_similarity)");

    // Test with all identical scores and name_similarity
    let candidates_tie = vec![
        EditionCandidate {
            mbid: "zzz-999".to_string(),
            title: "Edition A".to_string(),
            track_count: 3,
            total_duration_ms: edition_total_ms,
            track_durations_secs: vec![180.0, 210.0, 195.0],
            name_similarity: 0.85,
            score: score_a,
        },
        EditionCandidate {
            mbid: "aaa-111".to_string(),
            title: "Edition B".to_string(),
            track_count: 3,
            total_duration_ms: edition_total_ms,
            track_durations_secs: vec![180.0, 210.0, 195.0],
            name_similarity: 0.85,
            score: score_b,
        },
    ];

    let best_tie = select_best_edition(&candidates_tie).expect("Should select best edition");

    // B should win (lexicographically smallest MBID: "aaa" < "zzz")
    assert_eq!(
        best_tie.mbid, "aaa-111",
        "Edition B should win (lexicographically smallest MBID)"
    );
}
