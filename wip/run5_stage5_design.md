# Run 5 - Stage 5 Implementation Design

**Document:** Design Specification for Expanded MusicBrainz Search
**Date:** 2025-11-19
**Status:** Design Phase
**Target:** Run 5 Implementation

---

## Executive Summary

Run 5 introduces **two major enhancements** to the album matching pipeline:

### 1. Phase 0: ID3 Tag Extraction & Reconciliation (NEW)

Extracts ID3 metadata from audio files and reconciles with path/filename information to improve artist/album identification accuracy. Key features:
- Extracts artist, album, date, genre, and MusicBrainz IDs from ID3 tags
- Compares ID3 data with path/filename information
- Reconciles conflicts using heuristics (neither source considered 100% reliable)
- Provides alternate search terms when conflicts detected
- Extracts estimated track count from ID3 comment field (95.5% of files have "X tracks" comments)

**Based on ID3 analysis:** 100% of files have artist/album tags, but only 3.5% have MusicBrainz IDs, making reconciliation critical for accurate searches.

### 2. Stage 5: Expanded MusicBrainz Candidate Search

A comprehensive matching strategy that tests ALL segmentations discovered in Stages 1-4 against additional MusicBrainz release candidates. This stage addresses cases where:
- Initial MusicBrainz candidates don't match the actual file
- Multiple album editions exist (standard, deluxe, international, remaster)
- Segmentation variations may match different release formats

**Key Innovation:** Full cartesian product testing of (all segmentations × additional MB candidates) to maximize match probability.

---

## High-Level Process Flow

### 6-Phase Matching Pipeline

```
Phase 0: ID3 Tag Extraction & Reconciliation ← NEW (Run 5)
    ↓
Stage 1: Initial Parameters
    ↓ (if < 100% match)
Stage 2: Parameter Optimization (180-grid search)
    ↓ (if < 100% match AND over-segmentation)
Stage 3: Segment Assembly (DP-based merging)
    ↓ (if < 100% match)
Stage 4: Quiet Spot Detection (RMS amplitude analysis)
    ↓ (if < 100% match AND additional MB candidates exist)
Stage 5: Expanded MusicBrainz Search
    ↓
Final Result
```

### Stage 5 Trigger Conditions

Stage 5 executes when **ALL** conditions met:
1. **best_percentage < 100.0** (not yet perfect match)
2. **Additional MusicBrainz candidates exist** (more than initially tested)
3. **Segmentation pool is non-empty** (Stages 1-4 produced valid segmentations)

---

## Phase 0: ID3 Tag Extraction & Reconciliation

**Purpose:** Extract ID3 metadata from audio file and reconcile with path/filename information to improve artist/album identification accuracy.

**Key Principle:** Neither ID3 tags nor path/filename are 100% reliable. Use both sources as guidance, remaining open to the possibility that either or both may be misleading.

### Phase 0 Process Flow

```
1. Extract ID3 tags from file (ffprobe)
   ↓
2. Extract artist/album from path/filename (existing logic)
   ↓
3. Compare and reconcile both sources
   ↓
4. Select most reliable artist/album for MusicBrainz search
   ↓
5. Log reconciliation decision for transparency
```

### ID3 Tag Information Extracted

From the ID3 analysis ([id3_analysis_summary.md](id3_analysis_summary.md)), typical files contain:

**Always Available (100%):**
- `artist` - Artist name
- `album` - Album title

**Frequently Available:**
- `date` - Release year (78% of files)
- `comment` - Track count indicator (95.5%)
- `genre` - Musical genre (8.5%)

**Rarely Available (3.5%):**
- `title` - Track title
- MusicBrainz IDs (Album, Artist, Release)

### Reconciliation Strategies

#### Strategy 1: Direct Match (High Confidence)

**Condition:** ID3 artist/album matches path/filename information (case-insensitive, allowing minor variations)

**Action:** Use either source with high confidence

**Example:**
```
Path:     C:\Users\Music\John, Elton\GoodbyeYellowBrickRoad.mp3
ID3:      artist="Elton John", album="Goodbye Yellow Brick Road"
Decision: MATCH → Use "Elton John" / "Goodbye Yellow Brick Road"
Confidence: High
```

#### Strategy 2: Partial Match (Medium Confidence)

**Condition:** One field matches, one differs

**Action:** Use matching field from both sources, investigate mismatch

**Example:**
```
Path:     C:\Users\Music\Beatles, The\WhiteAlbum.mp3
ID3:      artist="The Beatles", album="The Beatles" (actual album title)
Decision: PARTIAL → Use artist="The Beatles", flag album mismatch
           Try both "WhiteAlbum" and "The Beatles" in MusicBrainz search
Confidence: Medium
```

#### Strategy 3: Conflict (Low Confidence)

**Condition:** ID3 artist/album completely different from path/filename

**Action:** Try both sources in MusicBrainz search, prioritize by heuristics

**Example:**
```
Path:     C:\Users\Music\Various\Moana.mp3
ID3:      artist="Various Artists", album="Moana (Original Soundtrack)"
Decision: CONFLICT → Use ID3 (more specific), but note path suggests compilation
Confidence: Low - Flag for manual review if poor match
```

**Prioritization Heuristics:**
1. If ID3 contains "Various Artists" and path contains specific artist → Use path artist
2. If path contains "Various" and ID3 has specific artist → Use ID3 artist
3. If ID3 album more detailed (e.g., includes year, edition) → Prefer ID3
4. If both equally detailed → Try both in sequence

#### Strategy 4: Missing Information (Gap Fill)

**Condition:** One source missing artist or album information

**Action:** Use available source, flag as incomplete metadata

**Example:**
```
Path:     C:\Users\Music\Unknown\BestOf80s.mp3
ID3:      artist="Various Artists", album="Best of the 80s"
Decision: GAP FILL → Use ID3 (path unreliable)
Confidence: Medium
```

### Implementation Data Structure

