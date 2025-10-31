//! Database schema definitions for WKMP audio library
//!
//! **[AIA-DB-010]** Database integration for audio ingest
//! Per [IMPL001-database_schema.md](../../docs/IMPL001-database_schema.md)

use anyhow::Result;
use sqlx::SqlitePool;

/// Initialize database schema
///
/// Creates all required tables for audio library management:
/// - Core entities: files, passages, songs, artists, works, albums, images
/// - Linking tables: passage_songs, song_artists, passage_albums
/// - System tables: schema_version, users, settings
pub async fn initialize_schema(pool: &SqlitePool) -> Result<()> {
    // Execute schema in transaction for atomic creation
    let mut tx = pool.begin().await?;

    // Schema version tracking
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // Users table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            guid TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            password_salt TEXT NOT NULL,
            config_interface_access BOOLEAN NOT NULL DEFAULT 1,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // Settings table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // Files table - Audio files discovered by scanner
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS files (
            guid TEXT PRIMARY KEY,
            path TEXT NOT NULL UNIQUE,
            hash TEXT NOT NULL,
            duration REAL,
            modification_time TIMESTAMP NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            CHECK (duration IS NULL OR duration > 0)
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_files_path ON files(path)")
        .execute(&mut *tx)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_files_hash ON files(hash)")
        .execute(&mut *tx)
        .await?;

    // Passages table - Playable segments
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS passages (
            guid TEXT PRIMARY KEY,
            file_id TEXT NOT NULL REFERENCES files(guid) ON DELETE CASCADE,
            start_time_ticks INTEGER NOT NULL,
            fade_in_start_ticks INTEGER,
            lead_in_start_ticks INTEGER,
            lead_out_start_ticks INTEGER,
            fade_out_start_ticks INTEGER,
            end_time_ticks INTEGER NOT NULL,
            fade_in_curve TEXT,
            fade_out_curve TEXT,
            title TEXT,
            user_title TEXT,
            artist TEXT,
            album TEXT,
            musical_flavor_vector TEXT,
            import_metadata TEXT,
            additional_metadata TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            CHECK (start_time_ticks >= 0),
            CHECK (end_time_ticks > start_time_ticks)
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_passages_file ON passages(file_id)")
        .execute(&mut *tx)
        .await?;

    // Songs table - MusicBrainz recordings
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS songs (
            guid TEXT PRIMARY KEY,
            recording_mbid TEXT NOT NULL UNIQUE,
            work_id TEXT,
            related_songs TEXT,
            lyrics TEXT,
            base_probability REAL NOT NULL DEFAULT 1.0,
            min_cooldown INTEGER NOT NULL DEFAULT 604800,
            ramping_cooldown INTEGER NOT NULL DEFAULT 1209600,
            last_played_at TIMESTAMP,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            CHECK (base_probability >= 0.0 AND base_probability <= 1000.0),
            CHECK (min_cooldown >= 0),
            CHECK (ramping_cooldown >= 0)
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_songs_recording_mbid ON songs(recording_mbid)")
        .execute(&mut *tx)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_songs_last_played ON songs(last_played_at)")
        .execute(&mut *tx)
        .await?;

    // Artists table - Performing artists
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS artists (
            guid TEXT PRIMARY KEY,
            artist_mbid TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            base_probability REAL NOT NULL DEFAULT 1.0,
            min_cooldown INTEGER NOT NULL DEFAULT 7200,
            ramping_cooldown INTEGER NOT NULL DEFAULT 14400,
            last_played_at TIMESTAMP,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            CHECK (base_probability >= 0.0 AND base_probability <= 1000.0),
            CHECK (min_cooldown >= 0),
            CHECK (ramping_cooldown >= 0)
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_artists_mbid ON artists(artist_mbid)")
        .execute(&mut *tx)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_artists_last_played ON artists(last_played_at)")
        .execute(&mut *tx)
        .await?;

    // Works table - Musical works (compositions)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS works (
            guid TEXT PRIMARY KEY,
            work_mbid TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            base_probability REAL NOT NULL DEFAULT 1.0,
            min_cooldown INTEGER NOT NULL DEFAULT 259200,
            ramping_cooldown INTEGER NOT NULL DEFAULT 604800,
            last_played_at TIMESTAMP,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            CHECK (base_probability >= 0.0 AND base_probability <= 1000.0),
            CHECK (min_cooldown >= 0),
            CHECK (ramping_cooldown >= 0)
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_works_mbid ON works(work_mbid)")
        .execute(&mut *tx)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_works_last_played ON works(last_played_at)")
        .execute(&mut *tx)
        .await?;

    // Albums table - Albums/releases
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS albums (
            guid TEXT PRIMARY KEY,
            album_mbid TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            release_date TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_albums_mbid ON albums(album_mbid)")
        .execute(&mut *tx)
        .await?;

    // Images table - Cover art and entity images
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS images (
            guid TEXT PRIMARY KEY,
            file_path TEXT NOT NULL,
            image_type TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            priority INTEGER NOT NULL DEFAULT 100,
            width INTEGER,
            height INTEGER,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            CHECK (image_type IN ('album_front', 'album_back', 'album_liner', 'song', 'passage', 'artist', 'work', 'logo')),
            CHECK (priority >= 0)
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_images_entity ON images(entity_id, image_type, priority)",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_images_type ON images(image_type)")
        .execute(&mut *tx)
        .await?;

    // Linking table: passage_songs
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS passage_songs (
            passage_id TEXT NOT NULL REFERENCES passages(guid) ON DELETE CASCADE,
            song_id TEXT NOT NULL REFERENCES songs(guid) ON DELETE CASCADE,
            start_time_ticks INTEGER NOT NULL,
            end_time_ticks INTEGER NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (passage_id, song_id),
            CHECK (start_time_ticks >= 0),
            CHECK (end_time_ticks > start_time_ticks)
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_passage_songs_passage ON passage_songs(passage_id)",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_passage_songs_song ON passage_songs(song_id)")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_passage_songs_timing ON passage_songs(passage_id, start_time_ticks)",
    )
    .execute(&mut *tx)
    .await?;

    // Linking table: song_artists
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS song_artists (
            song_id TEXT NOT NULL REFERENCES songs(guid) ON DELETE CASCADE,
            artist_id TEXT NOT NULL REFERENCES artists(guid) ON DELETE CASCADE,
            weight REAL NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (song_id, artist_id),
            CHECK (weight > 0.0 AND weight <= 1.0)
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_song_artists_song ON song_artists(song_id)")
        .execute(&mut *tx)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_song_artists_artist ON song_artists(artist_id)")
        .execute(&mut *tx)
        .await?;

    // Linking table: passage_albums
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS passage_albums (
            passage_id TEXT NOT NULL REFERENCES passages(guid) ON DELETE CASCADE,
            album_id TEXT NOT NULL REFERENCES albums(guid) ON DELETE CASCADE,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (passage_id, album_id)
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_passage_albums_passage ON passage_albums(passage_id)")
        .execute(&mut *tx)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_passage_albums_album ON passage_albums(album_id)")
        .execute(&mut *tx)
        .await?;

    // AcoustID cache table - Caches fingerprint → MBID mappings
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS acoustid_cache (
            fingerprint_hash TEXT PRIMARY KEY,
            mbid TEXT NOT NULL,
            cached_at TEXT NOT NULL DEFAULT (datetime('now')),
            CHECK (length(fingerprint_hash) = 64)
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_acoustid_cache_cached_at ON acoustid_cache(cached_at)")
        .execute(&mut *tx)
        .await?;

    // Import sessions table (already exists from earlier, but include for completeness)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS import_sessions (
            session_id TEXT PRIMARY KEY,
            state TEXT NOT NULL,
            root_folder TEXT NOT NULL,
            parameters TEXT NOT NULL,
            progress_current INTEGER NOT NULL,
            progress_total INTEGER NOT NULL,
            progress_percentage REAL NOT NULL,
            current_operation TEXT NOT NULL,
            errors TEXT NOT NULL,
            started_at TEXT NOT NULL,
            ended_at TEXT
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // Temporary file-song mapping table for import workflow
    // Stores file → song relationships discovered during fingerprinting phase
    // Used to link passages to songs after passages are created in segmenting phase
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS temp_file_songs (
            file_id TEXT PRIMARY KEY REFERENCES files(guid) ON DELETE CASCADE,
            song_id TEXT NOT NULL REFERENCES songs(guid) ON DELETE CASCADE,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // Temporary file-album mapping table for import workflow
    // Stores file → album relationships discovered during fingerprinting phase
    // Used to link passages to albums after passages are created in segmenting phase
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS temp_file_albums (
            file_id TEXT NOT NULL REFERENCES files(guid) ON DELETE CASCADE,
            album_id TEXT NOT NULL REFERENCES albums(guid) ON DELETE CASCADE,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (file_id, album_id)
        )
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // Insert initial schema version
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO schema_version (version) VALUES (1)
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // Insert Anonymous user if not exists
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO users (guid, username, password_hash, password_salt, config_interface_access)
        VALUES ('00000000-0000-0000-0000-000000000001', 'Anonymous', '', '', 1)
        "#,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    tracing::info!("Database schema initialized successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_schema_initialization() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");

        initialize_schema(&pool).await.expect("Schema initialization failed");

        // Verify schema_version table exists
        let version: i64 = sqlx::query_scalar("SELECT version FROM schema_version WHERE version = 1")
            .fetch_one(&pool)
            .await
            .expect("Schema version not found");
        assert_eq!(version, 1);

        // Verify Anonymous user exists
        let username: String = sqlx::query_scalar("SELECT username FROM users WHERE guid = '00000000-0000-0000-0000-000000000001'")
            .fetch_one(&pool)
            .await
            .expect("Anonymous user not found");
        assert_eq!(username, "Anonymous");
    }
}
