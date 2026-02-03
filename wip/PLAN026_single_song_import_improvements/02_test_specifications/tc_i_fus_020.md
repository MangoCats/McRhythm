# TC-I-FUS-020: Multi-Source Fusion

**Requirement:** SSI-FUS-020 (Combine AcoustID + MusicBrainz + ID3)
**Type:** Integration Test

---

## TC-I-FUS-020-01: Three Sources Combined Correctly

**Given:**
- Audio file with:
  - AcoustID match: MBID "rec-123" with score 0.85
  - MusicBrainz search match: MBID "rec-123" with similarity 0.92
  - ID3 metadata suggesting same recording

**When:**
- Multi-source fusion is performed via IdentityResolver

**Then:**
- All three sources contribute to final result
- Sources with same MBID are grouped

**Verify:**
```rust
#[tokio::test]
async fn test_three_sources_combined() {
    let sources = vec![
        IdentityExtraction {
            recording_mbid: "rec-123".to_string(),
            confidence: 0.85,
            source: "AcoustID".to_string(),
        },
        IdentityExtraction {
            recording_mbid: "rec-123".to_string(),
            confidence: 0.92,
            source: "MusicBrainz".to_string(),
        },
        IdentityExtraction {
            recording_mbid: "rec-123".to_string(),
            confidence: 0.80,
            source: "ID3".to_string(),
        },
    ];

    let resolver = IdentityResolver::new();
    let result = resolver.fuse(sources).await.unwrap();

    assert_eq!(result.output.recording_mbid, Some("rec-123".to_string()));
    assert!(result.output.conflicts.is_empty());
}
```

**Pass Criteria:** All sources processed, agreeing sources grouped
**Fail Criteria:** Any source ignored or lost

---

## TC-I-FUS-020-02: Agreeing Sources Boost Confidence

**Given:**
- Two sources agreeing on same MBID with moderate individual confidence

**When:**
- Bayesian fusion is applied

**Then:**
- Combined confidence higher than any individual source
- Formula: posterior = 1 - (1-c1)(1-c2)

**Verify:**
```rust
#[tokio::test]
async fn test_agreeing_sources_boost() {
    let sources = vec![
        IdentityExtraction {
            recording_mbid: "rec-123".to_string(),
            confidence: 0.70,
            source: "AcoustID".to_string(),
        },
        IdentityExtraction {
            recording_mbid: "rec-123".to_string(),
            confidence: 0.60,
            source: "MusicBrainz".to_string(),
        },
    ];

    let resolver = IdentityResolver::new();
    let result = resolver.fuse(sources).await.unwrap();

    // Expected: 1 - (1-0.70)(1-0.60) = 1 - 0.30*0.40 = 1 - 0.12 = 0.88
    let expected = 0.88;
    assert!((result.output.confidence - expected).abs() < 0.01);

    // Combined confidence should exceed both individual sources
    assert!(result.output.confidence > 0.70);
    assert!(result.output.confidence > 0.60);
}
```

**Pass Criteria:** Combined confidence = 1 - (1-c1)(1-c2)
**Fail Criteria:** Confidence not boosted or incorrect formula
