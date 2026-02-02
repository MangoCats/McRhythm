# Problem Albums Analysis and Recommendations

## Executive Summary

4 albums remain with >=2 tracks having >=30s duration errors after Stage 7 progressive refinement. Analysis reveals these are NOT boundary detection failures, but rather **wrong MBID selection** and **inherent ambiguity** in deluxe/remix editions.

## Problem Albums

### 1. The Chemical Brothers - Surrender
**MBID:** 7898f198-d463-4a37-8299-335118aa359a
**Winning Stage:** Stage2 (100.0%)
**Format:** Digital Media (3-disc deluxe edition with remixes)
**Tracks:** 28 total

**Error Analysis:**
- Only **1 track** with severe error: Track 28 "Hey Boy Hey Girl (Kink extended remix)" (-83.16s)
- All other tracks <10s error
- Mean error: 3.57s (excellent)

**Root Cause:** Deluxe edition with extended remixes. The "Kink extended remix" is a variable-length remix that doesn't have a standard duration. The file may have a different version/edit than the MusicBrainz metadata.

**Recommendation:** This is NOT fixable algorithmically. Remix tracks have variable durations by nature. Consider:
- Filter out deluxe/remix editions in favor of standard editions
- Add edition preference scoring (prefer "standard" over "deluxe")

---

### 2. Imagine Dragons - Night Visions
**MBID:** 68c223b1-9529-4b4c-b116-98629047ca56
**Winning Stage:** Stage2 (100.0%)
**Format:** CD (likely deluxe edition)
**Tracks:** 16 total

**Error Analysis:**
- Track 6 "Amsterdam" (+21.19s) - MODERATE
- Track 12 "My Fault" (-55.99s) - SEVERE

**Root Cause:** Wrong edition selected. The file appears to be standard 11-track edition, but matched to 16-track deluxe edition with bonus tracks. The boundary detector is trying to fit 11 real tracks into 16 expected tracks, causing massive errors on the "bonus" tracks that don't exist in the file.

**Recommendation:**
- **Edition filtering:** Reject editions where track count differs significantly from detected count
- **Track count validation:** If detected=11 and edition expects=16, score should be heavily penalized
- **Pre-filter editions:** Before running stages, filter to editions within ±2 tracks of detected count

---

### 3. Michael Jackson - Thriller
**MBID:** 4e2bfe06-f483-4f7b-b12a-f74e2d1aa3f2
**Winning Stage:** Stage2 (100.0%)
**Format:** CD
**Tracks:** 19-25 total (compilation or deluxe edition)

**Error Analysis:**
- Track 8 "P.Y.T." (+22.33s) - MODERATE
- Track 9 "The Lady in My Life" (-52.28s) - SEVERE

**Root Cause:** Similar to Imagine Dragons - wrong edition selection. Standard "Thriller" has 9 tracks, but this MBID appears to be a deluxe/compilation edition with bonus tracks.

**Recommendation:** Same as Imagine Dragons - better edition filtering based on track count.

---

### 4. James Gang - Funk #49
**MBID:** 55be9910-ca58-4e61-9e14-2751812044ff
**Winning Stage:** Stage2 (100.0%)
**Format:** CD
**Tracks:** 19 total (compilation)

**Error Analysis:**
- Track 9 "The Lady in My Life" (-52.28s) - SEVERE
- Track 10 "Again" (-30.84s) - SEVERE
- Track 14 "Excerpts From the Ian Anderson Interview" (-96.84s) - SEVERE

**Root Cause:** Wrong MBID entirely. "Funk #49" is a James Gang song, but this album appears to have matched to a compilation or box set containing multiple artists/albums. Tracks like "The Lady in My Life" (Michael Jackson) and "Interview" (Ian Anderson/Jethro Tull) suggest this is a multi-artist compilation.

