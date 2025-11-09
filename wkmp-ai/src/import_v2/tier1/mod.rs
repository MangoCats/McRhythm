// PLAN023 Tier 1: Independent Source Extractors
//
// Each module in this tier is an independent "concept" (Legible Software principle).
// Extractors run in parallel and return results with confidence scores.
//
// Contract: All extractors implement similar pattern:
//   input: SongContext (audio + metadata)
//   output: ExtractorResult<T> (data + confidence + source)

// Implemented extractors (Legible Software: Independent Concepts)
pub mod audio_loader;           // ✅ Complete: Symphonia-based PCM extraction
pub mod genre_mapper;           // ✅ Complete: 25 genres, fuzzy matching
pub mod chromaprint_analyzer;   // ✅ Complete: Audio fingerprinting
pub mod audio_features;         // ✅ Complete: Signal-derived characteristics
pub mod id3_extractor;          // ✅ Complete: ID3 tag reading
pub mod acoustid_client;        // ✅ Complete: AcoustID API queries
pub mod musicbrainz_client;     // ✅ Complete: MusicBrainz metadata lookup

// TODO: Remaining extractors for full implementation
// pub mod essentia_analyzer;   // Extract musical flavor via Essentia (optional)