```rust
#[derive(Debug, Clone)]
struct ID3Metadata {
    /// Artist from ID3 tags (if available)
    artist: Option<String>,

    /// Album from ID3 tags (if available)
    album: Option<String>,

    /// Release date/year from ID3 tags
    date: Option<String>,

    /// Genre from ID3 tags
    genre: Option<String>,

    /// MusicBrainz Album ID (if available - 3.5% of files)
    musicbrainz_albumid: Option<String>,

    /// MusicBrainz Artist ID (if available)
    musicbrainz_artistid: Option<String>,

    /// Comment field (often contains track count)
    comment: Option<String>,

    /// Raw tag map for debugging
    all_tags: HashMap<String, String>,
}

#[derive(Debug)]
struct ReconciledMetadata {
    /// Final artist name for MusicBrainz search
    artist: String,

    /// Final album name for MusicBrainz search
    album: String,

    /// Reconciliation strategy used
    strategy: ReconciliationStrategy,

    /// Confidence level
    confidence: MetadataConfidence,

    /// Source information
    artist_source: MetadataSource,  // ID3, Path, Both, Conflict
    album_source: MetadataSource,

    /// Alternate search terms (if conflict detected)
    alternate_artist: Option<String>,
    alternate_album: Option<String>,

    /// Additional context
    has_musicbrainz_ids: bool,
    estimated_track_count: Option<usize>,  // from comment field
}

#[derive(Debug, Clone, Copy)]
enum ReconciliationStrategy {
    DirectMatch,      // ID3 and path agree
    PartialMatch,     // One field agrees
    Conflict,         // Both disagree
    GapFill,          // One source missing
    ID3Only,          // No path information available
    PathOnly,         // No ID3 information available
}

#[derive(Debug, Clone, Copy)]
enum MetadataConfidence {
    High,    // Direct match or strong consensus
    Medium,  // Partial match or gap fill
    Low,     // Conflict or incomplete data
}

#[derive(Debug, Clone, Copy)]
enum MetadataSource {
    ID3,      // From ID3 tags
    Path,     // From path/filename
    Both,     // Both sources agree
    Conflict, // Sources disagree, choice made via heuristic
}
```

### Phase 0 Algorithm

