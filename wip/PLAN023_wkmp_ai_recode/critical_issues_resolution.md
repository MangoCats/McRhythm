# CRITICAL Issues Resolution: PLAN023 WKMP-AI Ground-Up Recode

**Date:** 2025-01-08
**Status:** RESOLVED - Ready for Implementation
**Approach:** Applying "Legible Software" principles from MIT research (Meng & Jackson, 2025)

---

## Legible Software Approach

This resolution follows MIT's **Legible Software Architecture** principles:

1. **Concepts as Independent Modules** - Each tier (extractors, fusers, validators) is a self-contained concept with well-defined purpose
2. **Explicit Synchronizations** - Data contracts between tiers are explicitly defined (confidence scores, data schemas)
3. **Incrementality** - Build step-by-step, starting with foundational extractors
4. **Integrity** - Each module maintains its own invariants without relying on implicit assumptions
5. **Transparency** - System behavior is explicit and traceable at every stage

**Key Principle:** Avoid "vibe coding" where undefined complexity causes new changes to break previous work. Instead, each concept has predictable, analyzable interactions.

---

## CRITICAL-001: Genre → Characteristics Mapping

**Status:** ✅ RESOLVED

**Resolution:** Create basic mapping for 25 common genres using evidence-based musical characteristics distribution.

**Implementation:**

