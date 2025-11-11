//! Audio metadata extraction service
//!
//! **[AIA-COMP-010]** Extract metadata from audio files using lofty
//!
//! Extracts:
//! - Artist, title, album
//! - Duration
//! - Track number, year
//! - File format

use lofty::file::{FileType, TaggedFileExt};
use lofty::prelude::*;
use lofty::probe::Probe;
use std::path::Path;
use thiserror::Error;

/// Metadata extraction errors
#[derive(Debug, Error)]
pub enum MetadataError {
    /// Failed to read audio file with Symphonia
    #[error("Failed to read file: {0}")]
    ReadError(String),

    /// Unsupported audio format
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    /// No ID3 or other metadata tags found in file
    #[error("No metadata found")]
    NoMetadata,

    /// I/O error (file read)
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Extracted audio metadata
#[derive(Debug, Clone)]
pub struct AudioMetadata {
    /// File path
    pub file_path: String,

    /// Artist name(s)
    pub artist: Option<String>,

    /// Track title
    pub title: Option<String>,

    /// Album title
    pub album: Option<String>,

    /// Track number
    pub track_number: Option<u32>,

    /// Release year
    pub year: Option<u32>,

    /// Duration in seconds
    pub duration_seconds: Option<f64>,

    /// Audio format (MP3, FLAC, etc.)
    pub format: String,

    /// Sample rate (Hz)
    pub sample_rate: Option<u32>,

    /// Bit depth
    pub bit_depth: Option<u8>,

    /// Number of channels
    pub channels: Option<u8>,

    /// Bitrate (kbps)
    pub bitrate: Option<u32>,

    /// File size in bytes
    pub file_size_bytes: u64,
}

/// Metadata extractor service
pub struct MetadataExtractor {}

impl MetadataExtractor {
    /// Create new metadata extractor
    pub fn new() -> Self {
        Self {}
    }

    /// Extract metadata from audio file
    ///
    /// **[AIA-COMP-010]** Parse audio tags and properties
    pub fn extract(&self, file_path: &Path) -> Result<AudioMetadata, MetadataError> {
        // Get file size
        let file_size_bytes = std::fs::metadata(file_path)?.len();

        // Probe the file to determine format
        let tagged_file = Probe::open(file_path)
            .map_err(|e| MetadataError::ReadError(e.to_string()))?
            .read()
            .map_err(|e| MetadataError::ReadError(e.to_string()))?;

        // Get audio properties
        let properties = tagged_file.properties();
        let duration_seconds = properties.duration().as_secs_f64();
        let sample_rate = properties.sample_rate();
        let bit_depth = properties.bit_depth();
        let channels = properties.channels();
        let bitrate = properties.audio_bitrate().map(|br| br / 1000); // Convert to kbps

        // Determine format
        let format = match tagged_file.file_type() {
            FileType::Mpeg => "MP3",
            FileType::Flac => "FLAC",
            FileType::Opus => "Opus",
            FileType::Vorbis => "OGG Vorbis",
            FileType::Aac => "AAC",
            FileType::Aiff => "AIFF",
            FileType::Wav => "WAV",
            FileType::WavPack => "WavPack",
            _ => "Unknown",
        }
        .to_string();

        // Try to get primary tag
        let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());

        let (artist, title, album, track_number, year) = if let Some(tag) = tag {
            let artist = tag.artist().map(|s| s.to_string());
            let title = tag.title().map(|s| s.to_string());
            let album = tag.album().map(|s| s.to_string());
            let track_number = tag.track();
            let year = tag.year();

            (artist, title, album, track_number, year)
        } else {
            (None, None, None, None, None)
        };

        tracing::debug!(
            file = %file_path.display(),
            artist = ?artist,
            title = ?title,
            duration_s = duration_seconds,
            format = %format,
            "Extracted metadata"
        );

        Ok(AudioMetadata {
            file_path: file_path.to_string_lossy().to_string(),
            artist,
            title,
            album,
            track_number,
            year,
            duration_seconds: Some(duration_seconds),
            format,
            sample_rate,
            bit_depth,
            channels,
            bitrate,
            file_size_bytes,
        })
    }

    /// Extract metadata from multiple files
    pub fn extract_batch(&self, file_paths: &[impl AsRef<Path>]) -> Vec<Result<AudioMetadata, MetadataError>> {
        file_paths
            .iter()
            .map(|path| self.extract(path.as_ref()))
            .collect()
    }
}

impl Default for MetadataExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extractor_creation() {
        let extractor = MetadataExtractor::new();
        assert_eq!(std::mem::size_of_val(&extractor), 0); // Zero-sized type
    }

    #[test]
    fn test_extract_nonexistent_file() {
        let extractor = MetadataExtractor::new();
        let result = extractor.extract(Path::new("/nonexistent/file.mp3"));
        assert!(result.is_err());
    }
}
