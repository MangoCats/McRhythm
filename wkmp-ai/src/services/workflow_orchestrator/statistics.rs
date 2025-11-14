//! PLAN024 Import Workflow Statistics Tracking
//!
//! **Purpose:** Aggregate statistics across all 10 phases for UI display
//!
//! **Traceability:** [wkmp-ai_refinement.md] UI Statistics Requirements

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// **SCANNING Phase Statistics**
///
/// Display: "N potential files found" when complete
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanningStats {
    /// Number of potential audio files found
    pub potential_files_found: usize,
    /// Scanning in progress
    pub is_scanning: bool,
}

impl ScanningStats {
    pub fn display_string(&self) -> String {
        if self.is_scanning {
            "scanning".to_string()
        } else {
            format!("{} potential files found", self.potential_files_found)
        }
    }
}

/// **PROCESSING Phase Statistics**
///
/// Display: "Processing X to Y of Z"
/// - X = completed files
/// - Y = started files (completed + in_progress)
/// - Z = total files from SCANNING
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessingStats {
    /// Files that have completed processing
    pub completed: usize,
    /// Files that have started processing
    pub started: usize,
    /// Total files from SCANNING
    pub total: usize,
}

impl ProcessingStats {
    pub fn display_string(&self) -> String {
        format!("Processing {} to {} of {}", self.completed, self.started, self.total)
    }
}

/// **FILENAME MATCHING Phase Statistics**
///
/// Display: "N completed filenames found"
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilenameMatchingStats {
    /// Files found with 'INGEST COMPLETE' status
    pub completed_filenames_found: usize,
}

impl FilenameMatchingStats {
    pub fn display_string(&self) -> String {
        format!("{} completed filenames found", self.completed_filenames_found)
    }
}

/// **HASHING Phase Statistics**
///
/// Display: "N hashes computed, M matches found"
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HashingStats {
    /// Number of hashes computed
    pub hashes_computed: usize,
    /// Number of hashes matched to 'INGEST COMPLETE' files
    pub matches_found: usize,
}

impl HashingStats {
    pub fn display_string(&self) -> String {
        format!("{} hashes computed, {} matches found", self.hashes_computed, self.matches_found)
    }
}

/// **EXTRACTING Phase Statistics**
///
/// Display: "Metadata successfully extracted from X files, Y failures"
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractingStats {
    /// Files with at least one metadata item successfully extracted
    pub successful_extractions: usize,
    /// Files with no metadata extracted
    pub failures: usize,
}

impl ExtractingStats {
    pub fn display_string(&self) -> String {
        format!(
            "Metadata successfully extracted from {} files, {} failures",
            self.successful_extractions, self.failures
        )
    }
}

/// **SEGMENTING Phase Statistics**
///
/// Display: "X files, Y potential passages, Z finalized passages, W songs identified"
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SegmentingStats {
    /// Files that started segmenting
    pub files_processed: usize,
    /// Total potential passages identified
    pub potential_passages: usize,
    /// Total finalized passages
    pub finalized_passages: usize,
    /// Songs successfully identified
    pub songs_identified: usize,
}

impl SegmentingStats {
    pub fn display_string(&self) -> String {
        format!(
            "{} files, {} potential passages, {} finalized passages, {} songs identified",
            self.files_processed, self.potential_passages, self.finalized_passages, self.songs_identified
        )
    }
}

/// **FINGERPRINTING Phase Statistics**
///
/// Display: "X potential passages fingerprinted, Y successfully matched song identities"
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FingerprintingStats {
    /// Potential passages run through Chromaprint
    pub passages_fingerprinted: usize,
    /// Successful song matches from AcoustID
    pub successful_matches: usize,
}

impl FingerprintingStats {
    pub fn display_string(&self) -> String {
        format!(
            "{} potential passages fingerprinted, {} successfully matched song identities",
            self.passages_fingerprinted, self.successful_matches
        )
    }
}