```rust
fn extract_and_reconcile_metadata(
    file_path: &Path
) -> Result<ReconciledMetadata, Box<dyn std::error::Error>> {

    // Step 1: Extract ID3 tags using ffprobe
    let id3_metadata = extract_id3_tags(file_path)?;

    // Step 2: Extract artist/album from path/filename (existing logic)
    let (path_artist, path_album) = extract_from_path(file_path);

    // Step 3: Reconcile
    let reconciled = reconcile_metadata(&id3_metadata, &path_artist, &path_album);

    // Step 4: Log decision
    log_reconciliation_decision(&reconciled, &id3_metadata, &path_artist, &path_album);

    Ok(reconciled)
}

fn extract_id3_tags(file_path: &Path) -> Result<ID3Metadata, Box<dyn std::error::Error>> {
    // Use ffprobe to extract metadata
    let output = Command::new("ffprobe")
        .args(&[
            "-v", "quiet",
            "-show_format",
            "-of", "json",
            file_path.to_str().unwrap()
        ])
        .output()?;

    if !output.status.success() {
        return Ok(ID3Metadata::default());  // Return empty metadata on failure
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let tags = &json["format"]["tags"];

    // Build tag map (case-insensitive)
    let mut all_tags = HashMap::new();
    if let Some(tag_obj) = tags.as_object() {
        for (key, value) in tag_obj {
            if let Some(v) = value.as_str() {
                all_tags.insert(key.to_lowercase(), v.to_string());
            }
        }
    }

    // Helper to get tag (try multiple case variations)
    let get_tag = |name: &str| -> Option<String> {
        all_tags.get(name)
            .or_else(|| all_tags.get(&name.to_uppercase()))
            .or_else(|| all_tags.get(&name.to_lowercase()))
            .cloned()
    };

    Ok(ID3Metadata {
        artist: get_tag("artist"),
        album: get_tag("album"),
        date: get_tag("date").or_else(|| get_tag("year")),
        genre: get_tag("genre"),
        musicbrainz_albumid: get_tag("musicbrainz album id")
            .or_else(|| get_tag("musicbrainz_albumid")),
        musicbrainz_artistid: get_tag("musicbrainz artist id")
            .or_else(|| get_tag("musicbrainz_artistid")),
        comment: get_tag("comment"),
        all_tags,
    })
}

fn reconcile_metadata(
    id3: &ID3Metadata,
    path_artist: &Option<String>,
    path_album: &Option<String>
) -> ReconciledMetadata {

    // Extract track count estimate from comment if available
    let estimated_track_count = id3.comment.as_ref().and_then(|c| {
        // Parse "52 tracks", "28 tracks", etc.
        c.split_whitespace()
            .next()
            .and_then(|s| s.parse::<usize>().ok())
    });

    // Check if MusicBrainz IDs present
    let has_musicbrainz_ids = id3.musicbrainz_albumid.is_some()
        || id3.musicbrainz_artistid.is_some();

    match (&id3.artist, path_artist, &id3.album, path_album) {
        // Both sources available - compare
        (Some(id3_artist), Some(path_artist_str), Some(id3_album), Some(path_album_str)) => {
            let artist_match = strings_match(id3_artist, path_artist_str);
            let album_match = strings_match(id3_album, path_album_str);

            match (artist_match, album_match) {
                (true, true) => {
                    // Direct match
                    ReconciledMetadata {
                        artist: id3_artist.clone(),
                        album: id3_album.clone(),
                        strategy: ReconciliationStrategy::DirectMatch,
                        confidence: MetadataConfidence::High,
                        artist_source: MetadataSource::Both,
                        album_source: MetadataSource::Both,
                        alternate_artist: None,
                        alternate_album: None,
                        has_musicbrainz_ids,
                        estimated_track_count,
                    }
                }
                (true, false) => {
                    // Artist matches, album differs - try both albums
                    ReconciledMetadata {
                        artist: id3_artist.clone(),
                        album: id3_album.clone(),
                        strategy: ReconciliationStrategy::PartialMatch,
                        confidence: MetadataConfidence::Medium,
                        artist_source: MetadataSource::Both,
                        album_source: MetadataSource::Conflict,
                        alternate_artist: None,
                        alternate_album: Some(path_album_str.clone()),
                        has_musicbrainz_ids,
                        estimated_track_count,
                    }
                }
                (false, true) => {
                    // Album matches, artist differs
                    let (final_artist, alternate) = choose_artist(
                        id3_artist,
                        path_artist_str,
                        id3_album
                    );

                    ReconciledMetadata {
                        artist: final_artist,
                        album: id3_album.clone(),
                        strategy: ReconciliationStrategy::PartialMatch,
                        confidence: MetadataConfidence::Medium,
                        artist_source: MetadataSource::Conflict,
                        album_source: MetadataSource::Both,
                        alternate_artist: Some(alternate),
                        alternate_album: None,
                        has_musicbrainz_ids,
                        estimated_track_count,
                    }
                }
                (false, false) => {
                    // Complete conflict - use heuristics
                    let (artist, artist_alt) = choose_artist(
                        id3_artist,
                        path_artist_str,
                        id3_album
                    );
                    let (album, album_alt) = choose_album(
                        id3_album,
                        path_album_str
                    );

                    ReconciledMetadata {
                        artist,
                        album,
                        strategy: ReconciliationStrategy::Conflict,
                        confidence: MetadataConfidence::Low,
                        artist_source: MetadataSource::Conflict,
                        album_source: MetadataSource::Conflict,
                        alternate_artist: Some(artist_alt),
                        alternate_album: Some(album_alt),
                        has_musicbrainz_ids,
                        estimated_track_count,
                    }
                }
            }
        }

        // ID3 available, path missing - use ID3
        (Some(id3_artist), None, Some(id3_album), None) => {
            ReconciledMetadata {
                artist: id3_artist.clone(),
                album: id3_album.clone(),
                strategy: ReconciliationStrategy::ID3Only,
                confidence: MetadataConfidence::Medium,
                artist_source: MetadataSource::ID3,
                album_source: MetadataSource::ID3,
                alternate_artist: None,
                alternate_album: None,
                has_musicbrainz_ids,
                estimated_track_count,
            }
        }

        // Path available, ID3 missing - use path
        (None, Some(path_artist_str), None, Some(path_album_str)) => {
            ReconciledMetadata {
                artist: path_artist_str.clone(),
                album: path_album_str.clone(),
                strategy: ReconciliationStrategy::PathOnly,
                confidence: MetadataConfidence::Medium,
                artist_source: MetadataSource::Path,
                album_source: MetadataSource::Path,
                alternate_artist: None,
                alternate_album: None,
                has_musicbrainz_ids,
                estimated_track_count,
            }
        }

        // Gap fill scenarios (one field from each source)
        (Some(id3_artist), _, None, Some(path_album_str)) => {
            ReconciledMetadata {
                artist: id3_artist.clone(),
                album: path_album_str.clone(),
                strategy: ReconciliationStrategy::GapFill,
                confidence: MetadataConfidence::Medium,
                artist_source: MetadataSource::ID3,
                album_source: MetadataSource::Path,
                alternate_artist: None,
                alternate_album: None,
                has_musicbrainz_ids,
                estimated_track_count,
            }
        }

        (None, Some(path_artist_str), Some(id3_album), _) => {
            ReconciledMetadata {
                artist: path_artist_str.clone(),
                album: id3_album.clone(),
                strategy: ReconciliationStrategy::GapFill,
                confidence: MetadataConfidence::Medium,
                artist_source: MetadataSource::Path,
                album_source: MetadataSource::ID3,
                alternate_artist: None,
                alternate_album: None,
                has_musicbrainz_ids,
                estimated_track_count,
            }
        }

        // Insufficient data - error case
        _ => {
            panic!("Insufficient metadata: cannot determine artist or album from either source");
        }
    }
}

/// Compare strings with normalization (case-insensitive, whitespace/punctuation tolerant)
fn strings_match(a: &str, b: &str) -> bool {
    let normalize = |s: &str| -> String {
        s.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };

    normalize(a) == normalize(b)
}

/// Choose artist when conflict exists (heuristic-based)
fn choose_artist(id3_artist: &str, path_artist: &str, album: &str) -> (String, String) {
    // Heuristic 1: Avoid "Various Artists" if possible
    if id3_artist.to_lowercase().contains("various")
        && !path_artist.to_lowercase().contains("various") {
        return (path_artist.to_string(), id3_artist.to_string());
    }

    if path_artist.to_lowercase().contains("various")
        && !id3_artist.to_lowercase().contains("various") {
        return (id3_artist.to_string(), path_artist.to_string());
    }

    // Heuristic 2: Prefer ID3 if album name suggests compilation
    if album.to_lowercase().contains("soundtrack")
        || album.to_lowercase().contains("various")
        || album.to_lowercase().contains("compilation") {
        return (id3_artist.to_string(), path_artist.to_string());
    }

    // Default: Prefer ID3 (more likely to be correct from tagging software)
    (id3_artist.to_string(), path_artist.to_string())
}

/// Choose album when conflict exists
fn choose_album(id3_album: &str, path_album: &str) -> (String, String) {
    // Prefer ID3 if it contains more detail (edition, year, etc.)
    if id3_album.len() > path_album.len() + 5 {
        return (id3_album.to_string(), path_album.to_string());
    }

    // Otherwise prefer ID3 as primary, path as alternate
    (id3_album.to_string(), path_album.to_string())
}

fn log_reconciliation_decision(
    reconciled: &ReconciledMetadata,
    id3: &ID3Metadata,
    path_artist: &Option<String>,
    path_album: &Option<String>
) {
    println!("\n=== Phase 0: ID3 Tag Extraction & Reconciliation ===");
    println!("  ID3 Tags:");
    println!("    Artist: {}", id3.artist.as_deref().unwrap_or("(none)"));
    println!("    Album:  {}", id3.album.as_deref().unwrap_or("(none)"));
    if let Some(date) = &id3.date {
        println!("    Date:   {}", date);
    }
    if let Some(comment) = &id3.comment {
        println!("    Comment: {}", comment);
    }
    if reconciled.has_musicbrainz_ids {
        println!("    MusicBrainz IDs: Present ✓");
    }

    println!("\n  Path/Filename:");
    println!("    Artist: {}", path_artist.as_deref().unwrap_or("(none)"));
    println!("    Album:  {}", path_album.as_deref().unwrap_or("(none)"));

    println!("\n  Reconciliation:");
    println!("    Strategy: {:?}", reconciled.strategy);
    println!("    Confidence: {:?}", reconciled.confidence);
    println!("    Final Artist: {} (source: {:?})",
             reconciled.artist, reconciled.artist_source);
    println!("    Final Album: {} (source: {:?})",
             reconciled.album, reconciled.album_source);

    if let Some(alt_artist) = &reconciled.alternate_artist {
        println!("    Alternate Artist: {} (will try if primary fails)", alt_artist);
    }
    if let Some(alt_album) = &reconciled.alternate_album {
        println!("    Alternate Album: {} (will try if primary fails)", alt_album);
    }
    if let Some(count) = reconciled.estimated_track_count {
        println!("    Estimated Track Count: {} (from ID3 comment)", count);
    }

    println!();  // Blank line before Stage 1
}
```

