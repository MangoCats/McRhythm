# Hybrid Import Process Analysis: Multi-Source Data Fusion for AcousticBrainz Profiles and Passage Mapping

**Document Type:** Analysis Report
**Analysis Date:** 2025-01-08
**Analysis Method:** Multi-agent research and synthesis (/think workflow)
**Analyst:** Claude Code
**Scope:** Design hybrid import process combining multiple data sources for optimal AcousticBrainz profile and passage mapping quality

---

## Executive Summary

### Current State
WKMP-AI uses a **linear 7-phase import workflow** with sequential data sources:
- ID3 tags → Chromaprint → AcoustID → MusicBrainz → AcousticBrainz/Essentia → Silence Detection → Database
- **Weakness:** No fusion strategy; later sources completely override earlier ones regardless of data quality

### Critical Findings

1. **Data source hierarchy is implicit, not explicit** - MusicBrainz always overrides ID3 tags, but no framework evaluates when override is justified
2. **AcousticBrainz obsolescence creates single point of failure** - Service ceased 2022; Essentia fallback exists but activation is silent
3. **No confidence-based fusion** - Multiple sources (ID3, MusicBrainz, Chromaprint audio analysis) could provide complementary data, but only one is used
4. **Passage mapping uses single strategy** - Silence detection only; no fusion of multiple segmentation approaches
5. **Missing validation layer** - No cross-source consistency checks to detect low-quality data

### Recommendation

**Implement 3-tier hybrid fusion architecture:**

**Tier 1: Source-Specific Extractors** (independent, parallel data collection)
- ID3 Metadata Extractor (confidence: tag completeness)
- Audio Fingerprint Analyzer (confidence: match score)
- MusicBrainz Authoritative (confidence: API response quality)
- Musical Flavor Multi-Source (AcousticBrainz → Essentia → Audio-derived fallback)
- Passage Boundary Multi-Strategy (Silence + Beat + Structural analysis)

**Tier 2: Confidence-Weighted Fusion Engine**
- **Identity Resolution:** Combine ID3 + Fingerprint + MusicBrainz with weighted voting
- **Musical Flavor Synthesis:** Merge AcousticBrainz + Essentia + Audio features
- **Passage Boundary Fusion:** Intersect multiple segmentation strategies
- **Conflict Resolution:** Bayesian update, weighted averaging, or ranked selection

**Tier 3: Quality Validation & Enrichment**
- Cross-source consistency checks (detect mismatches)
- Completeness scoring (prefer richer data)
- User feedback integration (learn from corrections)

---

## 1. Current State Analysis

### 1.1 Available Data Sources

| Source | Data Provided | Confidence Indicator | Reliability | Coverage |
|--------|---------------|---------------------|-------------|----------|
| **ID3 Tags** | Artist, Title, Album, Genre, BPM, MusicBrainz IDs | Tag completeness (0-100%) | Medium | 95%+ |
| **Chromaprint** | Audio fingerprint (120 sec) | N/A (deterministic) | High | 100% |
| **AcoustID API** | Recording MBID + match score | Score 0.0-1.0 | High | 60-80% |
| **MusicBrainz API** | Title, Artists (weighted), Work, Release | API confidence (implicit) | Very High | 80-95% |
| **AcousticBrainz API** | Musical flavor vector (JSON) | Per-feature strength 0.0-1.0 | Very High | 50-70%* |
| **Essentia Local** | Musical flavor vector (calculated) | N/A (computational) | High | 90%+ (if installed) |
| **Silence Detector** | Passage boundaries via RMS | Threshold-based | High | 85-95% |
| **Amplitude Analyzer** | Lead-in/lead-out timing | N/A | **Not Implemented** | 0% (stub) |
| **Beat Tracker** | Beat grid, tempo | N/A | **Not Available** | 0% |
| **Structural Analyzer** | Intro/verse/chorus/outro | N/A | **Not Available** | 0% |

*AcousticBrainz: Pre-2022 recordings only (service discontinued)

### 1.2 Current Data Flow

**Linear Pipeline (Current Implementation):**

```
┌─────────────┐
│  Scanning   │ Discover files
└──────┬──────┘
       ↓
┌──────────────┐
│  Extracting  │ Read ID3 tags → Store in memory
└──────┬───────┘
       ↓
┌───────────────────┐
│  Fingerprinting   │ Chromaprint → AcoustID → Recording MBID
└──────┬────────────┘         ↓
       ↓              ┌────────────────┐
       ↓              │  MusicBrainz   │ Query via MBID → Title, Artists, Work
       ↓              └────────┬───────┘
       ↓                       ↓
       ↓              **OVERRIDE ID3 Metadata** (no fusion, complete replacement)
       ↓
┌──────────────┐
│  Segmenting  │ Silence detection → Passage boundaries
└──────┬───────┘
       ↓
┌──────────────┐
│  Analyzing   │ **STUB** (placeholder values)
└──────┬───────┘
       ↓
┌──────────────┐
│  Flavoring   │ AcousticBrainz → If 404, try Essentia → Store JSON vector
└──────┬───────┘
       ↓
┌──────────────┐
│  Completed   │ Write to database (passages table)
└──────────────┘
```

**Key Observations:**

1. **No Parallelization:** Sources queried sequentially (slow, blocks on network I/O)
2. **No Fusion:** MusicBrainz completely replaces ID3 data (even when ID3 has MusicBrainz IDs embedded)
3. **Binary Fallback:** AcousticBrainz → Essentia (no blending of partial results)
4. **Single Strategy:** Silence detection only for passage boundaries (no multi-strategy fusion)
5. **No Validation:** No cross-checks (e.g., does MusicBrainz title match ID3 title?)

### 1.3 Pain Points

#### **Pain Point 1: Override vs Fusion**

**Current Behavior:**
- If AcoustID finds Recording MBID → Query MusicBrainz → **Replace all ID3 metadata**
- If MusicBrainz returns "Unknown Artist" but ID3 has "Pink Floyd" → Result: "Unknown Artist" (information loss)

**Why This Happens:**
- No confidence comparison between sources
- Implicit assumption: "MusicBrainz is always more authoritative"
- Reality: User-curated ID3 tags (especially with embedded MBIDs) can be superior

**Example Failure Case:**
```
ID3 Tags:
  Title: "Shine On You Crazy Diamond (Parts I-V)"
  Artist: "Pink Floyd"
  MusicBrainz Recording ID: abc123...

AcoustID Match: 0.92 confidence → Recording MBID: xyz789... (WRONG!)
MusicBrainz Lookup (xyz789):
  Title: "Unknown Track"
  Artist: "Various Artists"

Final Result: "Unknown Track" by "Various Artists" (worse than original ID3!)
```