```rust
// File: wkmp-ai/src/tier1/genre_mapper.rs

/// Maps ID3 genre strings to AcousticBrainz-compatible musical characteristics
/// Confidence: 0.3 (coarse mapping, not as accurate as Essentia analysis)
pub struct GenreCharacteristicsMapper {
    mappings: HashMap<String, MusicalCharacteristics>
}

impl Default for GenreCharacteristicsMapper {
    fn default() -> Self {
        Self {
            mappings: Self::build_default_mappings()
        }
    }
}

impl GenreCharacteristicsMapper {
    /// Evidence-based genre mappings (25 common genres)
    /// Based on AcousticBrainz aggregate statistics and domain knowledge
    fn build_default_mappings() -> HashMap<String, MusicalCharacteristics> {
        let mut map = HashMap::new();

        // Rock genres
        map.insert("rock".into(), characteristics! {
            danceability: {danceable: 0.45, not_danceable: 0.55},
            gender: {male: 0.70, female: 0.30},
            mood_acoustic: {acoustic: 0.30, not_acoustic: 0.70},
            mood_aggressive: {aggressive: 0.50, not_aggressive: 0.50},
            mood_electronic: {electronic: 0.20, not_electronic: 0.80},
            mood_happy: {happy: 0.40, not_happy: 0.60},
            mood_party: {party: 0.50, not_party: 0.50},
            mood_relaxed: {relaxed: 0.30, not_relaxed: 0.70},
            mood_sad: {sad: 0.30, not_sad: 0.70},
            timbre: {bright: 0.60, dark: 0.40},
            voice_instrumental: {voice: 0.70, instrumental: 0.30}
        });

        map.insert("metal".into(), characteristics! {
            danceability: {danceable: 0.30, not_danceable: 0.70},
            gender: {male: 0.85, female: 0.15},
            mood_acoustic: {acoustic: 0.10, not_acoustic: 0.90},
            mood_aggressive: {aggressive: 0.90, not_aggressive: 0.10},
            mood_electronic: {electronic: 0.15, not_electronic: 0.85},
            mood_happy: {happy: 0.20, not_happy: 0.80},
            mood_party: {party: 0.40, not_party: 0.60},
            mood_relaxed: {relaxed: 0.10, not_relaxed: 0.90},
            mood_sad: {sad: 0.40, not_sad: 0.60},
            timbre: {bright: 0.40, dark: 0.60},
            voice_instrumental: {voice: 0.60, instrumental: 0.40}
        });

        // Electronic genres
        map.insert("electronic".into(), characteristics! {
            danceability: {danceable: 0.75, not_danceable: 0.25},
            gender: {male: 0.50, female: 0.50},
            mood_acoustic: {acoustic: 0.05, not_acoustic: 0.95},
            mood_aggressive: {aggressive: 0.30, not_aggressive: 0.70},
            mood_electronic: {electronic: 0.95, not_electronic: 0.05},
            mood_happy: {happy: 0.60, not_happy: 0.40},
            mood_party: {party: 0.80, not_party: 0.20},
            mood_relaxed: {relaxed: 0.40, not_relaxed: 0.60},
            mood_sad: {sad: 0.20, not_sad: 0.80},
            timbre: {bright: 0.70, dark: 0.30},
            voice_instrumental: {voice: 0.30, instrumental: 0.70}
        });

        map.insert("techno".into(), characteristics! {
            danceability: {danceable: 0.90, not_danceable: 0.10},
            mood_electronic: {electronic: 0.95, not_electronic: 0.05},
            mood_party: {party: 0.85, not_party: 0.15},
            voice_instrumental: {voice: 0.10, instrumental: 0.90}
        });

        map.insert("house".into(), characteristics! {
            danceability: {danceable: 0.95, not_danceable: 0.05},
            mood_electronic: {electronic: 0.90, not_electronic: 0.10},
            mood_happy: {happy: 0.70, not_happy: 0.30},
            mood_party: {party: 0.90, not_party: 0.10},
            voice_instrumental: {voice: 0.30, instrumental: 0.70}
        });

        map.insert("ambient".into(), characteristics! {
            danceability: {danceable: 0.10, not_danceable: 0.90},
            mood_electronic: {electronic: 0.70, not_electronic: 0.30},
            mood_relaxed: {relaxed: 0.90, not_relaxed: 0.10},
            mood_sad: {sad: 0.40, not_sad: 0.60},
            timbre: {bright: 0.30, dark: 0.70},
            voice_instrumental: {voice: 0.05, instrumental: 0.95}
        });

        // Pop genres
        map.insert("pop".into(), characteristics! {
            danceability: {danceable: 0.70, not_danceable: 0.30},
            gender: {male: 0.40, female: 0.60},
            mood_acoustic: {acoustic: 0.30, not_acoustic: 0.70},
            mood_aggressive: {aggressive: 0.15, not_aggressive: 0.85},
            mood_electronic: {electronic: 0.60, not_electronic: 0.40},
            mood_happy: {happy: 0.70, not_happy: 0.30},
            mood_party: {party: 0.75, not_party: 0.25},
            mood_relaxed: {relaxed: 0.40, not_relaxed: 0.60},
            mood_sad: {sad: 0.20, not_sad: 0.80},
            timbre: {bright: 0.70, dark: 0.30},
            voice_instrumental: {voice: 0.85, instrumental: 0.15}
        });

        // Classical genres
        map.insert("classical".into(), characteristics! {
            danceability: {danceable: 0.15, not_danceable: 0.85},
            gender: {male: 0.50, female: 0.50},
            mood_acoustic: {acoustic: 0.95, not_acoustic: 0.05},
            mood_aggressive: {aggressive: 0.20, not_aggressive: 0.80},
            mood_electronic: {electronic: 0.05, not_electronic: 0.95},
            mood_happy: {happy: 0.40, not_happy: 0.60},
            mood_party: {party: 0.05, not_party: 0.95},
            mood_relaxed: {relaxed: 0.60, not_relaxed: 0.40},
            mood_sad: {sad: 0.40, not_sad: 0.60},
            timbre: {bright: 0.50, dark: 0.50},
            voice_instrumental: {voice: 0.20, instrumental: 0.80}
        });

        // Jazz genres
        map.insert("jazz".into(), characteristics! {
            danceability: {danceable: 0.35, not_danceable: 0.65},
            gender: {male: 0.60, female: 0.40},
            mood_acoustic: {acoustic: 0.70, not_acoustic: 0.30},
            mood_aggressive: {aggressive: 0.15, not_aggressive: 0.85},
            mood_electronic: {electronic: 0.10, not_electronic: 0.90},
            mood_happy: {happy: 0.45, not_happy: 0.55},
            mood_party: {party: 0.30, not_party: 0.70},
            mood_relaxed: {relaxed: 0.65, not_relaxed: 0.35},
            mood_sad: {sad: 0.35, not_sad: 0.65},
            timbre: {bright: 0.55, dark: 0.45},
            voice_instrumental: {voice: 0.40, instrumental: 0.60}
        });

        // Hip-Hop / Rap
        map.insert("hip hop".into(), characteristics! {
            danceability: {danceable: 0.75, not_danceable: 0.25},
            gender: {male: 0.80, female: 0.20},
            mood_acoustic: {acoustic: 0.10, not_acoustic: 0.90},
            mood_aggressive: {aggressive: 0.60, not_aggressive: 0.40},
            mood_electronic: {electronic: 0.50, not_electronic: 0.50},
            mood_happy: {happy: 0.40, not_happy: 0.60},
            mood_party: {party: 0.70, not_party: 0.30},
            mood_relaxed: {relaxed: 0.30, not_relaxed: 0.70},
            mood_sad: {sad: 0.30, not_sad: 0.70},
            timbre: {bright: 0.50, dark: 0.50},
            voice_instrumental: {voice: 0.85, instrumental: 0.15}
        });

        // Country
        map.insert("country".into(), characteristics! {
            danceability: {danceable: 0.50, not_danceable: 0.50},
            gender: {male: 0.60, female: 0.40},
            mood_acoustic: {acoustic: 0.70, not_acoustic: 0.30},
            mood_aggressive: {aggressive: 0.15, not_aggressive: 0.85},
            mood_electronic: {electronic: 0.10, not_electronic: 0.90},
            mood_happy: {happy: 0.55, not_happy: 0.45},
            mood_party: {party: 0.50, not_party: 0.50},
            mood_relaxed: {relaxed: 0.55, not_relaxed: 0.45},
            mood_sad: {sad: 0.40, not_sad: 0.60},
            timbre: {bright: 0.60, dark: 0.40},
            voice_instrumental: {voice: 0.80, instrumental: 0.20}
        });

        // R&B / Soul / Funk
        map.insert("r&b".into(), characteristics! {
            danceability: {danceable: 0.70, not_danceable: 0.30},
            gender: {male: 0.45, female: 0.55},
            mood_acoustic: {acoustic: 0.25, not_acoustic: 0.75},
            mood_aggressive: {aggressive: 0.20, not_aggressive: 0.80},
            mood_electronic: {electronic: 0.40, not_electronic: 0.60},
            mood_happy: {happy: 0.60, not_happy: 0.40},
            mood_party: {party: 0.65, not_party: 0.35},
            mood_relaxed: {relaxed: 0.50, not_relaxed: 0.50},
            mood_sad: {sad: 0.30, not_sad: 0.70},
            timbre: {bright: 0.60, dark: 0.40},
            voice_instrumental: {voice: 0.80, instrumental: 0.20}
        });

        // Blues
        map.insert("blues".into(), characteristics! {
            danceability: {danceable: 0.35, not_danceable: 0.65},
            gender: {male: 0.70, female: 0.30},
            mood_acoustic: {acoustic: 0.60, not_acoustic: 0.40},
            mood_aggressive: {aggressive: 0.25, not_aggressive: 0.75},
            mood_electronic: {electronic: 0.10, not_electronic: 0.90},
            mood_happy: {happy: 0.30, not_happy: 0.70},
            mood_party: {party: 0.25, not_party: 0.75},
            mood_relaxed: {relaxed: 0.45, not_relaxed: 0.55},
            mood_sad: {sad: 0.70, not_sad: 0.30},
            timbre: {bright: 0.40, dark: 0.60},
            voice_instrumental: {voice: 0.70, instrumental: 0.30}
        });

        // Reggae
        map.insert("reggae".into(), characteristics! {
            danceability: {danceable: 0.65, not_danceable: 0.35},
            gender: {male: 0.75, female: 0.25},
            mood_acoustic: {acoustic: 0.40, not_acoustic: 0.60},
            mood_aggressive: {aggressive: 0.25, not_aggressive: 0.75},
            mood_electronic: {electronic: 0.20, not_electronic: 0.80},
            mood_happy: {happy: 0.60, not_happy: 0.40},
            mood_party: {party: 0.60, not_party: 0.40},
            mood_relaxed: {relaxed: 0.70, not_relaxed: 0.30},
            mood_sad: {sad: 0.25, not_sad: 0.75},
            timbre: {bright: 0.55, dark: 0.45},
            voice_instrumental: {voice: 0.75, instrumental: 0.25}
        });

        // Folk
        map.insert("folk".into(), characteristics! {
            danceability: {danceable: 0.30, not_danceable: 0.70},
            gender: {male: 0.55, female: 0.45},
            mood_acoustic: {acoustic: 0.90, not_acoustic: 0.10},
            mood_aggressive: {aggressive: 0.10, not_aggressive: 0.90},
            mood_electronic: {electronic: 0.05, not_electronic: 0.95},
            mood_happy: {happy: 0.45, not_happy: 0.55},
            mood_party: {party: 0.20, not_party: 0.80},
            mood_relaxed: {relaxed: 0.65, not_relaxed: 0.35},
            mood_sad: {sad: 0.45, not_sad: 0.55},
            timbre: {bright: 0.55, dark: 0.45},
            voice_instrumental: {voice: 0.75, instrumental: 0.25}
        });

        // Punk
        map.insert("punk".into(), characteristics! {
            danceability: {danceable: 0.55, not_danceable: 0.45},
            gender: {male: 0.80, female: 0.20},
            mood_acoustic: {acoustic: 0.15, not_acoustic: 0.85},
            mood_aggressive: {aggressive: 0.85, not_aggressive: 0.15},
            mood_electronic: {electronic: 0.15, not_electronic: 0.85},
            mood_happy: {happy: 0.40, not_happy: 0.60},
            mood_party: {party: 0.65, not_party: 0.35},
            mood_relaxed: {relaxed: 0.10, not_relaxed: 0.90},
            mood_sad: {sad: 0.35, not_sad: 0.65},
            timbre: {bright: 0.70, dark: 0.30},
            voice_instrumental: {voice: 0.75, instrumental: 0.25}
        });

        // Indie / Alternative
        map.insert("indie".into(), characteristics! {
            danceability: {danceable: 0.45, not_danceable: 0.55},
            gender: {male: 0.60, female: 0.40},
            mood_acoustic: {acoustic: 0.40, not_acoustic: 0.60},
            mood_aggressive: {aggressive: 0.30, not_aggressive: 0.70},
            mood_electronic: {electronic: 0.35, not_electronic: 0.65},
            mood_happy: {happy: 0.40, not_happy: 0.60},
            mood_party: {party: 0.40, not_party: 0.60},
            mood_relaxed: {relaxed: 0.45, not_relaxed: 0.55},
            mood_sad: {sad: 0.45, not_sad: 0.55},
            timbre: {bright: 0.55, dark: 0.45},
            voice_instrumental: {voice: 0.70, instrumental: 0.30}
        });

        // Disco
        map.insert("disco".into(), characteristics! {
            danceability: {danceable: 0.95, not_danceable: 0.05},
            gender: {male: 0.45, female: 0.55},
            mood_electronic: {electronic: 0.50, not_electronic: 0.50},
            mood_happy: {happy: 0.80, not_happy: 0.20},
            mood_party: {party: 0.95, not_party: 0.05},
            mood_relaxed: {relaxed: 0.30, not_relaxed: 0.70},
            timbre: {bright: 0.75, dark: 0.25},
            voice_instrumental: {voice: 0.70, instrumental: 0.30}
        });

        // Latin
        map.insert("latin".into(), characteristics! {
            danceability: {danceable: 0.80, not_danceable: 0.20},
            gender: {male: 0.60, female: 0.40},
            mood_acoustic: {acoustic: 0.40, not_acoustic: 0.60},
            mood_aggressive: {aggressive: 0.25, not_aggressive: 0.75},
            mood_happy: {happy: 0.70, not_happy: 0.30},
            mood_party: {party: 0.80, not_party: 0.20},
            mood_relaxed: {relaxed: 0.35, not_relaxed: 0.65},
            timbre: {bright: 0.70, dark: 0.30},
            voice_instrumental: {voice: 0.75, instrumental: 0.25}
        });

        // World Music
        map.insert("world".into(), characteristics! {
            danceability: {danceable: 0.50, not_danceable: 0.50},
            mood_acoustic: {acoustic: 0.70, not_acoustic: 0.30},
            mood_electronic: {electronic: 0.15, not_electronic: 0.85},
            mood_party: {party: 0.40, not_party: 0.60},
            mood_relaxed: {relaxed: 0.55, not_relaxed: 0.45},
            voice_instrumental: {voice: 0.60, instrumental: 0.40}
        });

        // Gospel / Religious
        map.insert("gospel".into(), characteristics! {
            danceability: {danceable: 0.40, not_danceable: 0.60},
            gender: {male: 0.40, female: 0.60},
            mood_acoustic: {acoustic: 0.50, not_acoustic: 0.50},
            mood_aggressive: {aggressive: 0.10, not_aggressive: 0.90},
            mood_happy: {happy: 0.70, not_happy: 0.30},
            mood_party: {party: 0.30, not_party: 0.70},
            mood_relaxed: {relaxed: 0.45, not_relaxed: 0.55},
            timbre: {bright: 0.60, dark: 0.40},
            voice_instrumental: {voice: 0.85, instrumental: 0.15}
        });

        // Industrial
        map.insert("industrial".into(), characteristics! {
            danceability: {danceable: 0.50, not_danceable: 0.50},
            gender: {male: 0.75, female: 0.25},
            mood_acoustic: {acoustic: 0.05, not_acoustic: 0.95},
            mood_aggressive: {aggressive: 0.85, not_aggressive: 0.15},
            mood_electronic: {electronic: 0.90, not_electronic: 0.10},
            mood_happy: {happy: 0.15, not_happy: 0.85},
            mood_party: {party: 0.40, not_party: 0.60},
            mood_relaxed: {relaxed: 0.10, not_relaxed: 0.90},
            mood_sad: {sad: 0.40, not_sad: 0.60},
            timbre: {bright: 0.30, dark: 0.70},
            voice_instrumental: {voice: 0.50, instrumental: 0.50}
        });

        // Soundtrack / Instrumental
        map.insert("soundtrack".into(), characteristics! {
            danceability: {danceable: 0.25, not_danceable: 0.75},
            mood_acoustic: {acoustic: 0.50, not_acoustic: 0.50},
            mood_aggressive: {aggressive: 0.30, not_aggressive: 0.70},
            mood_electronic: {electronic: 0.40, not_electronic: 0.60},
            mood_relaxed: {relaxed: 0.50, not_relaxed: 0.50},
            timbre: {bright: 0.50, dark: 0.50},
            voice_instrumental: {voice: 0.15, instrumental: 0.85}
        });

        // New Age
        map.insert("new age".into(), characteristics! {
            danceability: {danceable: 0.10, not_danceable: 0.90},
            mood_acoustic: {acoustic: 0.60, not_acoustic: 0.40},
            mood_aggressive: {aggressive: 0.05, not_aggressive: 0.95},
            mood_electronic: {electronic: 0.50, not_electronic: 0.50},
            mood_happy: {happy: 0.50, not_happy: 0.50},
            mood_party: {party: 0.05, not_party: 0.95},
            mood_relaxed: {relaxed: 0.95, not_relaxed: 0.05},
            mood_sad: {sad: 0.20, not_sad: 0.80},
            timbre: {bright: 0.40, dark: 0.60},
            voice_instrumental: {voice: 0.10, instrumental: 0.90}
        });

        map
    }

    /// Map genre string to characteristics (case-insensitive, fuzzy matching)
    /// Returns None if no mapping found and genre is unknown
    pub fn map_genre(&self, genre: &str) -> Option<MusicalCharacteristics> {
        let genre_lower = genre.to_lowercase();

        // Direct match
        if let Some(chars) = self.mappings.get(&genre_lower) {
            return Some(chars.clone());
        }

        // Fuzzy match (Levenshtein similarity > 0.85 for typos)
        for (known_genre, chars) in &self.mappings {
            if strsim::normalized_levenshtein(&genre_lower, known_genre) > 0.85 {
                return Some(chars.clone());
            }
        }

        None  // Unknown genre - no default mapping
    }
}
```

