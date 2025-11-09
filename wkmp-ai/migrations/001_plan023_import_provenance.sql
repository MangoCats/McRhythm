-- PLAN023: Database Migration - Import Provenance and Hybrid Fusion
--
-- Adds columns to `passages` table for storing PLAN023 import data:
-- - Identity resolution (MBID, confidence, conflicts)
-- - Metadata provenance (title/artist/album source + confidence)
-- - Musical flavor synthesis (source blend, confidence map)
-- - Validation results (quality scores, conflicts, warnings)
-- - Import metadata (session, timestamp, version, duration)
--
-- Creates `import_provenance` table for detailed per-field tracking.
--
-- Requirements: REQ-AI-081 through REQ-AI-087
-- Architecture: 3-Tier Hybrid Fusion (PLAN023)

-- ============================================================================
-- Add 21 columns to passages table
-- ============================================================================

-- Identity Resolution (REQ-AI-083) [3 columns]
ALTER TABLE passages ADD COLUMN recording_mbid TEXT;
ALTER TABLE passages ADD COLUMN identity_confidence REAL;
ALTER TABLE passages ADD COLUMN identity_conflicts TEXT;  -- JSON array

-- Metadata Provenance (REQ-AI-082) [6 columns]
ALTER TABLE passages ADD COLUMN title_source TEXT;
ALTER TABLE passages ADD COLUMN title_confidence REAL;
ALTER TABLE passages ADD COLUMN artist_source TEXT;
ALTER TABLE passages ADD COLUMN artist_confidence REAL;
ALTER TABLE passages ADD COLUMN album_source TEXT;
ALTER TABLE passages ADD COLUMN album_confidence REAL;

-- Musical Flavor Synthesis (REQ-AI-081) [2 columns]
ALTER TABLE passages ADD COLUMN flavor_source_blend TEXT;     -- JSON array
ALTER TABLE passages ADD COLUMN flavor_confidence_map TEXT;   -- JSON object

-- Quality Validation (REQ-AI-084, REQ-AI-085) [5 columns]
ALTER TABLE passages ADD COLUMN overall_quality_score REAL;
ALTER TABLE passages ADD COLUMN metadata_completeness REAL;
ALTER TABLE passages ADD COLUMN flavor_completeness REAL;
ALTER TABLE passages ADD COLUMN validation_status TEXT;
ALTER TABLE passages ADD COLUMN validation_report TEXT;  -- JSON object

-- Import Metadata (REQ-AI-086) [5 columns]
ALTER TABLE passages ADD COLUMN import_session_id TEXT;
ALTER TABLE passages ADD COLUMN import_timestamp INTEGER;
ALTER TABLE passages ADD COLUMN import_strategy TEXT;
ALTER TABLE passages ADD COLUMN import_duration_ms INTEGER;
ALTER TABLE passages ADD COLUMN import_version TEXT;

-- ============================================================================
-- Create import_provenance table (REQ-AI-087)
-- ============================================================================

CREATE TABLE IF NOT EXISTS import_provenance (
    id TEXT PRIMARY KEY,
    passage_id TEXT NOT NULL,
    source_type TEXT NOT NULL,  -- "ID3", "AcoustID", "MusicBrainz", "Essentia", "AudioDerived", "GenreMapping"
    data_extracted TEXT,         -- JSON blob of raw data extracted from source
    confidence REAL,             -- Confidence score from extractor
    timestamp INTEGER,           -- Unix timestamp when extraction occurred
    FOREIGN KEY (passage_id) REFERENCES passages(guid) ON DELETE CASCADE
);

CREATE INDEX idx_import_provenance_passage_id ON import_provenance(passage_id);
CREATE INDEX idx_import_provenance_source_type ON import_provenance(source_type);
CREATE INDEX idx_import_provenance_timestamp ON import_provenance(timestamp);

-- ============================================================================
-- Notes
-- ============================================================================

-- Column Count: 21 total columns added
--   - Identity: 3 columns (recording_mbid, identity_confidence, identity_conflicts)
--   - Metadata Provenance: 6 columns (title/artist/album source + confidence)
--   - Flavor: 2 columns (source_blend, confidence_map)
--   - Validation: 5 columns (quality_score, metadata_completeness, flavor_completeness, status, report)
--   - Import Metadata: 5 columns (session_id, timestamp, strategy, duration_ms, version)
--
-- Note: Original task description mentioned "13 columns" but actual requirements
-- (REQ-AI-081 through REQ-AI-086) and ProcessedPassage data structure specify
-- 21 columns to capture complete provenance data.
--
-- JSON Column Formats:
--
-- identity_conflicts: Array of conflict descriptions
--   ["High-confidence MBID conflict: ID3 vs AcoustID", "Artist name mismatch"]
--
-- flavor_source_blend: Array of contributing sources
--   ["Essentia", "AudioDerived", "GenreMapping"]
--
-- flavor_confidence_map: Object mapping characteristic names to confidence
--   {"danceability": 0.85, "energy": 0.92, "valence": 0.67}
--
-- validation_report: Full validation report object
--   {
--     "quality_score": 0.87,
--     "has_conflicts": false,
--     "warnings": ["Album field missing"],
--     "conflicts": []
--   }
--
-- All time values in passages table use INTEGER ticks per SPEC017 (28,224,000 Hz).
-- Import timestamps use standard Unix INTEGER seconds (not ticks).
