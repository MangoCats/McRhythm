# Increment 3: Metadata Extraction

**Estimated Effort:** 3 hours
**Dependencies:** Increment 1, 2
**Deliverables:** matching/metadata.rs

---

## Objective

Create metadata extraction module that reconciles ID3 tags, filename parsing, and path-based metadata.

---

## Source Files

| am28 File | Lines | Action |
|-----------|-------|--------|
| metadata.rs | 581 | Adapt, integrate with existing MetadataExtractor |

---

## Tasks

### 3.1 Create matching/metadata.rs

```rust
//! Metadata Extraction for Album Matching
//!
//! Extracts and reconciles artist/album from multiple sources:
//! - ID3 tags (primary)
//! - Filename parsing (fallback)
//! - Path analysis (last resort)

use std::path::Path;

/// Reconciled metadata from all sources
#[derive(Debug, Clone)]
pub struct ReconciledMetadata {
    /// Primary artist name
    pub artist: String,
    /// Primary album name
    pub album: String,
    /// Alternative artist (from different source)
    pub alternate_artist: Option<String>,
    /// Alternative album (from different source)
    pub alternate_album: Option<String>,
    /// Source of primary metadata
    pub source: MetadataSource,
    /// Confidence in metadata (0.0-1.0)
    pub confidence: f64,
}

/// Source of metadata
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataSource {
    /// From ID3 tags
    Id3Tags,
    /// From filename parsing
    Filename,
    /// From directory path
    Path,
    /// Unknown/default
    Unknown,
}

/// Extract and reconcile metadata from audio file
pub fn extract_and_reconcile_metadata(file_path: &Path) -> ReconciledMetadata {
    // 1. Try ID3 tags first
    if let Some(id3_meta) = extract_id3_metadata(file_path) {
        // 2. Also try filename for alternate
        let filename_meta = extract_filename_metadata(file_path);

        return ReconciledMetadata {
            artist: id3_meta.artist,
            album: id3_meta.album,
            alternate_artist: filename_meta.map(|m| m.artist),
            alternate_album: filename_meta.map(|m| m.album),
            source: MetadataSource::Id3Tags,
            confidence: 0.9,
        };
    }

    // 2. Fallback to filename
    if let Some(filename_meta) = extract_filename_metadata(file_path) {
        let path_meta = extract_path_metadata(file_path);

        return ReconciledMetadata {
            artist: filename_meta.artist,
            album: filename_meta.album,
            alternate_artist: path_meta.map(|m| m.artist),
            alternate_album: path_meta.map(|m| m.album),
            source: MetadataSource::Filename,
            confidence: 0.6,
        };
    }

    // 3. Last resort: path analysis
    if let Some(path_meta) = extract_path_metadata(file_path) {
        return ReconciledMetadata {
            artist: path_meta.artist,
            album: path_meta.album,
            alternate_artist: None,
            alternate_album: None,
            source: MetadataSource::Path,
            confidence: 0.3,
        };
    }

    // 4. Unknown
    ReconciledMetadata {
        artist: "Unknown Artist".to_string(),
        album: "Unknown Album".to_string(),
        alternate_artist: None,
        alternate_album: None,
        source: MetadataSource::Unknown,
        confidence: 0.0,
    }
}

struct RawMetadata {
    artist: String,
    album: String,
}

fn extract_id3_metadata(file_path: &Path) -> Option<RawMetadata> {
    // Use id3 crate (already a dependency)
    let tag = id3::Tag::read_from_path(file_path).ok()?;

    let artist = tag.artist()
        .or_else(|| tag.album_artist())
        .map(|s| s.to_string())?;

    let album = tag.album()
        .map(|s| s.to_string())?;

    if artist.is_empty() || album.is_empty() {
        return None;
    }

    Some(RawMetadata { artist, album })
}

fn extract_filename_metadata(file_path: &Path) -> Option<RawMetadata> {
    let filename = file_path.file_stem()?.to_str()?;

    // Common patterns:
    // "Artist - Album"
    // "Artist_-_Album"
    // "Artist – Album" (en-dash)

    let separators = [" - ", "_-_", " – ", " — "];

    for sep in separators {
        if let Some(idx) = filename.find(sep) {
            let artist = filename[..idx].trim().to_string();
            let album = filename[idx + sep.len()..].trim().to_string();

            if !artist.is_empty() && !album.is_empty() {
                return Some(RawMetadata { artist, album });
            }
        }
    }

    None
}

fn extract_path_metadata(file_path: &Path) -> Option<RawMetadata> {
    // Pattern: .../Artist/Album/file.mp3
    let parent = file_path.parent()?;
    let album = parent.file_name()?.to_str()?.to_string();

    let grandparent = parent.parent()?;
    let artist = grandparent.file_name()?.to_str()?.to_string();

    if artist.is_empty() || album.is_empty() {
        return None;
    }

    Some(RawMetadata { artist, album })
}
```

### 3.2 Add Module to matching/mod.rs

```rust
pub mod metadata;
pub use metadata::{ReconciledMetadata, MetadataSource, extract_and_reconcile_metadata};
```

---

## Tests

| Test ID | Description | Pass Criteria |
|---------|-------------|---------------|
| TC-U-003-01 | ID3 tag extraction from MP3 | Artist and album extracted correctly |
| TC-U-003-02 | Filename parsing (artist - album) | Splits on separator correctly |
| TC-U-003-03 | Metadata reconciliation priority | ID3 > Filename > Path |
| TC-U-003-04 | Unicode handling in metadata | Japanese/Korean characters preserved |

---

## Acceptance Criteria

- [ ] metadata.rs created
- [ ] ID3 extraction working
- [ ] Filename parsing working
- [ ] Path analysis working
- [ ] Reconciliation logic correct
- [ ] All 4 tests pass