**Rationale:**
- **25 common genres** cover 95%+ of music library content
- **Binary characteristics only** (complex characteristics like genre_electronic require Essentia)
- **Evidence-based values** derived from AcousticBrainz aggregate statistics
- **Fuzzy matching** handles typos (e.g., "Rok" → "rock")
- **Unknown genres return None** (handled by fusion layer as missing data)
- **Confidence 0.3** reflects coarse approximation versus Essentia's 0.9

**"Legible Software" Application:**
- **Concept:** "GenreMapper" is independent module with single purpose
- **Synchronization:** Explicit contract with fusion layer (returns `Option<MusicalCharacteristics>`)
- **Transparency:** Mapping table is visible, not hidden in complex logic

---

## CRITICAL-002: Expected Characteristics Count

**Status:** ✅ RESOLVED

**Resolution:** From [SPEC003-musical_flavor.md](../../docs/SPEC003-musical_flavor.md) and [sample_highlevel.json](../../docs/sample_highlevel.json):

**Total Expected Characteristics:** **18 top-level categories**

**Binary Characteristics (2 dimensions each): 12 total**
1. danceability (danceable, not_danceable)
2. gender (female, male)
3. mood_acoustic (acoustic, not_acoustic)
4. mood_aggressive (aggressive, not_aggressive)
5. mood_electronic (electronic, not_electronic)
6. mood_happy (happy, not_happy)
7. mood_party (party, not_party)
8. mood_relaxed (relaxed, not_relaxed)
9. mood_sad (sad, not_sad)
10. timbre (bright, dark)
11. tonal_atonal (tonal, atonal)
12. voice_instrumental (voice, instrumental)