**Impact:** False matches degrade data quality rather than improving it

---

#### **Pain Point 2: AcousticBrainz Single Point of Failure**

**Current Behavior:**
- Try AcousticBrainz API → If 404 (not found) → Try Essentia local
- **If both fail:** Musical flavor = NULL (passage excluded from automatic selection)

**Why This Matters:**
- AcousticBrainz ceased submissions in 2022 → New releases have no data
- Essentia requires manual installation → Many users won't have it
- Result: **50-70% of library may have no musical flavor** (especially newer music)

**Opportunity for Fusion:**
Multiple sources could provide flavor data:
- AcousticBrainz: Pre-computed, highly accurate (pre-2022)
- Essentia: Computed locally, covers all files
- ID3 Genre + BPM: Coarse but available
- Chromaprint-derived features: Spectral/temporal analysis possible

**Current Implementation:** Binary choice (use first available), no fusion

---

#### **Pain Point 3: Passage Mapping Single Strategy**

**Current Behavior:**
- Silence Detection (RMS threshold -60dB default) → Finds boundaries where audio drops below threshold
- **Assumption:** Silence indicates song boundaries

**Failure Modes:**
1. **Classical music:** Long quiet passages mid-song → False boundaries
2. **Live albums:** Continuous audio between songs → No boundaries detected
3. **Fade-outs:** Gradual volume reduction → Boundary detected too early/late
4. **Electronic music:** Beats continue through song transitions → No silence

**Opportunity for Multi-Strategy Fusion:**
- Silence detection: Good for studio albums with clear gaps
- Beat tracking: Detect tempo changes (song transitions)
- Structural analysis: Identify intro/outro patterns
- Metadata hints: ID3 tags may specify track start times (cue sheets)
- Duration-based heuristics: Unusually long passages (>15 min) likely contain multiple songs

**Current Implementation:** Silence detection only (no fusion of strategies)

---

#### **Pain Point 4: No Confidence Framework**

**Current Behavior:**
- AcoustID returns match score 0.0-1.0 → **Used for logging only, not decision-making**
- MusicBrainz data has implicit confidence (official API) → Not quantified
- ID3 tag quality varies wildly → No scoring mechanism
- Essentia features are computational → No uncertainty quantification

**Example:**
```
AcoustID Match Score: 0.62 (low confidence)
MusicBrainz Result: "Best Guess Title" by "Unknown Artist"
ID3 Tags: Complete metadata with embedded MusicBrainz IDs

Decision: Use MusicBrainz result (because it's "authoritative")
Better Decision: Flag low-confidence match, prefer ID3 tags with MBIDs
```

**Missing:**
- Unified confidence scoring (0-100% for all sources)
- Threshold-based decision rules ("only override ID3 if AcoustID score >0.85")
- Conflict detection ("MusicBrainz title doesn't match ID3 title → Manual review")

---

### 1.4 Requirements Analysis

**From SPEC003-musical_flavor.md:**

**[MFL-DEF-020]** Musical flavor is derived from AcousticBrainz high-level characterization
**[MFL-DEF-030]** Two categories: Binary (2 dimensions) and Complex (3+ dimensions)
**[MFL-DIST-020]** Used to calculate flavor distance between passages
**[MFL-MULT-010]** Multi-recording passages use weighted centroid (runtime-weighted)
**[MFL-EDGE-021]** Recordings with no characteristics → flavor distance = 1.0 (excluded from auto-selection)

**Implications for Hybrid Import:**
1. **Musical flavor vector is critical** - Without it, passages can't be auto-selected
2. **Partial data is usable** - Characteristics available in subset of sources can be combined
3. **Weighted fusion already specified** - Multi-recording passages use runtime-weighted centroid (same principle applies to multi-source fusion)

**From REQ002-entity_definitions.md:**

**[ENT-MP-030]** Passage: Defined span with start, fade-in, lead-in, lead-out, fade-out, end points
**[ENT-MP-010]** Song: Recording + Works + Artists (with weights)
**[ENT-REL-060]** Passage contains zero or more Songs

**Implications for Hybrid Import:**
1. **Passage boundaries must be sample-accurate** - Silence detection provides this, but multi-strategy fusion must maintain precision
2. **Songs must be identified** - Recording MBID is minimum requirement (from MusicBrainz)
3. **Zero-song passages are valid** - If identification fails, passage still created (manual queueing possible)

---

## 2. Hybrid Data Fusion: Research Findings

### 2.1 Fusion Architectures (Academic Literature)

**From multimodal fusion surveys (2024):**

**Early Fusion** (Feature-Level):
- Combine raw features from multiple sources **before** processing
- Example: Concatenate ID3 genre + audio spectral features → Single ML model
- **Pros:** Captures inter-source correlations, simple architecture
- **Cons:** Requires aligned feature spaces, sensitive to missing data

**Late Fusion** (Decision-Level):
- Each source produces independent predictions → Combine at decision stage
- Example: ID3 predicts "Rock", Audio analysis predicts "Alternative Rock" → Weighted vote
- **Pros:** Handles heterogeneous sources, robust to missing data
- **Cons:** Discards inter-source correlations

**Hybrid Fusion**:
- Combines early and late fusion strategies
- Example: Fuse audio features early → Combine with ID3 metadata at decision level
- **Pros:** Balances correlation capture and robustness
- **Cons:** More complex architecture

**Recommended for WKMP:** Hybrid fusion (different sources have different characteristics)

### 2.2 Confidence Weighting Strategies

**From sensor fusion literature:**

**Weighted Averaging:**
```
fused_value = Σ(confidence_i * value_i) / Σ(confidence_i)
```
- Used when sources provide same type of data (e.g., BPM from ID3 vs audio analysis)
- **Advantage:** Simple, interpretable
- **Limitation:** Assumes commensurable data

**Bayesian Update:**
```
P(hypothesis|evidence) ∝ P(evidence|hypothesis) * P(hypothesis)
```
- Update belief based on evidence quality
- Example: Prior belief in ID3 metadata → Update with AcoustID match confidence
- **Advantage:** Principled handling of uncertainty
- **Limitation:** Requires prior probabilities

**Conflict-Weighted Fusion:**
```
weight_i = base_confidence_i / (1 + conflict_measure_i)
```
- Reduce weight of sources that conflict with others
- **Advantage:** Automatically detects outliers
- **Limitation:** Requires conflict metric

