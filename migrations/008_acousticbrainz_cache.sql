-- Migration 008: AcousticBrainz Cache
--
-- **Purpose:** Cache AcousticBrainz musical flavor data to avoid re-querying API
--
-- **Rationale:**
-- - AcousticBrainz ceased accepting submissions in 2022 (data is static)
-- - 200-file test shows 91.1% coverage (2,427/2,664 recordings available)
-- - Rate-limited API (1 req/sec) makes repeated queries expensive
-- - Multiple passages may reference same recording MBID
--
-- **Cache Strategy:**
-- - Key: MusicBrainz recording MBID (unique identifier)
-- - Value: Full JSON response from AcousticBrainz "low-level" endpoint
-- - Cache indefinitely (data never changes after 2022)
-- - No expiration or invalidation needed
--
-- **Performance Impact:**
-- - Cache hit: <1ms database query
-- - Cache miss: ~1000ms API query + rate limiting
-- - Expected hit rate: 91.1% for popular music (per coverage analysis)

-- AcousticBrainz cache table
CREATE TABLE IF NOT EXISTS acousticbrainz_cache (
    -- MusicBrainz recording MBID (primary key)
    recording_mbid TEXT PRIMARY KEY NOT NULL,

    -- Full JSON response from AcousticBrainz "low-level" endpoint
    -- Contains: metadata, tonal, rhythm, lowlevel features
    lowlevel_json TEXT NOT NULL,

    -- Quick availability flags (extracted from JSON for fast filtering)
    has_tonal BOOLEAN NOT NULL DEFAULT 0,
    has_rhythm BOOLEAN NOT NULL DEFAULT 0,

    -- Cache metadata
    fetched_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    -- Essentia version (for future compatibility tracking)
    essentia_version TEXT,

    -- Constraints
    CHECK (length(recording_mbid) = 36),  -- UUID format
    CHECK (json_valid(lowlevel_json))     -- Ensure valid JSON
);

-- Index for timestamp-based queries (optional analytics)
CREATE INDEX IF NOT EXISTS idx_acousticbrainz_cache_fetched_at
    ON acousticbrainz_cache(fetched_at);

-- Index for availability filtering (find recordings with complete data)
CREATE INDEX IF NOT EXISTS idx_acousticbrainz_cache_availability
    ON acousticbrainz_cache(has_tonal, has_rhythm)
    WHERE has_tonal = 1 AND has_rhythm = 1;