### Integration with Stage 1

**Before:** Stage 1 extracted artist/album from path/filename only

**After:** Stage 1 uses reconciled metadata from Phase 0

```rust
// OLD (Run 4):
let (artist_name, album_name) = extract_artist_album_from_path(file_path)?;

// NEW (Run 5):
let reconciled_metadata = extract_and_reconcile_metadata(file_path)?;
let artist_name = reconciled_metadata.artist;
let album_name = reconciled_metadata.album;

// Use estimated track count for validation (if available)
if let Some(estimated_count) = reconciled_metadata.estimated_track_count {
    println!("  Expected ~{} tracks based on ID3 comment", estimated_count);
}

// If MusicBrainz IDs present in ID3, could use direct lookup (future enhancement)
if reconciled_metadata.has_musicbrainz_ids {
    println!("  Note: File has MusicBrainz IDs - direct lookup possible");
}
```

### Alternate Search Fallback

If primary search yields poor results and alternates exist, retry with alternate terms:

```rust
// After Stage 5 completes with < 80% match AND alternates available
if best_percentage < 80.0 &&
   (reconciled_metadata.alternate_artist.is_some() ||
    reconciled_metadata.alternate_album.is_some()) {

    println!("\n  === Retry with Alternate Metadata ===");

    let retry_artist = reconciled_metadata.alternate_artist
        .as_ref()
        .unwrap_or(&artist_name);
    let retry_album = reconciled_metadata.alternate_album
        .as_ref()
        .unwrap_or(&album_name);

    println!("  Retrying with: Artist='{}', Album='{}'", retry_artist, retry_album);

    // Re-run MusicBrainz search with alternate terms
    // (Could trigger Stages 1-5 again, or just Stage 5 with new candidates)
}
```

**Note:** This fallback adds significant time (2+ minutes per retry). Use only for low-confidence matches where alternates exist.

---

## Data Structures

### 1. Segmentation Record

Tracks each unique segmentation produced during Stages 1-4:

```rust
struct SegmentationRecord {
    /// Detected track durations (seconds)
    durations: Vec<f64>,

    /// Number of tracks detected
    track_count: usize,

    /// Stage that produced this segmentation
    source_stage: String,  // "Initial", "Parameter Optimization", "Segment Assembly", "Quiet Spot Detection"

    /// Parameters used (if applicable)
    threshold_db: Option<f64>,
    min_duration_secs: Option<f64>,

    /// Unique identifier for deduplication
    signature: String,  // Hash of durations rounded to 0.1s
}

impl SegmentationRecord {
    /// Create signature for deduplication
    fn compute_signature(durations: &[f64]) -> String {
        // Round each duration to 0.1s precision and hash
        let rounded: Vec<u64> = durations
            .iter()
            .map(|d| (d * 10.0).round() as u64)
            .collect();
        format!("{:?}", rounded)
    }
}
```

### 2. MusicBrainz Candidate Tracker

Tracks which MB releases have been tested:

```rust
struct MBCandidateTracker {
    /// MBIDs of releases already tested in Stages 1-4
    tested_mbids: HashSet<String>,

    /// Total candidates available from initial search
    total_available: usize,

    /// Initial search response (for fetching additional candidates)
    initial_response: MBSearchResponse,
}

impl MBCandidateTracker {
    /// Get next batch of untested candidates
    fn get_next_batch(&self, offset: usize, limit: usize) -> Vec<&MBRelease> {
        self.initial_response.releases
            .iter()
            .skip(offset)
            .filter(|r| !self.tested_mbids.contains(&r.id))
            .take(limit)
            .collect()
    }

    /// Check if additional candidates exist
    fn has_more_candidates(&self) -> bool {
        let untested_count = self.initial_response.releases
            .iter()
            .filter(|r| !self.tested_mbids.contains(&r.id))
            .count();
        untested_count > 0
    }
}
```

### 3. Stage 5 Match Result

```rust
struct Stage5MatchResult {
    /// Best match percentage achieved
    best_percentage: f64,

    /// Segmentation that produced best match
    best_segmentation: SegmentationRecord,

    /// MusicBrainz release that produced best match
    best_mbid: String,
    best_mb_track_count: usize,

    /// Expected durations from best MB candidate
    best_expected_durations: Vec<u32>,

    /// Track matches for best result
    best_matches: Vec<TrackMatch>,

    /// Statistics
    total_segmentations_tested: usize,
    total_mb_candidates_tested: usize,
    total_combinations_tested: usize,  // segmentations × candidates
}
```