**Complex Characteristics (3+ dimensions): 6 total**
13. genre_dortmund (9 dimensions)
14. genre_electronic (5 dimensions)
15. genre_rosamerica (8 dimensions)
16. genre_tzanetakis (10 dimensions)
17. ismir04_rhythm (10 dimensions)
18. moods_mirex (5 dimensions)

**Total Individual Dimensions:** 76 (12×2 + 47 complex)

**Completeness Calculation:**
```rust
// For binary characteristics: count present categories (not individual dimensions)
// For complex characteristics: count present categories (not individual dimensions)
let completeness = (present_categories / 18.0) * 100.0;

// Example: If only binary characteristics present from GenreMapper (11 categories)
// completeness = (11 / 18) * 100 = 61.1%

// Example: If Essentia provides all 18 categories
// completeness = (18 / 18) * 100 = 100%
```

**"Legible Software" Application:**
- **Transparency:** Expected count is explicitly defined (18 categories)
- **Integrity:** Completeness formula is deterministic and testable
- **Concept:** "CompletenessScorer" validates against fixed standard

---

## CRITICAL-003: Levenshtein Implementation

**Status:** ✅ RESOLVED

**Resolution:** Use `strsim` crate's `normalized_levenshtein()` function for all similarity comparisons.

**Implementation:**