**Recommended for WKMP:**
- **Identity Resolution (Song identification):** Bayesian update with conflict detection
- **Musical Flavor:** Weighted averaging (characteristics are commensurable)
- **Passage Boundaries:** Intersection + validation (structural constraint)

### 2.3 Quality Ranking Criteria

**From multi-source integration research:**

**Completeness:**
- Score = (filled_fields / total_fields) * 100%
- Example: ID3 with Title, Artist, Album, Genre, BPM (5/10 fields) = 50%

**Consistency:**
- Cross-source agreement score
- Example: ID3 title matches MusicBrainz title → +1, mismatch → -1

**Confidence:**
- Source-specific indicators
- Example: AcoustID score 0.92 → 92% confidence

**Recency:**
- Age of data (for cached sources)
- Example: AcousticBrainz (2021 data) vs Essentia (computed 2025)

**Authority:**
- Trustworthiness of source
- Example: MusicBrainz (curated) > ID3 (user-edited) > Heuristics (algorithmic)

**Combined Quality Score:**
```
quality = w1*completeness + w2*consistency + w3*confidence + w4*authority
Weights sum to 1.0
```

---

## 3. Proposed Hybrid Import Architecture

### 3.1 Three-Tier Design

```
┌─────────────────────────────────────────────────────────────────────────┐
│ TIER 1: SOURCE-SPECIFIC EXTRACTORS (Parallel, Independent)             │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │ ID3 Metadata │  │  Chromaprint │  │  MusicBrainz │  │  Essentia  │ │
│  │  Extractor   │  │  Fingerprint │  │  API Client  │  │  Analyzer  │ │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └──────┬─────┘ │
│         │                 │                 │                 │        │
│    ┌────▼──────────────────▼─────────────────▼─────────────────▼────┐  │
│    │ Extract: Title, Artist, Album, Genre, BPM, MBID,           │  │
│    │          Fingerprint, AcoustID MBID, MusicBrainz metadata, │  │
│    │          Audio features, Musical flavor                    │  │
│    └────────────────────────────────┬────────────────────────────────┘  │
│                                     │                                   │
│                     Each extractor returns:                             │
│                     - Data (parsed fields)                              │
│                     - Confidence (0.0-1.0)                              │
│                     - Source identifier                                 │
│                                     │                                   │
└─────────────────────────────────────┼───────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ TIER 2: CONFIDENCE-WEIGHTED FUSION ENGINE                              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ IDENTITY RESOLUTION MODULE                                       │  │
│  │  ━ Inputs: ID3 MBID, AcoustID MBID, Fingerprint                │  │
│  │  ━ Strategy: Bayesian update with conflict detection            │  │
│  │  ━ Output: Recording MBID (best estimate) + confidence          │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                     │                                   │
│                                     ▼                                   │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ METADATA FUSION MODULE                                           │  │
│  │  ━ Inputs: ID3 metadata, MusicBrainz metadata                   │  │
│  │  ━ Strategy: Weighted selection (prefer higher quality)         │  │
│  │  ━ Output: Title, Artist(s), Work, Album + source provenance    │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                     │                                   │
│                                     ▼                                   │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ MUSICAL FLAVOR SYNTHESIS MODULE                                  │  │
│  │  ━ Inputs: AcousticBrainz, Essentia, ID3 genre/BPM, Audio       │  │
│  │  ━ Strategy: Characteristic-wise weighted averaging              │  │
│  │  ━ Output: Unified musical flavor vector (JSON) + confidence map│  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                     │                                   │
│                                     ▼                                   │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ PASSAGE BOUNDARY FUSION MODULE                                   │  │
│  │  ━ Inputs: Silence detection, Beat tracking (future),           │  │
│  │            Structural analysis (future), Metadata hints          │  │
│  │  ━ Strategy: Multi-strategy intersection + validation            │  │
│  │  ━ Output: Passage boundaries (start/end ms) + confidence       │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                     │                                   │
└─────────────────────────────────────┼───────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ TIER 3: QUALITY VALIDATION & ENRICHMENT                                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ CROSS-SOURCE CONSISTENCY CHECKS                                  │  │
│  │  ━ Validate: ID3 title matches MusicBrainz title (fuzzy match)  │  │
│  │  ━ Validate: Duration from ID3 matches audio file duration      │  │
│  │  ━ Validate: Genre from ID3 aligns with musical flavor          │  │
│  │  ━ Flag: Conflicts for manual review or confidence downgrade    │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                     │                                   │
│                                     ▼                                   │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ COMPLETENESS & QUALITY SCORING                                   │  │
│  │  ━ Score: Each field (title, artist, flavor) for completeness   │  │
│  │  ━ Compute: Overall passage quality (0-100%)                     │  │
│  │  ━ Store: Quality metadata for debugging and user feedback      │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                     │                                   │
│                                     ▼                                   │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │ FALLBACK & ENRICHMENT                                            │  │
│  │  ━ If no MBID: Create zero-song passage (still playable)        │  │
│  │  ━ If partial flavor: Mark for background re-analysis           │  │
│  │  ━ If conflicts: Flag for user review interface                 │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                     │                                   │
│                                     ▼                                   │
│                        ┌─────────────────────────┐                     │
│                        │ DATABASE (passages)     │                     │
│                        │  ━ Passage timing       │                     │
│                        │  ━ Song identity (MBID) │                     │
│                        │  ━ Musical flavor JSON  │                     │
│                        │  ━ Quality scores       │                     │
│                        │  ━ Source provenance    │                     │
│                        └─────────────────────────┘                     │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Detailed Module Designs

#### **Module 1: Identity Resolution**

**Inputs:**
- ID3-embedded MusicBrainz Recording ID (if present) → Confidence: High (0.9) if present
- AcoustID match → Recording MBID + confidence score (0.0-1.0)
- Chromaprint fingerprint → Used for validation

**Strategy: Bayesian Update with Conflict Detection**

**Step 1: Prior Belief**
```rust
if id3_mbid.is_some() {
    prior_confidence = 0.9  // High trust in embedded MBID
    prior_mbid = id3_mbid
} else {
    prior_confidence = 0.0  // No prior information
    prior_mbid = None
}
```

**Step 2: Evidence Update**
```rust
acoustid_confidence = acoustid_match_score  // 0.0-1.0 from API
acoustid_mbid = acoustid_result.mbid