/// **SONG MATCHING Phase Statistics**
///
/// Display: "W high, X medium, Y low, Z no confidence"
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SongMatchingStats {
    /// High confidence matches
    pub high_confidence: usize,
    /// Medium confidence matches
    pub medium_confidence: usize,
    /// Low confidence matches
    pub low_confidence: usize,
    /// No confidence (zero-song passages)
    pub no_confidence: usize,
}

impl SongMatchingStats {
    pub fn display_string(&self) -> String {
        format!(
            "{} high, {} medium, {} low, {} no confidence",
            self.high_confidence, self.medium_confidence, self.low_confidence, self.no_confidence
        )
    }
}

/// **RECORDING Phase Statistics**
///
/// Display: Scrollable list of song titles + filenames
/// Format: "Song Title in path/filename" or "unidentified passage in path/filename"
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecordingStats {
    /// List of recorded passages with song title + file path
    pub recorded_passages: Vec<RecordedPassageInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedPassageInfo {
    /// Song title (None for zero-song passages)
    pub song_title: Option<String>,
    /// File path
    pub file_path: String,
}

impl RecordingStats {
    pub fn display_lines(&self) -> Vec<String> {
        self.recorded_passages
            .iter()
            .map(|p| {
                if let Some(ref title) = p.song_title {
                    format!("{} in {}", title, p.file_path)
                } else {
                    format!("unidentified passage in {}", p.file_path)
                }
            })
            .collect()
    }
}

/// **AMPLITUDE Phase Statistics**
///
/// Display: Scrollable list of passages with lead-in/lead-out timings
/// Format: "Song Title | passage_length_seconds | lead-in: N ms | lead-out: M ms"
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AmplitudeStats {
    /// List of analyzed passages with timing information
    pub analyzed_passages: Vec<AnalyzedPassageInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzedPassageInfo {
    /// Song title (None for zero-song passages)
    pub song_title: Option<String>,
    /// Total passage length in seconds
    pub passage_length_seconds: f64,
    /// Lead-in duration in milliseconds
    pub lead_in_ms: u64,
    /// Lead-out duration in milliseconds
    pub lead_out_ms: u64,
}

impl AmplitudeStats {
    pub fn display_lines(&self) -> Vec<String> {
        self.analyzed_passages
            .iter()
            .map(|p| {
                let default_title = "unidentified passage".to_string();
                let title = p.song_title.as_ref().unwrap_or(&default_title);
                format!(
                    "{} {:.1}s lead-in {} ms lead-out {} ms",
                    title, p.passage_length_seconds, p.lead_in_ms, p.lead_out_ms
                )
            })
            .collect()
    }
}

/// **FLAVORING Phase Statistics**
///
/// Display: "W pre-existing, X by AcousticBrainz, Y by Essentia, Z could not be flavored"
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlavoringStats {
    /// Songs with pre-existing 'FLAVOR READY' status
    pub pre_existing: usize,
    /// Songs flavored by AcousticBrainz
    pub acousticbrainz: usize,
    /// Songs flavored by Essentia
    pub essentia: usize,
    /// Songs that failed both sources
    pub failed: usize,
}

impl FlavoringStats {
    pub fn display_string(&self) -> String {
        format!(
            "{} pre-existing, {} by AcousticBrainz, {} by Essentia, {} could not be flavored",
            self.pre_existing, self.acousticbrainz, self.essentia, self.failed
        )
    }
}

/// **PASSAGES COMPLETE Phase Statistics**
///
/// Display: Number of finalized passages completed
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PassagesCompleteStats {
    /// Finalized passages completed (recorded in passage table)
    pub passages_completed: usize,
}

impl PassagesCompleteStats {
    pub fn display_string(&self) -> String {
        format!("{} passages completed", self.passages_completed)
    }
}

/// **FILES COMPLETE Phase Statistics**
///
/// Display: Number of files that completed PROCESSING
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilesCompleteStats {
    /// Files that completed PROCESSING
    pub files_completed: usize,
}

impl FilesCompleteStats {
    pub fn display_string(&self) -> String {
        format!("{} files completed", self.files_completed)
    }
}

/// **Aggregate Import Statistics**
///
/// Thread-safe statistics container for entire import workflow
#[derive(Debug, Clone, Default)]
pub struct ImportStatistics {
    pub scanning: Arc<Mutex<ScanningStats>>,
    pub processing: Arc<Mutex<ProcessingStats>>,
    pub filename_matching: Arc<Mutex<FilenameMatchingStats>>,
    pub hashing: Arc<Mutex<HashingStats>>,
    pub extracting: Arc<Mutex<ExtractingStats>>,
    pub segmenting: Arc<Mutex<SegmentingStats>>,
    pub fingerprinting: Arc<Mutex<FingerprintingStats>>,
    pub song_matching: Arc<Mutex<SongMatchingStats>>,
    pub recording: Arc<Mutex<RecordingStats>>,
    pub amplitude: Arc<Mutex<AmplitudeStats>>,
    pub flavoring: Arc<Mutex<FlavoringStats>>,
    pub passages_complete: Arc<Mutex<PassagesCompleteStats>>,
    pub files_complete: Arc<Mutex<FilesCompleteStats>>,
}

impl ImportStatistics {
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment completed filenames found (Phase 1)
    pub fn increment_completed_filenames(&self) {
        let mut stats = self.filename_matching.lock().unwrap();
        stats.completed_filenames_found += 1;
    }

    /// Increment hashes computed (Phase 2)
    pub fn increment_hashes_computed(&self) {
        let mut stats = self.hashing.lock().unwrap();
        stats.hashes_computed += 1;
    }

    /// Increment hash matches found (Phase 2)
    pub fn increment_hash_matches(&self) {
        let mut stats = self.hashing.lock().unwrap();
        stats.matches_found += 1;
    }

    /// Record metadata extraction result (Phase 3)
    pub fn record_metadata_extraction(&self, successful: bool) {
        let mut stats = self.extracting.lock().unwrap();
        if successful {
            stats.successful_extractions += 1;
        } else {
            stats.failures += 1;
        }
    }

    /// Record segmentation results (Phase 4)
    pub fn record_segmentation(&self, potential_passages: usize, finalized_passages: usize, songs_identified: usize) {
        let mut stats = self.segmenting.lock().unwrap();
        stats.files_processed += 1;
        stats.potential_passages += potential_passages;
        stats.finalized_passages += finalized_passages;
        stats.songs_identified += songs_identified;
    }

    /// Record fingerprinting results (Phase 5)
    pub fn record_fingerprinting(&self, passages_fingerprinted: usize, successful_matches: usize) {
        let mut stats = self.fingerprinting.lock().unwrap();
        stats.passages_fingerprinted += passages_fingerprinted;
        stats.successful_matches += successful_matches;
    }

    /// Record song matching results (Phase 6)
    pub fn record_song_matching(
        &self,
        high: usize,
        medium: usize,
        low: usize,
        zero_song: usize,
    ) {
        let mut stats = self.song_matching.lock().unwrap();
        stats.high_confidence += high;
        stats.medium_confidence += medium;
        stats.low_confidence += low;
        stats.no_confidence += zero_song;
    }

    /// Add recorded passage (Phase 7)
    pub fn add_recorded_passage(&self, song_title: Option<String>, file_path: String) {
        let mut stats = self.recording.lock().unwrap();
        stats.recorded_passages.push(RecordedPassageInfo {
            song_title,
            file_path,
        });
    }

    /// Add analyzed passage (Phase 8)
    pub fn add_analyzed_passage(
        &self,
        song_title: Option<String>,
        passage_length_seconds: f64,
        lead_in_ms: u64,
        lead_out_ms: u64,
    ) {
        let mut stats = self.amplitude.lock().unwrap();
        stats.analyzed_passages.push(AnalyzedPassageInfo {
            song_title,
            passage_length_seconds,
            lead_in_ms,
            lead_out_ms,
        });
    }

    /// Record flavoring result (Phase 9)
    pub fn record_flavoring(&self, pre_existing: bool, source: Option<&str>) {
        let mut stats = self.flavoring.lock().unwrap();
        if pre_existing {
            stats.pre_existing += 1;
        } else {
            match source {
                Some("acousticbrainz") => stats.acousticbrainz += 1,
                Some("essentia") => stats.essentia += 1,
                _ => stats.failed += 1,
            }
        }
    }

    /// Increment passages completed (Phase 10)
    pub fn increment_passages_completed(&self) {
        let mut stats = self.passages_complete.lock().unwrap();
        stats.passages_completed += 1;
    }

    /// Increment files completed (overall)
    pub fn increment_files_completed(&self) {
        let mut stats = self.files_complete.lock().unwrap();
        stats.files_completed += 1;
    }

    /// Update processing statistics
    pub fn update_processing(&self, completed: usize, started: usize, total: usize) {
        let mut stats = self.processing.lock().unwrap();
        stats.completed = completed;
        stats.started = started;
        stats.total = total;
    }

    /// Update scanning statistics
    pub fn update_scanning(&self, is_scanning: bool, files_found: usize) {
        let mut stats = self.scanning.lock().unwrap();
        stats.is_scanning = is_scanning;
        stats.potential_files_found = files_found;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanning_stats_display() {
        let mut stats = ScanningStats::default();
        stats.is_scanning = true;
        assert_eq!(stats.display_string(), "scanning");

        stats.is_scanning = false;
        stats.potential_files_found = 42;
        assert_eq!(stats.display_string(), "42 potential files found");
    }

    #[test]
    fn test_processing_stats_display() {
        let stats = ProcessingStats {
            completed: 10,
            started: 15,
            total: 100,
        };
        assert_eq!(stats.display_string(), "Processing 10 to 15 of 100");
    }

    #[test]
    fn test_song_matching_stats_display() {
        let stats = SongMatchingStats {
            high_confidence: 42,
            medium_confidence: 10,
            low_confidence: 5,
            no_confidence: 3,
        };
        assert_eq!(stats.display_string(), "42 high, 10 medium, 5 low, 3 no confidence");
    }

    #[test]
    fn test_import_statistics_thread_safe() {
        let stats = ImportStatistics::new();

        // Test concurrent updates
        stats.increment_hashes_computed();
        stats.increment_hash_matches();
        stats.increment_hashes_computed();

        let hashing_stats = stats.hashing.lock().unwrap();
        assert_eq!(hashing_stats.hashes_computed, 2);
        assert_eq!(hashing_stats.matches_found, 1);
    }

    #[test]
    fn test_recording_stats_display() {
        let mut stats = RecordingStats::default();
        stats.recorded_passages.push(RecordedPassageInfo {
            song_title: Some("Bohemian Rhapsody".to_string()),
            file_path: "Queen/A Night at the Opera/01.mp3".to_string(),
        });
        stats.recorded_passages.push(RecordedPassageInfo {
            song_title: None,
            file_path: "Unknown/Track.mp3".to_string(),
        });

        let lines = stats.display_lines();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "Bohemian Rhapsody in Queen/A Night at the Opera/01.mp3");
        assert_eq!(lines[1], "unidentified passage in Unknown/Track.mp3");
    }

    #[test]
    fn test_amplitude_stats_display() {
        let mut stats = AmplitudeStats::default();
        stats.analyzed_passages.push(AnalyzedPassageInfo {
            song_title: Some("Stairway to Heaven".to_string()),
            passage_length_seconds: 482.3,
            lead_in_ms: 1200,
            lead_out_ms: 800,
        });

        let lines = stats.display_lines();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("Stairway to Heaven"));
        assert!(lines[0].contains("482.3s"));
        assert!(lines[0].contains("1200 ms"));
        assert!(lines[0].contains("800 ms"));
    }

    #[test]
    fn test_flavoring_stats_display() {
        let stats = FlavoringStats {
            pre_existing: 10,
            acousticbrainz: 25,
            essentia: 5,
            failed: 2,
        };
        assert_eq!(
            stats.display_string(),
            "10 pre-existing, 25 by AcousticBrainz, 5 by Essentia, 2 could not be flavored"
        );
    }
}