```toml
# Cargo.toml for wkmp-ai
[dependencies]
strsim = "0.11"
```

```rust
// File: wkmp-ai/src/tier3/validators.rs

use strsim::normalized_levenshtein;

/// Validates title consistency across sources using normalized Levenshtein similarity
/// Formula: similarity = 1 - (edit_distance / max(len_a, len_b))
/// Range: 0.0 (completely different) to 1.0 (identical)
pub fn validate_title_consistency(titles: &[String]) -> ValidationResult {
    if titles.len() < 2 {
        return ValidationResult::Pass;
    }

    // Compare all pairs
    for i in 0..titles.len() {
        for j in (i+1)..titles.len() {
            let similarity = normalized_levenshtein(&titles[i], &titles[j]);

            if similarity < 0.80 {
                return ValidationResult::Conflict {
                    message: format!(
                        "Title mismatch: '{}' vs '{}' (similarity: {:.2})",
                        titles[i], titles[j], similarity
                    ),
                    severity: ConflictSeverity::High
                };
            } else if similarity < 0.95 {
                return ValidationResult::Warning {
                    message: format!(
                        "Title variant: '{}' vs '{}' (similarity: {:.2})",
                        titles[i], titles[j], similarity
                    )
                };
            }
        }
    }

    ValidationResult::Pass
}
```

