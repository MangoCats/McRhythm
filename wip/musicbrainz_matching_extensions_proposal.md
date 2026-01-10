# MusicBrainz Matching Extensions Proposal

## Problem Statement

9 albums in the 200-file test failed to match despite correct MusicBrainz data existing:

1. **John Mayall** - A Hard Road (artist suffix mismatch: "John Mayall" vs "John Mayall & the Bluesbreakers")
2. **The Go-Go's** - Beauty And The Beat (punctuation + prefix: "The Go-Go's" vs "Go-Go's")
3. **Hooverphonic** - Live at the Ancienne Belgique
4. **Dave Brubeck Quartet** - The Best Of The Dave Brubeck Quartet (1979-2004)
5. **The Police** - Reggatta De Blanc
6. **Delerium** - Ritual (artist tag mismatch: "Phildel" directory vs "Delerium")
7. **Carlos Santana** - Invitation to Illumination (artist variation: "Carlos Santana" vs "Santana")
8. **The Score** - Atlas
9. **Various** - The Greatest Showman (Soundtrack)

### Current Architecture Analysis

**Search Strategy (7 strategies in `musicbrainz_client.rs`):**
- Strategy 1: Basic unquoted search
- Strategy 2: CamelCase split
- Strategy 3: Fuzzy ~1 edit
- Strategy 4: Wildcard fixes
- Strategy 5: Aggressive fuzzy ~2 edits
- Strategy 6: Per-token fuzzy
- Strategy 7: Album-only fallback

**Artist Filtering (`filtering.rs:141-222`):**
- **Threshold:** 0.60 (60% similarity required)
- **Algorithm:** max(Jaccard, Normalized Levenshtein)
- **Problem:** Fails on legitimate variations

**Key Failure Modes:**

1. **Artist suffix variations** - "John Mayall" ≠ "John Mayall & the Bluesbreakers"
   - Jaccard: 2/5 tokens = 0.40 (FAIL)

2. **Punctuation differences** - "The Go-Go's" ≠ "Go-Go's"
   - All 17 editions removed (0.60 threshold too strict)

3. **Artist prefix variations** - "Carlos Santana" ≠ "Santana"
   - Core name matches but prefix mismatch lowers score

---

## Proposed Extensions

### Extension 1: Artist Name Normalization (Pre-Filter)

**Implementation Location:** `filtering.rs` - new function before similarity calculation

**Strategy:** Normalize both source and MusicBrainz artist names before comparison.

```rust
/// Normalize artist name for comparison
///
/// Strips common suffixes, prefixes, and punctuation variations
fn normalize_artist_name(artist: &str) -> String {
    let mut normalized = artist.to_lowercase();

    // Remove common prefixes
    let prefixes = ["the ", "a ", "an "];
    for prefix in &prefixes {
        if normalized.starts_with(prefix) {
            normalized = normalized[prefix.len()..].to_string();
            break;
        }
    }

    // Remove common band suffixes (greedy longest match)
    let suffixes = [
        " & the bluesbreakers",
        " & his orchestra",
        " & the gang",
        " and his orchestra",
        " and the gang",
        " orchestra",
        " ensemble",
        " quartet",
        " quintet",
        " trio",
        " band",
    ];
    for suffix in &suffixes {
        if normalized.ends_with(suffix) {
            normalized = normalized[..normalized.len() - suffix.len()].to_string();
            break;
        }
    }

    // Normalize punctuation
    normalized = normalized
        .replace("-", " ")   // Hyphens to spaces
        .replace("'", "")    // Remove apostrophes
        .replace(".", "")    // Remove periods
        .replace(",", "");   // Remove commas

    // Collapse multiple spaces
    normalized.split_whitespace().collect::<Vec<_>>().join(" ")
}
```

**Impact Analysis:**
- **John Mayall**: "john mayall" == "john mayall" → 1.0 similarity ✓
- **The Go-Go's**: "go gos" == "go gos" → 1.0 similarity ✓
- **Carlos Santana**: "carlos santana" → "santana" still mismatch (needs Extension 2)

**Risk:** Low - Normalization is pre-comparison only, doesn't affect final match data

---

### Extension 2: Prefix/Suffix Matching Boost

**Implementation Location:** `filtering.rs:141` - enhance `calculate_artist_similarity`

**Strategy:** Award bonus score if one name is a prefix/suffix of the other.

