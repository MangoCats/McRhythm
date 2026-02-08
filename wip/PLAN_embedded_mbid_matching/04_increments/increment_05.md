# Increment 5: Integration Testing and Validation

**Increment:** 5 of 5
**Phase:** Testing/Validation
**Estimated Effort:** 2-3 hours
**Confidence:** HIGH (±25%)
**Prerequisites:** Increment 4 (All implementation complete)

---

## Objective

Validate the complete Stage 0 implementation against real audio files and verify all success criteria are met.

---

## Deliverables

1. **New:** Integration test suite
   - `wkmp-ai/tests/stage0_integration.rs`

2. **Validation Report:**
   - Coverage metrics (target: >= 98%)
   - Tier distribution analysis
   - Performance benchmarks

---

## Requirements Covered

- **NFR-01:** Stage 0 matching < 50ms per file (verify)
- **NFR-02:** Total coverage >= 98% (verify)
- **NFR-03:** Tier 1 false positive rate < 0.1% (verify)
- **NFR-04:** Stage 0 works without network (verify)

---

## Implementation Notes

**Integration Test Structure:**
```rust
// tests/stage0_integration.rs

#[tokio::test]
async fn test_stage0_with_embedded_mbid() {
    // Setup: Use test file with known embedded MB ID
    let file_path = test_file_with_embedded_mbid();

    // Execute: Run Stage 0 identification
    let result = identify_recording(&file_path).await.unwrap();

    // Verify: Should be Tier 1A or 1B
    assert!(matches!(
        result.confidence_tier,
        ConfidenceTier::Tier1A | ConfidenceTier::Tier1B
    ));
    assert!(result.recording_mbid.is_some());
}

#[tokio::test]
async fn test_stage0_fallback_to_acoustid() {
    // Setup: Use test file WITHOUT embedded MB ID
    let file_path = test_file_without_embedded_mbid();

    // Execute: Run identification
    let result = identify_recording(&file_path).await.unwrap();

    // Verify: Should NOT be Tier 1
    assert!(!matches!(
        result.confidence_tier,
        ConfidenceTier::Tier1A | ConfidenceTier::Tier1B
    ));
}

#[tokio::test]
async fn test_stage0_performance() {
    // Setup: Collect 100 test files
    let files = collect_test_files(100);

    // Execute: Time Stage 0 only (no AcoustID)
    let start = Instant::now();
    for file in &files {
        let _ = extract_id3_and_check_stage0(file).await;
    }
    let elapsed = start.elapsed();

    // Verify: Should be < 50ms per file (< 5s total)
    let per_file_ms = elapsed.as_millis() as f64 / files.len() as f64;
    assert!(per_file_ms < 50.0, "Stage 0 too slow: {:.1}ms/file", per_file_ms);
}

#[tokio::test]
async fn test_stage0_offline_capability() {
    // Setup: Disable network (mock)
    let _guard = disable_network();

    // Execute: Run Stage 0 identification
    let file_path = test_file_with_embedded_mbid();
    let result = identify_recording_stage0_only(&file_path).await;

    // Verify: Should succeed without network
    assert!(result.is_ok());
    assert!(result.unwrap().recording_mbid.is_some());
}
```

**Validation Against Test Dataset:**
```rust
#[tokio::test]
#[ignore] // Run manually: cargo test --test stage0_integration validation -- --ignored
async fn test_stage0_validation_full_dataset() {
    // Load crossval results
    let crossval_results = load_crossval_results().await;

    // Run Stage 0 on all files
    let mut tier_counts = HashMap::new();
    for result in &crossval_results {
        let stage0 = identify_stage0(&result.file_path).await;
        *tier_counts.entry(stage0.confidence_tier).or_insert(0) += 1;
    }

    // Calculate metrics
    let total = crossval_results.len();
    let tier1_count = tier_counts.get(&ConfidenceTier::Tier1A).unwrap_or(&0)
        + tier_counts.get(&ConfidenceTier::Tier1B).unwrap_or(&0);
    let coverage = tier1_count as f64 / total as f64;

    // Verify success criteria
    assert!(coverage >= 0.98, "Coverage {:.1}% < 98%", coverage * 100.0);

    println!("Validation Results:");
    println!("  Total files: {}", total);
    println!("  Tier 1A: {}", tier_counts.get(&ConfidenceTier::Tier1A).unwrap_or(&0));
    println!("  Tier 1B: {}", tier_counts.get(&ConfidenceTier::Tier1B).unwrap_or(&0));
    println!("  Coverage: {:.1}%", coverage * 100.0);
}
```

---

## Tests to Pass

- **TC-S-001:** Full pipeline integration test
- **TC-S-002:** Performance benchmark (< 50ms/file)
- **TC-S-003:** Coverage validation (>= 98%)
- **TC-S-004:** Offline capability test

---

## Acceptance Criteria

- [ ] Integration tests pass
- [ ] Performance benchmark: Stage 0 < 50ms/file
- [ ] Coverage on test dataset >= 98%
- [ ] No regressions in existing functionality
- [ ] Documentation updated

---

## Verification Commands

```bash
# Unit tests
cargo test -p wkmp-ai

# Integration tests
cargo test --test stage0_integration

# Full validation (takes longer)
cargo test --test stage0_integration validation -- --ignored

# Performance benchmark
cargo bench -p wkmp-ai stage0
```