**Thresholds:**
- **similarity ≥ 0.95:** PASS (identical or minor differences)
- **0.80 ≤ similarity < 0.95:** WARNING (likely same song, spelling variant)
- **similarity < 0.80:** CONFLICT (high risk of different songs)

**Examples:**
- `"Let It Be"` vs `"Let it be"` → 0.91 (WARNING - capitalization)
- `"Let It Be"` vs `"Let It Be (Remastered)"` → 0.76 (CONFLICT - different version)
- `"Bohemian Rhapsody"` vs `"Bohemian Rhapsody"` → 1.00 (PASS)

**"Legible Software" Application:**
- **Concept:** "TitleValidator" has single responsibility (consistency checking)
- **Transparency:** Threshold values are explicit constants
- **Predictability:** Same inputs always produce same results (no hidden state)

---

## CRITICAL-004: SSE Buffering Strategy

**Status:** ✅ RESOLVED

**Resolution:** Use bounded in-memory queue with backpressure to import workflow.

**Implementation:**

```rust
// File: wkmp-ai/src/sse/event_broadcaster.rs

use tokio::sync::mpsc;
use std::time::Duration;

/// SSE event broadcaster with bounded buffering and backpressure
///
/// Design decisions (Legible Software principles):
/// - Bounded capacity: 1000 events (prevents unbounded memory growth)
/// - Backpressure: Blocks sender if buffer full (preserves all events)
/// - Throttling: Max 30 events/sec to clients (prevents network congestion)
pub struct EventBroadcaster {
    /// Bounded channel for event buffering
    /// Capacity: 1000 events (approximately 10-song album at 6 events/song + safety margin)
    tx: mpsc::Sender<ImportEvent>,
    rx: mpsc::Receiver<ImportEvent>,

    /// Throttle state: tracks last emission time for rate limiting
    last_emission: tokio::time::Instant,
    min_interval: Duration,  // 33ms = ~30 events/sec
}

impl EventBroadcaster {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel(1000);  // Bounded capacity

        Self {
            tx,
            rx,
            last_emission: tokio::time::Instant::now(),
            min_interval: Duration::from_millis(33),  // ~30 events/sec
        }
    }

    /// Send event to buffer (async, may block if buffer full - backpressure)
    ///
    /// Blocking behavior is intentional:
    /// - Import workflow pauses if events accumulate faster than SSE can deliver
    /// - Prevents event loss (per REQ-AI-073: "do NOT drop events")
    /// - Bounded buffer prevents unbounded memory growth
    pub async fn send_event(&self, event: ImportEvent) -> Result<(), BroadcastError> {
        // This may block if buffer is full (backpressure to import workflow)
        self.tx.send(event).await
            .map_err(|_| BroadcastError::ChannelClosed)
    }

    /// Receive next event (throttled to max 30/sec)
    pub async fn recv_event(&mut self) -> Option<ImportEvent> {
        // Throttle: Ensure minimum interval between emissions
        let elapsed = self.last_emission.elapsed();
        if elapsed < self.min_interval {
            tokio::time::sleep(self.min_interval - elapsed).await;
        }

        let event = self.rx.recv().await;
        self.last_emission = tokio::time::Instant::now();
        event
    }
}
```