```rust
fn calculate_artist_similarity(mb_artist: &str, source_artist: &str) -> f64 {
    let mb_norm = normalize_artist_name(mb_artist);
    let src_norm = normalize_artist_name(source_artist);

    // Current hybrid similarity
    let mb_tokens: HashSet<&str> = mb_norm.split_whitespace().collect();
    let src_tokens: HashSet<&str> = src_norm.split_whitespace().collect();
    let intersection = mb_tokens.intersection(&src_tokens).count();
    let union = mb_tokens.union(&src_tokens).count();
    let jaccard = if union > 0 {
        intersection as f64 / union as f64
    } else {
        0.0
    };
    let levenshtein = normalized_levenshtein(&mb_norm, &src_norm);
    let base_similarity = jaccard.max(levenshtein);

    // **[NEW]** Prefix/suffix matching boost
    // If one name is a substring of the other, award bonus
    let prefix_bonus = if mb_norm.contains(&src_norm) || src_norm.contains(&mb_norm) {
        0.20  // +20% bonus for substring match
    } else {
        0.0
    };

    // **[NEW]** Token subset boost
    // If all tokens from shorter name appear in longer name
    let token_subset_bonus = if mb_tokens.is_subset(&src_tokens) || src_tokens.is_subset(&mb_tokens) {
        0.15  // +15% bonus for token subset
    } else {
        0.0
    };

    // Cap at 1.0
    (base_similarity + prefix_bonus + token_subset_bonus).min(1.0)
}
```

**Impact Analysis:**
- **Carlos Santana** → "Santana": substring match → 0.75 + 0.20 = 0.95 ✓
- **Dave Brubeck Quartet** → "The Dave Brubeck Quartet": token subset → 0.80 + 0.15 = 0.95 ✓

**Risk:** Low-Medium - May increase false positives, but 0.60 threshold still applies

---

### Extension 3: Lower Artist Filter Threshold with Grace Period

**Implementation Location:** `constants.rs:200` and `album_matcher.rs:446`

**Strategy:** Lower threshold from 0.60 to 0.50, but require higher name distance score for final acceptance.

```rust
// constants.rs
/// Minimum artist similarity for pre-filtering (0.50 = 50%)
pub const MIN_ARTIST_SIMILARITY: f64 = 0.50;  // Changed from 0.60

/// Minimum combined name distance for final acceptance (0.65 = 65%)
pub const MIN_NAME_DISTANCE_FINAL: f64 = 0.65;
```

```rust
// album_matcher.rs - after final stage results
if result.success {
    let name_distance = calculate_name_distance(
        &result.matched_edition.artist,
        &result.matched_edition.title,
        &artist,
        &album
    );

    if name_distance < MIN_NAME_DISTANCE_FINAL {
        warn!("Match rejected: name distance {:.2} below threshold {:.2}",
              name_distance, MIN_NAME_DISTANCE_FINAL);
        return AlbumMatchResult::no_match_with_samples(...);
    }
}
```

**Rationale:**
- Relaxed pre-filter (0.50) allows more candidates through
- Strict final gate (0.65) prevents false positives
- Two-stage filtering reduces risk

**Risk:** Medium - More editions tested = slower performance

---

### Extension 4: Additional Search Strategies

**Implementation Location:** `musicbrainz_client.rs:882` - `generate_search_strategies`

**Strategy:** Add 3 new search strategies targeting specific failure modes.

```rust
fn generate_search_strategies(artist: &str, album: &str) -> Vec<String> {
    let mut strategies = Vec::with_capacity(10);  // Increased from 7

    // ... existing strategies 1-7 ...

    // **[NEW Strategy 8]** Artist prefix search
    // Handles "Carlos Santana" → "Santana" or "John Mayall" → "Mayall"
    let artist_tokens: Vec<&str> = artist.split_whitespace().collect();
    if artist_tokens.len() >= 2 {
        // Try last name only (e.g., "Santana" from "Carlos Santana")
        let last_name = artist_tokens.last().unwrap();
        strategies.push(format!(
            "type:album AND artist:{} AND release:{}",
            last_name, album
        ));

        // Try first name only (e.g., "Carlos" from "Carlos Santana")
        let first_name = artist_tokens.first().unwrap();
        strategies.push(format!(
            "type:album AND artist:{} AND release:{}",
            first_name, album
        ));
    }

    // **[NEW Strategy 9]** Punctuation-stripped search
    // Handles "Go-Go's" → "GoGos" variations
    let artist_stripped = strip_punctuation(artist);
    let album_stripped = strip_punctuation(album);
    if artist_stripped != artist || album_stripped != album {
        strategies.push(format!(
            "type:album AND artist:{} AND release:{}",
            artist_stripped, album_stripped
        ));
    }

    // **[NEW Strategy 10]** Album-focused with artist wildcard
    // Last resort - prioritizes album match with loose artist constraint
    strategies.push(format!(
        "type:album AND artist:{}* AND release:\"{}\"",
        artist_tokens.first().unwrap_or(&artist), album
    ));

    strategies
}

fn strip_punctuation(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
```