---

## Algorithm Specification

### Phase 1: Segmentation Collection

**Input:** Stages 1-4 execution state
**Output:** `Vec<SegmentationRecord>` (all unique segmentations)

**Algorithm:**

```rust
fn collect_all_segmentations(
    stage1_durations: &[f64],
    stage2_results: &[(Vec<f64>, f64, f64)],  // (durations, threshold_db, min_duration_secs)
    stage3_durations: &Option<Vec<f64>>,
    stage4_durations: &Option<Vec<f64>>,
) -> Vec<SegmentationRecord> {
    let mut segmentations = Vec::new();
    let mut seen_signatures = HashSet::new();

    // Helper to add segmentation if unique
    let mut add_if_unique = |durations: &[f64], stage: &str, threshold: Option<f64>, min_dur: Option<f64>| {
        let sig = SegmentationRecord::compute_signature(durations);
        if !seen_signatures.contains(&sig) {
            seen_signatures.insert(sig.clone());
            segmentations.push(SegmentationRecord {
                durations: durations.to_vec(),
                track_count: durations.len(),
                source_stage: stage.to_string(),
                threshold_db: threshold,
                min_duration_secs: min_dur,
                signature: sig,
            });
        }
    };

    // Stage 1: Initial parameters
    add_if_unique(stage1_durations, "Initial", Some(-57.0), Some(0.9));

    // Stage 2: All parameter combinations tested
    for (durations, threshold, min_dur) in stage2_results {
        add_if_unique(durations, "Parameter Optimization", Some(*threshold), Some(*min_dur));
    }

    // Stage 3: Segment assembly (if produced)
    if let Some(durations) = stage3_durations {
        add_if_unique(durations, "Segment Assembly", None, None);
    }

    // Stage 4: Quiet spot detection (if produced)
    if let Some(durations) = stage4_durations {
        add_if_unique(durations, "Quiet Spot Detection", None, None);
    }

    segmentations
}
```

**Deduplication Strategy:**
- Signature = hash of track durations rounded to 0.1s precision
- Prevents testing identical segmentations multiple times
- Preserves source stage attribution for reporting

**Expected Segmentation Counts:**
- Minimum: 1 (Stage 1 only, if perfect match)
- Typical: 5-20 (Stage 2 variations + potential Stage 3/4)
- Maximum: ~180 (all Stage 2 combinations if all unique)

---

### Phase 2: Additional Candidate Retrieval

**Input:**
- `artist: &str`
- `album: &str`
- `candidate_tracker: &MBCandidateTracker`
- `rate_limiter: &RateLimiter`

**Output:** `Vec<(Vec<u32>, String)>` (additional candidates with durations and MBIDs)

**Algorithm:**

```rust
async fn get_additional_mb_candidates(
    artist: &str,
    album: &str,
    candidate_tracker: &MBCandidateTracker,
    rate_limiter: &RateLimiter,
    max_additional: usize,  // Default: 50
) -> Result<Vec<(Vec<u32>, String)>, Box<dyn std::error::Error>> {

    // Calculate offset (skip already-tested candidates)
    let offset = candidate_tracker.tested_mbids.len();

    // Determine how many untested candidates remain
    let untested_count = candidate_tracker.initial_response.releases
        .iter()
        .skip(offset)
        .filter(|r| !candidate_tracker.tested_mbids.contains(&r.id))
        .count();

    let fetch_count = untested_count.min(max_additional);

    if fetch_count == 0 {
        return Ok(Vec::new());
    }

    println!("  STAGE 5: Fetching {} additional MusicBrainz candidates (offset {})...",
             fetch_count, offset);

    // Get untested release IDs
    let untested_releases: Vec<&MBRelease> = candidate_tracker.initial_response.releases
        .iter()
        .skip(offset)
        .filter(|r| !candidate_tracker.tested_mbids.contains(&r.id))
        .take(fetch_count)
        .collect();

    let mut additional_candidates = Vec::new();

    // Fetch details for each release (with rate limiting and retry)
    for (idx, release) in untested_releases.iter().enumerate() {
        rate_limiter.wait().await;

        println!("    Fetching release {}/{}: {}",
                 idx + 1, fetch_count, release.id);

        // Use existing retry_with_exponential_backoff logic
        match fetch_release_details_with_retry(&release.id, rate_limiter).await {
            Ok(durations) => {
                additional_candidates.push((durations, release.id.clone()));
            }
            Err(e) => {
                println!("    WARNING: Failed to fetch release {}: {}", release.id, e);
                // Continue with other releases
            }
        }
    }

    println!("  Successfully fetched {} additional candidates", additional_candidates.len());

    Ok(additional_candidates)
}
```

**Rate Limiting:**
- Uses existing 2-second rate limit (0.5 req/sec)
- Exponential backoff retry on network failures

**Error Handling:**
- Individual release fetch failures logged but don't abort Stage 5
- Continues testing with successfully fetched candidates

---

### Phase 3: Cartesian Product Testing

**Input:**
- `segmentations: &[SegmentationRecord]`
- `additional_candidates: &[(Vec<u32>, String)]`  // (durations, mbid)
- `match_tolerance_secs: f64`

**Output:** `Stage5MatchResult`

**Algorithm:**