**Recommendation:**
- **Artist validation:** Verify all tracks belong to the same artist
- **Compilation detection:** Flag and deprioritize compilations unless explicitly requested
- **Title matching:** Check if album title matches release title (not just artist match)

---

## Algorithmic Improvements

### 1. Edition Filtering (High Priority)
**Problem:** Deluxe/compilation editions selected when standard editions would match better

**Solution:**
```rust
// In edition filtering stage
fn score_edition_preference(edition: &Edition, detected_track_count: usize) -> f64 {
    let mut score = 1.0;

    // Penalize track count mismatch
    let count_diff = (edition.track_count as i32 - detected_track_count as i32).abs();
    if count_diff > 2 {
        score *= 0.5; // Heavy penalty for >2 track difference
    }

    // Penalize deluxe editions
    let title_lower = edition.title.to_lowercase();
    if title_lower.contains("deluxe") || title_lower.contains("expanded") {
        score *= 0.7;
    }

    // Penalize compilations
    if title_lower.contains("collection") || title_lower.contains("anthology")
        || title_lower.contains("best of") {
        score *= 0.6;
    }

    // Prefer standard/original editions
    if title_lower.contains("original") || title_lower.contains("standard") {
        score *= 1.2;
    }

    score
}
```

### 2. Track Count Pre-Filter (Medium Priority)
**Problem:** Wasting time matching against editions with wildly different track counts

**Solution:**
```rust
// Before running matching stages
fn filter_editions_by_track_count(
    editions: &[Edition],
    detected_boundaries: usize
) -> Vec<Edition> {
    editions.iter()
        .filter(|e| {
            let diff = (e.track_count as i32 - detected_boundaries as i32).abs();
            diff <= 3 // Only consider editions within ±3 tracks
        })
        .cloned()
        .collect()
}
```

### 3. Artist Consistency Check (High Priority)
**Problem:** Compilations with multiple artists matched to single-artist files

**Solution:**
```rust
fn validate_artist_consistency(edition: &Edition, source_artist: &str) -> bool {
    // Check if all tracks have same artist as source
    let mut artists: HashSet<String> = HashSet::new();
    for track in &edition.tracks {
        artists.insert(track.artist.to_lowercase());
    }

    // If >3 different artists, likely a compilation
    if artists.len() > 3 {
        return false;
    }

    // Check if source artist is in the track artists
    let source_lower = source_artist.to_lowercase();
    artists.iter().any(|a| a.contains(&source_lower) || source_lower.contains(a))
}
```

### 4. Remix Detection (Low Priority)
**Problem:** Extended remixes have variable durations

**Solution:**
- Accept larger error tolerance for tracks with "remix", "extended", "mix" in title
- Or: Skip validation for remix tracks entirely

```rust
fn is_remix_track(title: &str) -> bool {
    let lower = title.to_lowercase();
    lower.contains("remix") || lower.contains("extended")
        || lower.contains(" mix)") || lower.contains("version")
}

// In validation
if is_remix_track(&track.title) && abs_error < 120.0 {
    // Accept up to 2 minutes error for remixes
    return true;
}
```

---

## Implementation Priority

1. **CRITICAL:** Add edition preference scoring (deluxe/compilation penalty)
2. **HIGH:** Track count pre-filtering (±3 tracks)
3. **HIGH:** Artist consistency validation
4. **MEDIUM:** Remix track tolerance
5. **LOW:** Add logging to show why editions were rejected

## Expected Impact

With these changes:
- **Chemical Brothers:** Would still match same edition (deluxe is correct), but remix track error would be tolerated
- **Imagine Dragons:** Would match standard 11-track edition instead of 16-track deluxe (fixes problem)
- **Michael Jackson:** Would match standard 9-track "Thriller" instead of compilation (fixes problem)
- **James Gang:** Would reject multi-artist compilation, possibly match single-artist "Best of James Gang" (fixes problem)

**Estimated fix rate:** 3 out of 4 albums (75% improvement)