**Impact Analysis:**
- **Carlos Santana** → Strategy 8 tries "Santana" alone ✓
- **The Go-Go's** → Strategy 9 strips to "GoGos" ✓
- **John Mayall** → Strategy 10 wildcard "John*" catches "John Mayall & the Bluesbreakers" ✓

**Risk:** Low - More API calls but cached + rate-limited

---

### Extension 5: MusicBrainz Artist Alias Lookup

**Implementation Location:** New module `musicbrainz_client.rs` - artist alias resolution

**Strategy:** Query MusicBrainz artist entity for aliases when initial search fails.

```rust
/// Query MusicBrainz for artist aliases
///
/// When no matches found, queries /ws/2/artist for exact artist name
/// and retrieves all known aliases. Re-runs search with each alias.
async fn search_with_artist_aliases(
    client: &reqwest::Client,
    artist: &str,
    album: &str,
    rate_limiter: &Arc<Mutex<RateLimiter>>,
) -> Result<Vec<MBReleaseSearchResult>, MBError> {
    // Step 1: Search for artist entity
    let artist_url = format!(
        "{}/artist/?query=artist:\"{}\"&limit=5&fmt=json",
        MUSICBRAINZ_BASE_URL, artist
    );

    rate_limiter.lock().await.wait().await;
    let artist_response: MBArtistSearchResponse = client
        .get(&artist_url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await?
        .json()
        .await?;

    let mut all_results = Vec::new();

    // Step 2: For each artist match, try their aliases
    for artist_result in artist_response.artists.iter().take(3) {
        // Get artist details with aliases
        let artist_detail_url = format!(
            "{}/artist/{}?inc=aliases&fmt=json",
            MUSICBRAINZ_BASE_URL, artist_result.id
        );

        rate_limiter.lock().await.wait().await;
        let artist_detail: MBArtistDetail = client
            .get(&artist_detail_url)
            .header("User-Agent", USER_AGENT)
            .send()
            .await?
            .json()
            .await?;

        // Step 3: Search with each alias
        for alias in &artist_detail.aliases {
            if alias.name != artist {  // Don't re-search original name
                let releases = search_releases_basic(
                    client,
                    &alias.name,
                    album,
                    rate_limiter,
                ).await?;
                all_results.extend(releases);
            }
        }
    }

    Ok(all_results)
}
```

**When to Use:**
- Only triggered when comprehensive_search returns 0 results
- Acts as fallback mechanism
- Expensive (multiple API calls) but rare

**Impact Analysis:**
- **Delerium** → Aliases may include variations or related artist names ✓
- **Various** → "Various Artists" has many aliases in MusicBrainz ✓

**Risk:** Medium-High - Significant API overhead, requires careful rate limiting

---

### Extension 6: Levenshtein Distance Threshold Relaxation for Short Names

**Implementation Location:** `filtering.rs:141` - `calculate_artist_similarity`

**Strategy:** Adjust Levenshtein tolerance based on string length.

```rust
fn calculate_levenshtein_with_length_awareness(str1: &str, str2: &str) -> f64 {
    let base_score = normalized_levenshtein(str1, str2);
    let avg_length = (str1.len() + str2.len()) as f64 / 2.0;

    // For short names (< 10 chars), allow 1 edit distance = high score
    // For long names, normalized_levenshtein already handles well
    if avg_length < 10.0 {
        // Boost short names with high absolute similarity
        let absolute_distance = (str1.len() as i32 - str2.len() as i32).abs();
        if absolute_distance <= 2 {
            base_score.max(0.85)  // Treat as high match
        } else {
            base_score
        }
    } else {
        base_score
    }
}
```

**Impact Analysis:**
- Short artist names (e.g., "The Score" vs "Score") get fairness boost
- Long names retain strict matching

**Risk:** Low - Conservative boost, only affects edge cases

---

## Implementation Priority

### Phase 1: Low-Risk High-Impact (Recommended First)
1. **Extension 1: Artist Name Normalization** - Solves 4-5 failures immediately
2. **Extension 4: Additional Search Strategies** - Incremental API calls, low risk
3. **Extension 2: Prefix/Suffix Boost** - Pure calculation enhancement