```rust
fn test_all_combinations(
    segmentations: &[SegmentationRecord],
    additional_candidates: &[(Vec<u32>, String)],
    current_best_percentage: f64,
    match_tolerance_secs: f64,
) -> Stage5MatchResult {

    let mut best_percentage = current_best_percentage;
    let mut best_segmentation = None;
    let mut best_mbid = String::new();
    let mut best_expected_durations = Vec::new();
    let mut best_matches = Vec::new();
    let mut best_mb_track_count = 0;

    let total_combinations = segmentations.len() * additional_candidates.len();
    let mut tested = 0;

    println!("  STAGE 5: Testing {} segmentations × {} candidates = {} combinations",
             segmentations.len(), additional_candidates.len(), total_combinations);

    // Iterate through cartesian product
    for segmentation in segmentations {
        for (mb_durations, mbid) in additional_candidates {
            tested += 1;

            // Test this (segmentation, MB candidate) pair
            let (matches, matched_count, percentage) =
                analyze_track_matching(&segmentation.durations, mb_durations, match_tolerance_secs);

            // Update best if improved
            if percentage > best_percentage {
                best_percentage = percentage;
                best_segmentation = Some(segmentation.clone());
                best_mbid = mbid.clone();
                best_expected_durations = mb_durations.clone();
                best_matches = matches;
                best_mb_track_count = mb_durations.len();

                println!("    New best: {:.1}% (segmentation: {}, MB candidate: {}, tested {}/{})",
                         best_percentage,
                         segmentation.source_stage,
                         mbid,
                         tested,
                         total_combinations);

                // Early exit if perfect match found
                if best_percentage >= 100.0 {
                    println!("    Perfect match found! Stopping Stage 5 early.");
                    break;
                }
            }
        }

        if best_percentage >= 100.0 {
            break;
        }
    }

    Stage5MatchResult {
        best_percentage,
        best_segmentation: best_segmentation.unwrap_or_else(|| segmentations[0].clone()),
        best_mbid,
        best_mb_track_count,
        best_expected_durations,
        best_matches,
        total_segmentations_tested: segmentations.len(),
        total_mb_candidates_tested: additional_candidates.len(),
        total_combinations_tested: tested,
    }
}
```

**Performance Characteristics:**
- **Time Complexity:** O(S × C) where S = segmentations, C = candidates
- **Expected:** ~10 segmentations × 50 candidates = 500 combinations
- **Maximum:** ~180 segmentations × 50 candidates = 9,000 combinations
- **Per-combination cost:** O(min(detected_tracks, expected_tracks)) - cheap!

**Early Exit Optimization:**
- Stops immediately when 100% match found
- Prevents unnecessary testing once perfect match achieved

---

### Phase 4: Integration into Main Flow

**Placement:** After Stage 4, before final result reporting

**Pseudocode:**

```rust
// After Stage 4 completes...

// === STAGE 5: Expanded MusicBrainz Search (if needed) ===
if best_percentage < 100.0 && candidate_tracker.has_more_candidates() {
    println!("  STAGE 5: Attempting expanded MusicBrainz search...");

    // Phase 1: Collect all segmentations from Stages 1-4
    let all_segmentations = collect_all_segmentations(
        &stage1_durations,
        &stage2_all_results,  // NEW: need to save ALL Stage 2 results, not just best
        &stage3_durations,
        &stage4_durations,
    );

    println!("    Collected {} unique segmentations", all_segmentations.len());

    // Phase 2: Fetch additional MusicBrainz candidates
    let additional_candidates = match get_additional_mb_candidates(
        &artist, &album, &candidate_tracker, &rate_limiter, 50
    ).await {
        Ok(candidates) => candidates,
        Err(e) => {
            println!("    Failed to fetch additional candidates: {}", e);
            Vec::new()  // Continue without Stage 5
        }
    };

    if !additional_candidates.is_empty() {
        // Phase 3: Test all combinations
        let stage5_result = test_all_combinations(
            &all_segmentations,
            &additional_candidates,
            best_percentage,
            match_tolerance_secs,
        );

        // Phase 4: Update best result if improved
        if stage5_result.best_percentage > best_percentage {
            println!("    Stage 5 improved match to {:.1}%", stage5_result.best_percentage);

            best_durations = stage5_result.best_segmentation.durations;
            best_matches = stage5_result.best_matches;
            best_matched_count = stage5_result.best_matches.iter().filter(|m| m.is_match).count();
            best_percentage = stage5_result.best_percentage;
            best_stage = "Expanded MusicBrainz Search";

            // Update MBID and expected durations
            mbid = stage5_result.best_mbid;
            expected_durations = stage5_result.best_expected_durations;
        } else {
            println!("    Stage 5: No improvement found ({:.1}% vs {:.1}%)",
                     stage5_result.best_percentage, best_percentage);
        }
    }
}
```

---

## Implementation Modifications Required

### 1. Save All Stage 2 Results

**Current Code (Stage 2):**
```rust
// Only saves best result
let mut best_durations = stage1_durations.clone();
let mut best_percentage = stage1_percentage;
```

**Modified Code:**
```rust
// Save ALL results for Stage 5
let mut stage2_all_results = Vec::new();  // NEW
let mut best_durations = stage1_durations.clone();
let mut best_percentage = stage1_percentage;

for &thresh in &threshold_values {
    for &min_dur in &min_duration_values {
        let test_durations = get_track_durations(&samples, sample_rate, thresh, min_dur);
        let (test_matches, test_matched_count, test_percentage) =
            analyze_track_matching(&test_durations, &expected_durations, match_tolerance_secs);

        // NEW: Save all results
        stage2_all_results.push((test_durations.clone(), thresh, min_dur));

        if test_percentage > best_percentage {
            best_durations = test_durations;
            best_percentage = test_percentage;
            // ...
        }
    }
}
```

### 2. Track MusicBrainz Candidates

**Add tracking after initial MB fetch:**

```rust
// After get_expected_durations() in Stage 1
let mut candidate_tracker = MBCandidateTracker {
    tested_mbids: HashSet::new(),
    total_available: search_response.releases.len(),
    initial_response: search_response.clone(),
};

// Record which release was used
candidate_tracker.tested_mbids.insert(mbid.clone());
```

### 3. Update Header