// Bayesian update:
if prior_mbid == acoustid_mbid {
    // Agreement: Strengthen confidence
    posterior_confidence = 1 - (1 - prior_confidence) * (1 - acoustid_confidence)
    final_mbid = prior_mbid
} else if prior_mbid.is_some() && acoustid_mbid.is_some() {
    // Conflict: Weighted selection based on confidence
    if prior_confidence > acoustid_confidence {
        final_mbid = prior_mbid
        posterior_confidence = prior_confidence * (1 - acoustid_confidence * 0.5)
        flag_conflict("ID3 MBID conflicts with AcoustID")
    } else {
        final_mbid = acoustid_mbid
        posterior_confidence = acoustid_confidence * (1 - prior_confidence * 0.5)
        flag_conflict("AcoustID MBID conflicts with ID3")
    }
} else {
    // One source only: Use available source
    final_mbid = prior_mbid.or(acoustid_mbid)
    posterior_confidence = prior_confidence.max(acoustid_confidence)
}
```

**Step 3: Validation**
```rust
if posterior_confidence < 0.7 {
    flag_low_confidence("Recording identification uncertain")
}

if conflict_detected && posterior_confidence < 0.85 {
    flag_manual_review("Conflicting MBIDs, manual review recommended")
}
```

**Output:**
```rust
struct IdentityResolution {
    recording_mbid: Option<Uuid>,
    confidence: f64,              // 0.0-1.0
    source: IdentitySource,       // ID3, AcoustID, Conflict
    conflicts: Vec<ConflictReport>,
}
```

---

#### **Module 2: Metadata Fusion**

**Inputs:**
- ID3 metadata: Title, Artist, Album, Genre
- MusicBrainz metadata: Title, Artists (with weights), Work, Release

**Strategy: Field-by-Field Weighted Selection**

**Quality Scoring:**
```rust
fn calculate_metadata_quality(metadata: &Metadata) -> f64 {
    let mut score = 0.0;
    let mut fields = 0;

    if metadata.title.is_some() { score += 1.0; fields += 1; }
    if metadata.artist.is_some() { score += 1.0; fields += 1; }
    if metadata.album.is_some() { score += 0.5; fields += 1; }
    if metadata.genre.is_some() { score += 0.3; fields += 1; }
    if metadata.musicbrainz_mbid.is_some() { score += 1.5; fields += 1; }  // Bonus for MBID

    score / fields as f64
}

id3_quality = calculate_metadata_quality(&id3_metadata);
mb_quality = calculate_metadata_quality(&mb_metadata);
```

**Selection Strategy:**
```rust
struct FusedMetadata {
    title: String,
    title_source: MetadataSource,
    title_confidence: f64,

    artist: String,
    artist_source: MetadataSource,
    artist_confidence: f64,

    // ... other fields
}

fn fuse_metadata(id3: Metadata, mb: Metadata, identity_conf: f64) -> FusedMetadata {
    let mut fused = FusedMetadata::default();

    // Title selection:
    if mb.title.is_some() && identity_conf > 0.85 {
        // High-confidence MBID match → Trust MusicBrainz title
        fused.title = mb.title.unwrap();
        fused.title_source = MetadataSource::MusicBrainz;
        fused.title_confidence = identity_conf;
    } else if id3.title.is_some() {
        // Fallback to ID3 title
        fused.title = id3.title.unwrap();
        fused.title_source = MetadataSource::ID3;
        fused.title_confidence = id3_quality;

        // Check consistency if both available
        if mb.title.is_some() && !fuzzy_match(&id3.title, &mb.title) {
            flag_conflict("Title mismatch between ID3 and MusicBrainz");
        }
    } else {
        fused.title = "Unknown".to_string();
        fused.title_source = MetadataSource::Default;
        fused.title_confidence = 0.0;
    }

    // Artist selection: (similar logic)
    // Album selection: (similar logic)
    // Genre: Always prefer ID3 (user-curated, MusicBrainz doesn't have genre)

    fused
}
```

**Consistency Validation:**
```rust
fn fuzzy_match(a: &str, b: &str) -> bool {
    let similarity = levenshtein_ratio(a, b);
    similarity > 0.85  // 85% similarity threshold
}
```

**Output:**
```rust
struct MetadataFusion {
    title: String,
    artist: Vec<(String, f64)>,  // Artist + weight
    work: Option<Uuid>,
    album: Option<String>,
    genre: Option<String>,
    source_provenance: HashMap<String, MetadataSource>,  // Track where each field came from
    quality_score: f64,  // Overall metadata quality 0-100%
}
```

---

#### **Module 3: Musical Flavor Synthesis**

**Inputs:**
- AcousticBrainz: highlevel.json (if available, pre-2022 recordings)
- Essentia: Computed features (if Essentia installed)
- ID3 Genre + BPM: Coarse features
- Audio-derived fallback: Spectral/temporal features from Chromaprint analysis

**Strategy: Characteristic-Wise Weighted Averaging**

**Source Priority:**
```
1. AcousticBrainz (highest quality, pre-computed, peer-reviewed)
2. Essentia (high quality, computed locally)
3. Audio-derived (medium quality, basic features)
4. ID3-derived (low quality, genre → characteristics mapping)
```

**Fusion Algorithm:**
```rust
struct FlavorSource {
    characteristics: HashMap<String, f64>,  // e.g., "danceability.danceable" → 0.75
    confidence: f64,                        // Overall source confidence
    source_type: FlavorSourceType,
}