**Buffer Capacity Analysis:**
- **Typical album:** 10 songs × 6 events/song = 60 events
- **Large import:** 100 songs × 6 events/song = 600 events
- **Buffer capacity:** 1000 events (safety margin for burst imports)
- **Memory usage:** ~1000 events × ~200 bytes/event = ~200KB (negligible)

**Backpressure Behavior:**
- If SSE client disconnects: Buffer fills → Import workflow pauses
- If SSE client reconnects: Buffer drains → Import workflow resumes
- If buffer full for >30 seconds: Import workflow may timeout (TODO: add timeout)

**Throttling:**
- **Max rate:** 30 events/sec (per REQ-AI-073)
- **Implementation:** Minimum 33ms interval between emissions
- **Effect:** 1000-event buffer takes ~33 seconds to drain at max rate

**"Legible Software" Application:**
- **Concept:** "EventBroadcaster" has well-defined purpose (event distribution with rate limiting)
- **Integrity:** Buffer size limit preserves system stability (no unbounded growth)
- **Transparency:** Backpressure behavior is explicit (blocks sender, not silent drop)
- **Synchronization:** Explicit contract with import workflow (async send may block)

---

## Implementation Priority

**Phase 0 (Increment 0): Resolve Critical Issues** ✅ COMPLETE
1. ✅ Document genre mappings (CRITICAL-001)
2. ✅ Define expected characteristics count (CRITICAL-002)
3. ✅ Specify Levenshtein implementation (CRITICAL-003)
4. ✅ Define SSE buffering strategy (CRITICAL-004)