```rust
println!("=== Comprehensive Album Matcher (Run 5) ===");
println!("6-Phase Matching: ID3 Reconciliation -> Initial -> Parameter Opt -> Segment Assembly -> Quiet Spots -> Expanded MB Search\n");
```

---

## Output Format

### Console Output Example

```
  STAGE 5: Attempting expanded MusicBrainz search...
    Collected 12 unique segmentations
      - Initial: 1 segmentation
      - Parameter Optimization: 9 segmentations
      - Segment Assembly: 1 segmentation
      - Quiet Spot Detection: 1 segmentation
  STAGE 5: Fetching 50 additional MusicBrainz candidates (offset 30)...
    Fetching release 1/50: a1b2c3d4-e5f6-7890-abcd-ef1234567890
    Fetching release 2/50: b2c3d4e5-f6a7-8901-bcde-f12345678901
    ...
  Successfully fetched 48 additional candidates (2 failures)
  STAGE 5: Testing 12 segmentations × 48 candidates = 576 combinations
    New best: 73.3% (segmentation: Parameter Optimization, MB candidate: a1b2c3d4..., tested 145/576)
    New best: 86.7% (segmentation: Segment Assembly, MB candidate: c3d4e5f6..., tested 302/576)
    New best: 100.0% (segmentation: Parameter Optimization, MB candidate: d4e5f6a7..., tested 418/576)
    Perfect match found! Stopping Stage 5 early.
  Stage 5 improved match to 100.0%

  FINAL RESULT:
    Matching stage: Expanded MusicBrainz Search
    Segmentation source: Parameter Optimization (-52dB, 0.5s)
    MusicBrainz release: d4e5f6a7-8901-2345-6789-0abcdef12345
    Track count: 12/12 ✓
    Matched tracks: 12/12 (100.0%)
    Mean error: 3.21s
    Confidence: Excellent
```

---

## Error Handling

### Network Failures
- Individual release fetch failures don't abort Stage 5
- Continue with successfully fetched candidates
- Report failure count in summary

### Empty Candidate Pool
- If no additional candidates available: skip Stage 5 gracefully
- If all fetches fail: report warning, continue with Stages 1-4 result

### Memory Constraints
- Limit segmentation pool size if needed (keep best N by diversity)
- Limit MB candidates to 50 (configurable)

---

## Performance Considerations

### Time Complexity

**Stage 5 Runtime:**
- MB fetch: 50 candidates × 2 sec/request = **100 seconds** (1.7 minutes)
- Testing: 12 segmentations × 50 candidates × 0.001 sec/test = **0.6 seconds**
- **Total: ~2 minutes per album** (when Stage 5 triggers)

**Impact on Total Runtime:**
- Albums achieving 100% before Stage 5: 0 additional time
- Albums requiring Stage 5: +2 minutes each
- Expected: ~30% of albums reach Stage 5 → +12 minutes total for 21-album run

### Memory Usage

**Segmentation Storage:**
- 180 max segmentations × 20 tracks × 8 bytes = **28.8 KB** (negligible)

**MB Candidate Storage:**
- 50 candidates × 20 tracks × 4 bytes = **4 KB** (negligible)

**Total Stage 5 Memory Overhead:** < 50 KB (insignificant)

---

## Testing Strategy

### Unit Tests

1. **Segmentation Deduplication:**
   - Verify identical segmentations produce same signature
   - Verify signature changes with 0.1s duration difference

2. **Candidate Filtering:**
   - Verify tested MBIDs excluded from additional fetch
   - Verify offset calculation correct

3. **Cartesian Product:**
   - Verify all combinations tested
   - Verify early exit on 100% match
   - Verify best result correctly identified

### Integration Tests

1. **Stage 5 Trigger:**
   - Verify skips when best_percentage = 100%
   - Verify skips when no additional candidates
   - Verify executes when conditions met

2. **End-to-End:**
   - Run on known album with multiple editions
   - Verify correct edition selected
   - Verify segmentation/candidate pair reported

---

## Success Metrics

### Run 5 Expected Improvements

**Albums Expected to Benefit from Stage 5:**
- Albums with multiple editions (deluxe, remaster, international)
- Albums where initial MB match incorrect
- Albums with unusual track counts (bonus tracks, hidden tracks)

**Quantitative Goals:**
- **Perfect match rate:** 60% → 70% (Run 2: 45.5% perfect)
- **Excellent confidence:** 82% → 90% (Run 2: 81.8% excellent)
- **Fair or worse:** 18% → 5% (reduce by testing more MB candidates)

**Validation:**
- Billy Thorpe album: May find correct edition if over-segmentation issue is MB mismatch
- Journey album: May find better-matching edition (16 tracks suggests deluxe version)

---

## Future Enhancements

### Adaptive Candidate Fetching
- Fetch candidates in batches of 10
- Stop early if 5 consecutive batches show no improvement
- Reduces unnecessary MB queries for hopeless cases

### Segmentation Pruning
- Keep only "diverse" segmentations (different track counts)
- Skip near-duplicates (e.g., 15 tracks vs 16 tracks with similar durations)
- Reduces test matrix size while maintaining coverage

### Parallel Testing
- Test multiple (segmentation, candidate) pairs concurrently
- Reduces wall-clock time for large matrices
- Respects MB rate limits for fetching

---