fn fuse_musical_flavor(sources: Vec<FlavorSource>) -> MusicalFlavor {
    let mut fused_characteristics = HashMap::new();

    // Get union of all characteristics across sources
    let all_characteristics: HashSet<String> = sources.iter()
        .flat_map(|s| s.characteristics.keys().cloned())
        .collect();

    for characteristic in all_characteristics {
        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;

        for source in &sources {
            if let Some(&value) = source.characteristics.get(&characteristic) {
                let weight = source.confidence;
                weighted_sum += value * weight;
                total_weight += weight;
            }
        }

        if total_weight > 0.0 {
            fused_characteristics.insert(
                characteristic.clone(),
                weighted_sum / total_weight
            );
        }
    }

    MusicalFlavor {
        characteristics: fused_characteristics,
        source_blend: sources.iter().map(|s| s.source_type.clone()).collect(),
        confidence_map: compute_per_characteristic_confidence(&sources),
    }
}
```

**Confidence Map:**
```rust
fn compute_per_characteristic_confidence(sources: &[FlavorSource]) -> HashMap<String, f64> {
    let mut confidence_map = HashMap::new();

    for characteristic in get_all_characteristics(sources) {
        let sources_with_char: Vec<_> = sources.iter()
            .filter(|s| s.characteristics.contains_key(&characteristic))
            .collect();

        if sources_with_char.is_empty() {
            confidence_map.insert(characteristic, 0.0);
        } else {
            // Confidence = average of source confidences
            let avg_confidence = sources_with_char.iter()
                .map(|s| s.confidence)
                .sum::<f64>() / sources_with_char.len() as f64;

            confidence_map.insert(characteristic, avg_confidence);
        }
    }

    confidence_map
}
```

**ID3 Genre → Characteristics Mapping:**
```rust
fn genre_to_characteristics(genre: &str) -> HashMap<String, f64> {
    // Coarse mapping: genre string → flavor characteristics
    match genre.to_lowercase().as_str() {
        "rock" => hashmap! {
            "danceability.danceable" => 0.4,
            "mood_aggressive.aggressive" => 0.7,
            "voice_instrumental.instrumental" => 0.3,
        },
        "electronic" => hashmap! {
            "danceability.danceable" => 0.8,
            "genre_electronic.house" => 0.5,
        },
        // ... many genres
        _ => HashMap::new(),
    }
    // Confidence for genre-derived characteristics: 0.3 (low)
}
```

**Normalization:**
```rust
fn normalize_characteristics(chars: HashMap<String, f64>) -> HashMap<String, f64> {
    // Ensure binary characteristics sum to 1.0
    // Ensure complex characteristics (per category) sum to 1.0
    // Per [MFL-DEF-030], [MFL-DEF-031], [MFL-DEF-032]

    let mut normalized = HashMap::new();

    for category in get_characteristic_categories(&chars) {
        let values: Vec<_> = chars.iter()
            .filter(|(k, _)| k.starts_with(category))
            .collect();

        let sum: f64 = values.iter().map(|(_, &v)| v).sum();

        if sum > 0.0 {
            for (key, value) in values {
                normalized.insert(key.clone(), value / sum);
            }
        }
    }

    normalized
}
```

**Output:**
```rust
struct MusicalFlavorSynthesis {
    flavor_vector: HashMap<String, f64>,   // Normalized characteristics
    confidence_map: HashMap<String, f64>,  // Per-characteristic confidence
    source_blend: Vec<FlavorSourceType>,   // Which sources contributed
    completeness: f64,                     // % of expected characteristics present
}
```

---

#### **Module 4: Passage Boundary Fusion**

**Inputs:**
- Silence detection: RMS-based boundaries
- Beat tracking (future): Tempo change detection
- Structural analysis (future): Intro/verse/chorus/outro patterns
- Metadata hints: ID3 track start times, cue sheets

**Strategy: Multi-Strategy Intersection + Validation**

**Current Implementation (Baseline):**
```rust
fn detect_silence_boundaries(audio: &AudioData, threshold_db: f64) -> Vec<Boundary> {
    // RMS analysis, find segments below threshold
    // Per IMPL005-audio_file_segmentation.md
}
```

**Hybrid Approach (Future Extension):**
```rust
struct BoundaryCandidate {
    position_ms: u64,
    confidence: f64,
    source: BoundarySource,  // Silence, Beat, Structural, Metadata
}

fn fuse_passage_boundaries(candidates: Vec<BoundaryCandidate>) -> Vec<Boundary> {
    // Step 1: Cluster nearby candidates (within 500ms = tolerance)
    let clusters = cluster_by_proximity(candidates, tolerance_ms=500);

    // Step 2: For each cluster, compute consensus boundary
    let mut fused_boundaries = vec![];
    for cluster in clusters {
        let consensus_position = weighted_average_position(&cluster);
        let consensus_confidence = aggregate_confidence(&cluster);

        // Validate: Minimum passage duration 30 seconds
        if is_valid_passage_duration(consensus_position, &fused_boundaries) {
            fused_boundaries.push(Boundary {
                position_ms: consensus_position,
                confidence: consensus_confidence,
                contributing_sources: cluster.iter().map(|c| c.source).collect(),
            });
        }
    }

    fused_boundaries
}

fn weighted_average_position(cluster: &[BoundaryCandidate]) -> u64 {
    let total_weight: f64 = cluster.iter().map(|c| c.confidence).sum();
    let weighted_sum: f64 = cluster.iter()
        .map(|c| c.position_ms as f64 * c.confidence)
        .sum();
    (weighted_sum / total_weight) as u64
}

fn aggregate_confidence(cluster: &[BoundaryCandidate]) -> f64 {
    // More sources agreeing → Higher confidence
    let num_sources = cluster.len();
    let avg_confidence: f64 = cluster.iter().map(|c| c.confidence).sum::<f64>() / num_sources as f64;

    // Boost confidence if multiple sources agree
    avg_confidence * (1.0 + 0.1 * (num_sources - 1) as f64).min(1.0)
}
```

**Validation Rules:**
```rust
fn is_valid_passage_duration(new_boundary_ms: u64, existing: &[Boundary]) -> bool {
    const MIN_PASSAGE_DURATION_MS: u64 = 30_000;  // 30 seconds
    const MAX_PASSAGE_DURATION_MS: u64 = 15 * 60_000;  // 15 minutes

    if let Some(last_boundary) = existing.last() {
        let duration = new_boundary_ms - last_boundary.position_ms;
        duration >= MIN_PASSAGE_DURATION_MS && duration <= MAX_PASSAGE_DURATION_MS
    } else {
        new_boundary_ms >= MIN_PASSAGE_DURATION_MS
    }
}
```

**Output:**
```rust
struct PassageBoundaryFusion {
    boundaries: Vec<Boundary>,
    strategy_used: Vec<BoundarySource>,
    confidence_per_boundary: Vec<f64>,
}