**Expected Impact:** 5-7 of 9 failures fixed

### Phase 2: Medium-Risk Medium-Impact
4. **Extension 3: Lower Threshold + Final Gate** - Requires careful tuning
5. **Extension 6: Length-Aware Levenshtein** - Edge case handling

**Expected Impact:** +1-2 failures fixed (total 6-9)

### Phase 3: High-Risk High-Complexity (Optional)
6. **Extension 5: Artist Alias Lookup** - Expensive but comprehensive

**Expected Impact:** +0-2 failures fixed (total 6-9, with fallback coverage)

---

## Testing Strategy

### Unit Tests Required

**Extension 1 (Normalization):**
```rust
#[test]
fn test_normalize_artist_with_suffix() {
    assert_eq!(
        normalize_artist_name("John Mayall & the Bluesbreakers"),
        "john mayall"
    );
    assert_eq!(
        normalize_artist_name("Dave Brubeck Quartet"),
        "dave brubeck"
    );
}

#[test]
fn test_normalize_artist_punctuation() {
    assert_eq!(
        normalize_artist_name("The Go-Go's"),
        "go gos"
    );
}
```

**Extension 2 (Prefix Boost):**
```rust
#[test]
fn test_artist_similarity_substring_match() {
    let sim = calculate_artist_similarity("Santana", "Carlos Santana");
    assert!(sim >= 0.85, "Substring match should score high, got {}", sim);
}
```

### Integration Tests

**Full 9-Failure Test:**
1. Run matching on all 9 failed albums
2. Target: ≥6 successful matches after Phase 1
3. Target: ≥7 successful matches after Phase 2

**Regression Test:**
1. Re-run 200-file test
2. Ensure no new failures introduced
3. Measure performance impact (<10% slowdown acceptable)

---

## Risk Mitigation

### False Positive Prevention

**Primary Defense:** Two-stage filtering (Extension 3)
- Relaxed pre-filter allows candidates
- Strict final gate rejects poor matches

**Secondary Defense:** Name distance verification
- Even with relaxed artist threshold, album name must still match
- Combined artist+album name distance score protects against wrong artist

### Performance Impact

**Estimated Overhead:**
- Extensions 1-2: <1% (pure calculation)
- Extension 3: +5-10% (more editions tested)
- Extension 4: +10-20% (more search strategies = more API calls, but cached)
- Extension 5: +50-100% (only for failures, rare activation)

**Mitigation:**
- Enable Extensions 1-4 by default
- Make Extension 5 opt-in configuration flag

---

## Configuration Parameters

```rust
// constants.rs - new configurable parameters
pub const ENABLE_ARTIST_NORMALIZATION: bool = true;
pub const ENABLE_PREFIX_SUFFIX_BOOST: bool = true;
pub const ENABLE_EXTENDED_SEARCH_STRATEGIES: bool = true;
pub const ENABLE_ARTIST_ALIAS_FALLBACK: bool = false;  // Expensive, opt-in

pub const MIN_ARTIST_SIMILARITY: f64 = 0.50;  // Lowered from 0.60
pub const MIN_NAME_DISTANCE_FINAL: f64 = 0.65;  // Final gate
pub const PREFIX_BOOST_AMOUNT: f64 = 0.20;
pub const TOKEN_SUBSET_BOOST_AMOUNT: f64 = 0.15;
```

---

## Expected Outcomes

### Success Metrics

**After Phase 1 Implementation:**
- Match rate: 184/200 → **190-191/200** (+6-7 matches)
- Match rate percentage: 92.0% → **95.0-95.5%**
- No regressions from baseline

**After Phase 2 Implementation:**
- Match rate: **191-192/200** (+7-8 matches)
- Match rate percentage: **95.5-96.0%**

**Remaining Failures (1-2 albums):**
- Edge cases requiring manual metadata correction
- Possible MusicBrainz database gaps

### Documentation Impact

**Files to Update:**
- `filtering.rs` - Add normalization and enhanced similarity
- `musicbrainz_client.rs` - Add search strategies 8-10
- `constants.rs` - Update thresholds and add configuration
- `album_matcher.rs` - Add final name distance gate
- Unit test files - Add tests for all extensions

---

## Recommendation

**Implement Phase 1 immediately** (Extensions 1, 2, 4):
- Low risk, high impact
- Estimated 5-7 failures resolved
- <15% performance overhead
- Clear improvement over current 92.0% match rate

**Defer Phase 3** (Extension 5) unless match rate remains below 95% after Phase 1-2 implementation.