## Appendix: Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                      STAGE 1-4 EXECUTION                    │
│  - Collect segmentations: stage1_durations, stage2_results, │
│    stage3_durations, stage4_durations                       │
│  - Track MB candidates tested: tested_mbids                 │
│  - Current best: best_percentage, best_durations            │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ├─ best_percentage < 100%? ──No──> Skip Stage 5
                      │
                     Yes
                      │
                      ├─ has_more_candidates()? ──No──> Skip Stage 5
                      │
                     Yes
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                  STAGE 5: PHASE 1                           │
│              Collect All Segmentations                      │
│                                                             │
│  Input:  stage1_durations, stage2_results,                 │
│          stage3_durations, stage4_durations                │
│  Output: Vec<SegmentationRecord> (deduplicated)            │
│  Count:  Typically 5-20 unique segmentations               │
└─────────────────────┬───────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                  STAGE 5: PHASE 2                           │
│          Fetch Additional MB Candidates                     │
│                                                             │
│  Input:  candidate_tracker, max_additional=50              │
│  Output: Vec<(Vec<u32>, String)> - (durations, mbid)       │
│  Count:  Up to 50 additional candidates                    │
│  Time:   ~2 seconds per candidate (rate limiting)          │
└─────────────────────┬───────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                  STAGE 5: PHASE 3                           │
│            Cartesian Product Testing                        │
│                                                             │
│  For each segmentation:                                     │
│    For each MB candidate:                                   │
│      Test match → Update best if improved                  │
│                                                             │
│  Matrix: segmentations × candidates                        │
│  Early exit: Stop at 100% match                            │
└─────────────────────┬───────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│                  STAGE 5: PHASE 4                           │
│              Update Best Result                             │
│                                                             │
│  If stage5_result.best_percentage > current_best:          │
│    - Update best_durations                                 │
│    - Update best_matches                                   │
│    - Update best_stage = "Expanded MusicBrainz Search"     │
│    - Update mbid, expected_durations                       │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
              Final Result Output
```

---

## MusicBrainz Link Reporting

**New Requirement:** Include MusicBrainz release links in all output for verification and context.

**URL Format:**
```
https://musicbrainz.org/release/{mbid}
```

**Implementation Locations:**

### 1. Console Output (FINAL RESULT Section)

```rust
println!("\n  FINAL RESULT:");
println!("    Matching stage: {}", best_stage);
println!("    Track count: {}/{} {}",
         best_durations.len(), expected_durations.len(),
         if best_durations.len() == expected_durations.len() { "✓" } else { "✗" });
println!("    Matched tracks: {}/{} ({:.1}%)",
         best_matched_count, expected_durations.len(), best_percentage);
println!("    Mean error: {:.2}s", mean_error);
println!("    Confidence: {}", confidence_level);
println!("    MusicBrainz: https://musicbrainz.org/release/{}", best_mbid);  // NEW
if best_stage != "Initial" {
    println!("    Best parameters: {:.0}dB, {}s", best_threshold, best_min_duration);
}
```

### 2. JSON Results File

Add `musicbrainz_url` field to each album result:

```rust
serde_json::json!({
    "file_path": file_path,
    "artist": artist_name,
    "album": album_name,
    "matching_stage": best_stage,
    "confidence": confidence_level,
    "match_percentage": best_percentage,
    "mean_error": mean_error,
    "track_count": format!("{}/{}", best_durations.len(), expected_durations.len()),
    "musicbrainz_mbid": best_mbid,  // Existing field
    "musicbrainz_url": format!("https://musicbrainz.org/release/{}", best_mbid),  // NEW
    // ... other fields
})
```

### 3. Comprehensive Analysis Output

Include links in Best/Worst performers lists:

```rust
println!("\nBest 5 Albums:");
for (i, result) in best_albums.iter().enumerate() {
    println!("  {}. {} - {} ({:.1}%, {} via {})",
             i + 1,
             result.artist,
             result.album,
             result.match_percentage,
             result.confidence,
             result.matching_stage);
    println!("     MusicBrainz: https://musicbrainz.org/release/{}", result.mbid);  // NEW
}
```

**Rationale:**
- Allows manual verification of matches
- Provides context about which release edition was matched (standard/deluxe/remaster/international)
- Enables investigation of mismatches (wrong edition, wrong album entirely)
- Facilitates debugging when match quality is unexpectedly low

---

## Implementation Checklist

### Phase 0: ID3 Tag Extraction (NEW - Run 5)
- [ ] Add `ID3Metadata` struct
- [ ] Add `ReconciledMetadata` struct
- [ ] Add `ReconciliationStrategy` enum
- [ ] Add `MetadataConfidence` enum
- [ ] Add `MetadataSource` enum
- [ ] Implement `extract_id3_tags()` function (uses ffprobe)
- [ ] Implement `reconcile_metadata()` function
- [ ] Implement `strings_match()` helper (normalized comparison)
- [ ] Implement `choose_artist()` heuristic
- [ ] Implement `choose_album()` heuristic
- [ ] Implement `log_reconciliation_decision()` output
- [ ] Implement `extract_and_reconcile_metadata()` main function
- [ ] Update main flow to use Phase 0 before Stage 1
- [ ] Test reconciliation with sample files (direct match, partial match, conflict, gap fill)
- [ ] Verify track count extraction from ID3 comment field

### Stage 5: Expanded MusicBrainz Search
- [ ] Add `SegmentationRecord` struct
- [ ] Add `MBCandidateTracker` struct
- [ ] Add `Stage5MatchResult` struct
- [ ] Implement `collect_all_segmentations()` function
- [ ] Implement `get_additional_mb_candidates()` function
- [ ] Implement `test_all_combinations()` function
- [ ] Modify Stage 2 to save all results
- [ ] Add candidate tracking in Stage 1
- [ ] Add Stage 5 integration in main flow
- [ ] Update console output header (6-phase pipeline)
- [ ] Add Stage 5 reporting in final output
- [ ] **Add MusicBrainz links to console output**
- [ ] **Add `musicbrainz_url` field to JSON results**
- [ ] **Add MusicBrainz links to Best/Worst performers output**
- [ ] Test deduplication logic
- [ ] Test early exit optimization

### Integration & Testing
- [ ] Implement alternate search fallback (if < 80% and alternates exist)
- [ ] Test Phase 0 + Stage 5 integration
- [ ] Run on training set
- [ ] Analyze reconciliation decisions (Phase 0 log output)
- [ ] Measure impact of ID3 reconciliation on match accuracy
- [ ] Analyze results vs Run 4
- [ ] Document improvements in run5_report.md

---

**End of Design Document**