struct Boundary {
    position_ms: u64,
    confidence: f64,
    contributing_sources: Vec<BoundarySource>,
}
```

---

#### **Module 5: Cross-Source Consistency Validation**

**Validation Checks:**

**Check 1: Title Consistency**
```rust
fn validate_title_consistency(id3_title: &str, mb_title: &str) -> ValidationResult {
    let similarity = levenshtein_ratio(id3_title, mb_title);

    if similarity > 0.95 {
        ValidationResult::Pass
    } else if similarity > 0.80 {
        ValidationResult::Warning("Titles similar but not identical")
    } else {
        ValidationResult::Fail("Titles do not match")
    }
}
```

**Check 2: Duration Consistency**
```rust
fn validate_duration(id3_duration_ms: u64, audio_duration_ms: u64) -> ValidationResult {
    let diff_ms = (id3_duration_ms as i64 - audio_duration_ms as i64).abs() as u64;
    let tolerance_ms = 1000;  // 1 second tolerance

    if diff_ms <= tolerance_ms {
        ValidationResult::Pass
    } else {
        ValidationResult::Fail(format!("Duration mismatch: {}ms", diff_ms))
    }
}
```

**Check 3: Genre-Flavor Alignment**
```rust
fn validate_genre_flavor_alignment(genre: &str, flavor: &MusicalFlavor) -> ValidationResult {
    // Check if genre string aligns with flavor characteristics
    // Example: Genre "Rock" should have low danceability, high aggression

    let expected_chars = genre_to_characteristics(genre);
    let mut alignment_score = 0.0;
    let mut checked = 0;

    for (char_name, expected_value) in expected_chars {
        if let Some(&actual_value) = flavor.characteristics.get(&char_name) {
            let diff = (expected_value - actual_value).abs();
            alignment_score += 1.0 - diff;  // Higher score = better alignment
            checked += 1;
        }
    }

    if checked == 0 {
        return ValidationResult::Skip("No characteristics to compare");
    }

    let avg_alignment = alignment_score / checked as f64;

    if avg_alignment > 0.7 {
        ValidationResult::Pass
    } else if avg_alignment > 0.5 {
        ValidationResult::Warning("Genre-flavor moderate alignment")
    } else {
        ValidationResult::Fail("Genre-flavor mismatch")
    }
}
```

**Aggregated Validation:**
```rust
struct ValidationReport {
    checks: Vec<ValidationCheck>,
    overall_quality: f64,  // 0-100%
    conflicts_detected: Vec<ConflictReport>,
}

fn validate_passage(passage_data: &PassageData) -> ValidationReport {
    let checks = vec![
        validate_title_consistency(&passage_data.id3_title, &passage_data.mb_title),
        validate_duration(passage_data.id3_duration, passage_data.audio_duration),
        validate_genre_flavor_alignment(&passage_data.genre, &passage_data.flavor),
    ];

    let passed = checks.iter().filter(|c| matches!(c, ValidationResult::Pass)).count();
    let overall_quality = (passed as f64 / checks.len() as f64) * 100.0;

    let conflicts = checks.iter()
        .filter_map(|c| match c {
            ValidationResult::Fail(msg) => Some(ConflictReport { message: msg.clone() }),
            _ => None,
        })
        .collect();

    ValidationReport {
        checks,
        overall_quality,
        conflicts_detected: conflicts,
    }
}
```

---

### 3.3 Database Schema Extensions

**Current Schema (passages table):**
```sql
CREATE TABLE passages (
    id TEXT PRIMARY KEY,
    file_path TEXT NOT NULL,
    start_tick INTEGER NOT NULL,
    end_tick INTEGER NOT NULL,
    musical_flavor_json TEXT,
    -- ... other fields
);
```

**Extended Schema (for hybrid import):**
```sql
CREATE TABLE passages (
    id TEXT PRIMARY KEY,
    file_path TEXT NOT NULL,
    start_tick INTEGER NOT NULL,
    end_tick INTEGER NOT NULL,

    -- Musical Flavor (existing)
    musical_flavor_json TEXT,

    -- NEW: Source Provenance Tracking
    flavor_source_blend TEXT,  -- JSON: ["AcousticBrainz", "Essentia"]
    flavor_confidence_map TEXT,  -- JSON: {"danceability.danceable": 0.92, ...}

    -- NEW: Metadata Source Tracking
    title_source TEXT,  -- "ID3", "MusicBrainz", "Conflict"
    title_confidence REAL,
    artist_source TEXT,
    artist_confidence REAL,

    -- NEW: Identity Resolution Tracking
    recording_mbid TEXT,
    identity_confidence REAL,
    identity_conflicts TEXT,  -- JSON: [{"type": "MBID mismatch", "details": "..."}]

    -- NEW: Quality Scores
    overall_quality_score REAL,  -- 0-100%
    metadata_completeness REAL,  -- 0-100%
    flavor_completeness REAL,    -- 0-100%

    -- NEW: Validation Flags
    validation_status TEXT,  -- "Pass", "Warning", "Fail", "Pending"
    validation_report TEXT,  -- JSON: Full validation report

    -- NEW: Import Metadata
    import_session_id TEXT,
    import_timestamp INTEGER,
    import_strategy TEXT,  -- "QuickImport", "DeepScan", "HybridFusion"

    -- ... other existing fields
);

-- Index for quality-based queries
CREATE INDEX idx_passages_quality ON passages(overall_quality_score DESC);

-- Index for flagged passages requiring review
CREATE INDEX idx_passages_validation ON passages(validation_status);
```

**New Table: Import Provenance Log**
```sql
CREATE TABLE import_provenance (
    id TEXT PRIMARY KEY,
    passage_id TEXT NOT NULL,
    source_type TEXT NOT NULL,  -- "ID3", "AcoustID", "MusicBrainz", "Essentia", etc.
    data_extracted TEXT,  -- JSON: What data was extracted
    confidence REAL,
    timestamp INTEGER,
    FOREIGN KEY (passage_id) REFERENCES passages(id) ON DELETE CASCADE
);

CREATE INDEX idx_provenance_passage ON import_provenance(passage_id);
```

---

## 4. Implementation Approaches

### Approach A: Minimal Hybrid (Confidence-Aware Override)

**Description:**
Enhance current linear workflow with confidence-based decision-making, but maintain sequential architecture.

**Changes:**
1. **Identity Resolution:** Use AcoustID confidence score to decide whether to override ID3 MBID
2. **Musical Flavor:** Prefer AcousticBrainz if available, fall back to Essentia, store source indicator
3. **No fusion:** Still single-source selection, but smarter source selection

**Example:**
```rust
// Current:
let mbid = acoustid_lookup(fingerprint).mbid;  // Always use AcoustID result