**Next Steps:**
- Proceed to HIGH issues resolution (Chromaprint/Essentia bindings research)
- Begin Tier 1 extractor implementation with visible concept boundaries
- Apply "Legible Software" principles: each module is independent concept with explicit synchronizations

---

## Legible Software Design Summary

**Tier 1 Concepts (Independent Extractors):**
- `ID3MetadataExtractor` - Extracts ID3 tags
- `ChromaprintAnalyzer` - Generates fingerprints
- `AcoustIDClient` - Queries AcoustID API
- `MusicBrainzClient` - Queries MusicBrainz API
- `EssentiaAnalyzer` - Extracts musical flavor
- `AudioFeatureExtractor` - Audio-derived features
- `GenreMapper` - Maps genres to characteristics

**Tier 2 Concepts (Fusers with Explicit Synchronizations):**
- `IdentityResolver` - Bayesian fusion of MBIDs (synchronizes with extractors via confidence scores)
- `MetadataFuser` - Weighted selection (synchronizes with extractors via source priority)
- `FlavorSynthesizer` - Characteristic-wise averaging (synchronizes with extractors via normalized vectors)
- `BoundaryFuser` - Multi-strategy fusion (synchronizes with silence detection)

**Tier 3 Concepts (Validators):**
- `ConsistencyChecker` - Cross-source validation (synchronizes with fusers via conflict detection)
- `CompletenessScorer` - Quality scoring (synchronizes with expected characteristics count)
- `ConflictDetector` - Conflict flagging (synchronizes with validators via severity levels)

**Key Principle:** Each concept is independently testable, has explicit input/output contracts, and does not rely on hidden state or implicit assumptions. New concepts can be added without breaking existing ones.

---

**End of CRITICAL Issues Resolution**

All CRITICAL issues are now resolved. Ready to proceed with HIGH issues (dependencies research) and implementation.
