-- Migration 006: WKMP-AI Hybrid Fusion Provenance Tracking
-- PLAN023: WKMP-AI Ground-Up Recode - Database Extensions
-- Created: 2025-01-08
--
-- Purpose: Extend passages table with 13 new columns for 3-tier hybrid fusion provenance
--          and create import_provenance table for detailed source tracking.
--
-- Requirements Implemented:
--   REQ-AI-081: Flavor Source Provenance (flavor_source_blend, flavor_confidence_map, flavor_completeness)
--   REQ-AI-082: Metadata Source Provenance (title_source, title_confidence, artist_source, artist_confidence)
--   REQ-AI-083: Identity Resolution Tracking (recording_mbid, identity_confidence, identity_conflicts)
--   REQ-AI-084: Quality Scores (overall_quality_score, metadata_completeness)
--   REQ-AI-085: Validation Flags (validation_status, validation_report)
--   REQ-AI-086: Import Metadata (import_session_id, import_timestamp, import_strategy)
--   REQ-AI-087: Import Provenance Log Table

-- ============================================================================
-- PART 1: Extend passages table with 13 new provenance columns
-- ============================================================================

-- Flavor Source Provenance (REQ-AI-081)
ALTER TABLE passages ADD COLUMN flavor_source_blend TEXT;
-- JSON array of contributing sources, e.g., ["Essentia:0.9", "ID3Genre:0.3", "Audio:0.5"]

ALTER TABLE passages ADD COLUMN flavor_confidence_map TEXT;
-- JSON object mapping characteristics to confidence, e.g., {"danceability.danceable": 0.95, ...}

ALTER TABLE passages ADD COLUMN flavor_completeness REAL;
-- Completeness score 0.0-1.0 (present_characteristics / 18 expected)

-- Metadata Source Provenance (REQ-AI-082)
ALTER TABLE passages ADD COLUMN title_source TEXT;
-- Source of title: "ID3", "MusicBrainz", "Conflict"

ALTER TABLE passages ADD COLUMN title_confidence REAL;
-- Confidence score 0.0-1.0 for title

ALTER TABLE passages ADD COLUMN artist_source TEXT;
-- Source of artist: "ID3", "MusicBrainz", "Conflict"

ALTER TABLE passages ADD COLUMN artist_confidence REAL;
-- Confidence score 0.0-1.0 for artist

-- Identity Resolution Tracking (REQ-AI-083)
ALTER TABLE passages ADD COLUMN recording_mbid TEXT;
-- Final resolved MusicBrainz Recording MBID (UUID format)

ALTER TABLE passages ADD COLUMN identity_confidence REAL;
-- Posterior confidence from Bayesian update (0.0-1.0)

ALTER TABLE passages ADD COLUMN identity_conflicts TEXT;
-- JSON array of conflict reports, e.g., [{"source1": "ID3", "mbid1": "...", "source2": "AcoustID", "mbid2": "...", "levenshtein_similarity": 0.45}]

-- Quality Scores (REQ-AI-084)
ALTER TABLE passages ADD COLUMN overall_quality_score REAL;
-- Overall quality 0-100% from Tier 3 validation

ALTER TABLE passages ADD COLUMN metadata_completeness REAL;
-- Metadata completeness 0.0-1.0 (filled fields / total fields)

-- Validation Flags (REQ-AI-085)
ALTER TABLE passages ADD COLUMN validation_status TEXT;
-- Validation status: "Pass", "Warning", "Fail", "Pending"

ALTER TABLE passages ADD COLUMN validation_report TEXT;
-- JSON object with full validation report, e.g., {"title_check": {"pass": true, ...}, ...}

-- Import Metadata (REQ-AI-086)
ALTER TABLE passages ADD COLUMN import_session_id TEXT;
-- UUID for import session (groups all passages from single file import)

ALTER TABLE passages ADD COLUMN import_timestamp INTEGER;
-- Unix timestamp of import (seconds since epoch)

ALTER TABLE passages ADD COLUMN import_strategy TEXT;
-- Import strategy used: "HybridFusion" (for future mode support)

-- ============================================================================
-- PART 2: Create import_provenance table (REQ-AI-087)
-- ============================================================================

CREATE TABLE import_provenance (
    id TEXT PRIMARY KEY,
    -- UUID for this provenance entry

    passage_id TEXT NOT NULL,
    -- Foreign key to passages.guid

    source_type TEXT NOT NULL,
    -- Source extractor type: "ID3", "Chromaprint", "AcoustID", "MusicBrainz", "Essentia", "AudioDerived", "ID3Genre"

    data_extracted TEXT,
    -- JSON blob of data extracted from this source
    -- Example for ID3: {"title": "Breathe", "artist": "Pink Floyd", "mbid": "..."}
    -- Example for AcoustID: {"mbid": "...", "score": 0.95, "duration": 175.3}

    confidence REAL,
    -- Confidence score for this source extraction (0.0-1.0)

    timestamp INTEGER NOT NULL,
    -- Unix timestamp when extraction occurred

    FOREIGN KEY (passage_id) REFERENCES passages(guid) ON DELETE CASCADE
);

-- Index for efficient queries by passage
CREATE INDEX idx_import_provenance_passage_id ON import_provenance(passage_id);

-- Index for efficient queries by source type
CREATE INDEX idx_import_provenance_source_type ON import_provenance(source_type);

-- ============================================================================
-- PART 3: Update schema_version table
-- ============================================================================

UPDATE schema_version SET version = 6, applied_at = CURRENT_TIMESTAMP;

-- ============================================================================
-- Migration Complete
-- ============================================================================