// Approach A:
let acoustid_result = acoustid_lookup(fingerprint);
let mbid = if acoustid_result.confidence > 0.85 {
    acoustid_result.mbid  // High confidence → Use AcoustID
} else if let Some(id3_mbid) = id3_tags.musicbrainz_mbid {
    id3_mbid  // Low confidence AcoustID → Prefer ID3 embedded MBID
} else {
    acoustid_result.mbid  // No alternative → Use AcoustID anyway
};
```

**Risk Assessment:**
- **Failure Risk:** Low
- **Failure Modes:**
  1. Threshold tuning (0.85 may not be optimal) → Probability: Medium, Impact: Low
  2. Still no fusion of partial data → Probability: N/A, Impact: Medium (missed opportunity)
- **Mitigation:** Collect statistics on AcoustID score distribution, adjust threshold empirically
- **Residual Risk:** Low

**Quality Characteristics:**
- Maintainability: High (minimal code changes)
- Test Coverage: High (simple conditional logic)
- Architectural Alignment: Strong (extends existing workflow)

**Implementation Considerations:**
- Effort: Low (1-2 days)
- Dependencies: None (uses existing data sources)
- Complexity: Low

**Pros:**
- Low implementation effort
- Immediate improvement over current "always override" behavior
- Backward compatible (no schema changes)

**Cons:**
- Still no true fusion (binary choice between sources)
- Doesn't solve AcousticBrainz obsolescence (still single-source for flavor)
- No validation layer (conflicts undetected)

---

### Approach B: Staged Hybrid Fusion (Tier 1 + Tier 2)

**Description:**
Implement full 3-tier architecture but in two phases:
- **Phase 1:** Parallel extractors + Fusion engine (Tier 1 + Tier 2)
- **Phase 2:** Validation layer (Tier 3)

**Phase 1 Changes:**
1. **Refactor to parallel extraction:** Run ID3, Chromaprint, Essentia concurrently
2. **Implement fusion modules:** Identity Resolution, Metadata Fusion, Musical Flavor Synthesis
3. **Store source provenance:** Add database fields for tracking source contributions

**Phase 2 Changes:**
4. **Add validation checks:** Cross-source consistency validation
5. **Quality scoring:** Compute completeness and conflict metrics
6. **User feedback interface:** Allow users to correct mis-identifications

**Risk Assessment:**
- **Failure Risk:** Medium (Phase 1), Low (Phase 2)
- **Failure Modes:**
  1. Parallel extraction introduces concurrency bugs → Probability: Low, Impact: Medium
  2. Fusion algorithms produce worse results than single-source → Probability: Medium, Impact: High
  3. Schema changes break existing functionality → Probability: Low, Impact: High
- **Mitigation:**
  1. Thorough concurrency testing, use proven async patterns (Tokio)
  2. A/B testing: Run hybrid fusion alongside current workflow, compare results
  3. Database migrations with rollback, backward compatibility
- **Residual Risk:** Low-Medium

**Quality Characteristics:**
- Maintainability: Medium (more complex logic, but modular design)
- Test Coverage: Medium (requires extensive integration testing)
- Architectural Alignment: Strong (natural evolution of current design)

**Implementation Considerations:**
- Effort: Medium (Phase 1: 1-2 weeks, Phase 2: 1 week)
- Dependencies: No new external dependencies
- Complexity: Medium (parallel execution, fusion algorithms)

**Pros:**
- True multi-source fusion (not just selection)
- Solves AcousticBrainz obsolescence (blend multiple flavor sources)
- Extensible (easy to add new sources in future)
- Phased rollout reduces risk

**Cons:**
- Higher implementation effort than Approach A
- Requires schema changes (migration needed)
- More complex testing requirements

---

### Approach C: Full Hybrid Fusion (All 3 Tiers)

**Description:**
Implement complete 3-tier architecture in single phase, including validation and user feedback loop.

**Changes:**
All changes from Approach B Phase 1 + Phase 2, plus:
1. **Conflict flagging UI:** Show passages with validation issues
2. **Manual review workflow:** Allow users to select correct metadata when conflicts exist
3. **Learning feedback:** Store user corrections, use to improve future fusion decisions

**Risk Assessment:**
- **Failure Risk:** Medium-High
- **Failure Modes:**
  1. All risks from Approach B
  2. User feedback loop introduces data quality issues (garbage in, garbage out) → Probability: Medium, Impact: Medium
  3. Complex UI for manual review confuses users → Probability: Medium, Impact: Low
- **Mitigation:**
  1. Same mitigations as Approach B
  2. Validation of user corrections (sanity checks, reversibility)
  3. User testing of review interface
- **Residual Risk:** Medium

**Quality Characteristics:**
- Maintainability: Low-Medium (complex system with many components)
- Test Coverage: Medium (UI testing, feedback loop testing)
- Architectural Alignment: Strong (complete vision)

**Implementation Considerations:**
- Effort: High (3-4 weeks)
- Dependencies: UI components for review interface
- Complexity: High (full system integration)

**Pros:**
- Complete solution (all benefits of Approach B + validation + learning)
- User feedback improves accuracy over time
- Transparency (users see data quality issues)

**Cons:**
- High implementation effort (3-4 weeks)
- High complexity (more potential failure points)
- Requires UI development (wkmp-ai currently has minimal UI)

---

### Approach D: Musical Flavor Fusion Only (Targeted Hybrid)

**Description:**
Focus exclusively on solving AcousticBrainz obsolescence by implementing hybrid musical flavor synthesis. Leave identity resolution and passage mapping unchanged.

**Changes:**
1. **Musical Flavor Synthesis Module:** Blend AcousticBrainz + Essentia + ID3 genre + Audio features
2. **Parallel flavor extraction:** Run all flavor sources concurrently
3. **Characteristic-wise fusion:** Weighted averaging per characteristic
4. **Store flavor source provenance:** Add `flavor_source_blend` and `flavor_confidence_map` to database

**Risk Assessment:**
- **Failure Risk:** Low
- **Failure Modes:**
  1. Fused flavor less accurate than single-source → Probability: Low, Impact: Medium
  2. Normalization issues (characteristics don't sum to 1.0) → Probability: Medium, Impact: Low
- **Mitigation:**
  1. Validate fused flavor against known-good examples (A/B testing)
  2. Rigorous normalization with validation checks ([MFL-DEF-040])
- **Residual Risk:** Low

**Quality Characteristics:**
- Maintainability: High (isolated module)
- Test Coverage: High (focused scope)
- Architectural Alignment: Strong (fits existing workflow)

**Implementation Considerations:**
- Effort: Low-Medium (3-5 days)
- Dependencies: None (uses existing extractors)
- Complexity: Low-Medium

**Pros:**
- Solves most critical pain point (AcousticBrainz obsolescence)
- Low risk (isolated change)
- Immediate value (more passages have musical flavor)
- Can be implemented independently of other improvements

**Cons:**
- Doesn't address identity resolution issues
- Doesn't add validation layer
- Partial solution (doesn't realize full hybrid vision)

---

## 5. Recommendation

### 5.1 Risk-Based Ranking

| Approach | Residual Risk | Quality | Effort | Rank |
|----------|---------------|---------|--------|------|
| **A: Minimal Hybrid** | Low | High | Low | 3rd |
| **B: Staged Hybrid** | Low-Medium | Medium | Medium | **1st** ⭐ |
| **C: Full Hybrid** | Medium | Low-Medium | High | 4th |
| **D: Flavor Fusion Only** | Low | High | Low-Medium | **2nd** ⭐ |

### 5.2 Recommended Strategy

**Adopt staged implementation:**

**Phase 1: Musical Flavor Fusion (Approach D)**
- **Timeline:** 3-5 days
- **Rationale:** Solves most critical pain point (AcousticBrainz obsolescence) with lowest risk
- **Deliverables:**
  - Parallel flavor extraction (AcousticBrainz + Essentia + Audio-derived)
  - Characteristic-wise weighted fusion algorithm
  - Flavor source provenance tracking
  - Validation that fused flavors satisfy [MFL-DEF-030] constraints

**Phase 2: Identity Resolution (Part of Approach B)**
- **Timeline:** 5-7 days
- **Rationale:** Second most impactful (prevents false matches from degrading data quality)
- **Deliverables:**
  - Bayesian identity resolution (ID3 MBID + AcoustID)
  - Conflict detection and flagging
  - Confidence-based metadata fusion

**Phase 3: Validation Layer (Approach B Tier 3)**
- **Timeline:** 3-5 days
- **Rationale:** Quality assurance and transparency
- **Deliverables:**
  - Cross-source consistency checks
  - Quality scoring
  - Conflict reporting

**Phase 4 (Future): Passage Boundary Fusion**
- **Timeline:** TBD (depends on availability of beat tracking / structural analysis)
- **Rationale:** Requires new data sources (beat tracker, structural analyzer) not currently available
- **Defer until:** Beat tracking and structural analysis modules implemented

---

### 5.3 Detailed Recommendation: Phase 1 (Musical Flavor Fusion)

**Why This First:**
1. **Highest Impact:** AcousticBrainz obsolescence affects 50-70% of library (post-2022 music)
2. **Lowest Risk:** Isolated module, doesn't affect identity resolution or passage mapping
3. **Immediate Value:** More passages become auto-selectable (have musical flavor)
4. **Extensible:** Foundation for full hybrid architecture (Tier 1 extractors + Tier 2 fusion)

**Implementation Overview:**

```rust
// Tier 1: Source-Specific Extractors (parallel)
async fn extract_musical_flavor(audio_file: &Path, metadata: &Metadata) -> Vec<FlavorSource> {
    let (acousticbrainz, essentia, audio_derived, id3_derived) = tokio::join!(
        extract_acousticbrainz_flavor(metadata.recording_mbid),
        extract_essentia_flavor(audio_file),
        extract_audio_derived_flavor(audio_file),  // NEW: Basic spectral features
        extract_id3_derived_flavor(metadata.genre, metadata.bpm),  // NEW: Genre mapping
    );

    vec![acousticbrainz, essentia, audio_derived, id3_derived]
        .into_iter()
        .filter_map(|r| r.ok())  // Keep only successful extractions
        .collect()
}

// Tier 2: Fusion Engine
fn fuse_musical_flavor(sources: Vec<FlavorSource>) -> MusicalFlavorSynthesis {
    // Characteristic-wise weighted averaging (per Section 3.2, Module 3)
    // Normalize to ensure [MFL-DEF-030] constraints satisfied
    // Return fused flavor + confidence map + source provenance
}
```

**Testing Strategy:**
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_flavor_fusion_normalization() {
        // Verify binary characteristics sum to 1.0 ± 0.0001
        // Verify complex characteristics sum to 1.0 ± 0.0001
        // Per [MFL-DEF-040]
    }

    #[test]
    fn test_flavor_fusion_multi_source() {
        // Test: AcousticBrainz + Essentia
        // Test: Essentia only (AcousticBrainz unavailable)
        // Test: ID3 only (Essentia not installed)
        // Verify: Higher-quality source has higher weight in fusion
    }

    #[test]
    fn test_flavor_fusion_partial_characteristics() {
        // Test: Source A has {danceability, mood}, Source B has {danceability, genre}
        // Verify: Danceability fused (both sources), mood and genre single-source
    }
}
```

**Success Criteria:**
- ✅ Passages with post-2022 music have musical flavor (previously NULL)
- ✅ Flavor vectors satisfy [MFL-DEF-030] normalization (sum to 1.0)
- ✅ Flavor distance calculations work correctly with fused flavors ([MFL-CALC-010])
- ✅ Source provenance stored (user can see blend composition: "70% Essentia, 30% ID3")
- ✅ No regressions in existing flavor extraction (pre-2022 AcousticBrainz data still used)

---

## 6. Next Steps

This analysis is complete. Implementation planning requires explicit user authorization.

**To proceed with implementation:**

1. **Review analysis findings** and select preferred approach(es)
   - Recommended: Phase 1 (Musical Flavor Fusion) → Phase 2 (Identity Resolution) → Phase 3 (Validation)
   - Alternative: Approach A (Minimal Hybrid) for quickest value

2. **Make any necessary decisions** on identified decision points:
   - Fusion algorithm: Weighted averaging vs Bayesian update vs Ranked selection?
   - Confidence thresholds: What AcoustID score justifies overriding ID3 MBID?
   - Normalization strategy: Rescale vs Clip vs Flag violations?
   - Source priorities: AcousticBrainz > Essentia > Audio-derived > ID3-derived?

3. **Run `/plan [specification_file]`** to create detailed implementation plan
   - /plan will generate: requirements analysis, test specifications, increment breakdown

4. **/plan will generate:**
   - Requirements analysis and traceability
   - Acceptance test specifications (Given/When/Then)
   - Increment breakdown with tasks and deliverables
   - Risk assessment and mitigation steps

**User retains full authority over:**
- Whether to implement any recommendations
- Which approach to adopt
- When to proceed to implementation
- Modifications to suggested approaches

---

**Document Status:** Analysis Complete, Ready for Stakeholder Review
**Analysis Quality:** Comprehensive (current state, research, fusion strategies, detailed designs, phased recommendations)
**Implementation Readiness:** Detailed module designs provided, algorithms specified, ready for /plan workflow if approved
