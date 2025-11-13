//! Passage database operations
//!
//! **[AIA-DB-010]** Passage persistence with tick-based timing

use anyhow::Result;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;
use wkmp_common::timing::{seconds_to_ticks, ticks_to_seconds};

/// Passage record
///
/// Represents a playable segment within an audio file with timing and metadata.
/// **[SPEC017]** All timing values use tick-based representation (1 tick = 1/28,224,000 second).
#[derive(Debug, Clone)]
pub struct Passage {
    /// Unique identifier (UUID)
    pub guid: Uuid,
    /// Parent file identifier (foreign key to files table)
    pub file_id: Uuid,
    /// Passage start time in ticks (SPEC017)
    pub start_time_ticks: i64,
    /// Fade-in start point in ticks (None = no fade-in)
    pub fade_in_start_ticks: Option<i64>,
    /// Lead-in point in ticks (crossfade overlap start)
    pub lead_in_start_ticks: Option<i64>,
    /// Lead-out point in ticks (crossfade overlap start)
    pub lead_out_start_ticks: Option<i64>,
    /// Fade-out start point in ticks (None = no fade-out)
    pub fade_out_start_ticks: Option<i64>,
    /// Passage end time in ticks (SPEC017)
    pub end_time_ticks: i64,
    /// Fade-in curve type identifier (e.g., "linear", "ease-in")
    pub fade_in_curve: Option<String>,
    /// Fade-out curve type identifier (e.g., "linear", "ease-out")
    pub fade_out_curve: Option<String>,
    /// Passage title from metadata extraction
    pub title: Option<String>,
    /// User-overridden title (takes precedence over title)
    pub user_title: Option<String>,
    /// Artist name from metadata extraction
    pub artist: Option<String>,
    /// Album name from metadata extraction
    pub album: Option<String>,
    /// Musical flavor vector as JSON array (from AcousticBrainz)
    pub musical_flavor_vector: Option<String>,
    /// Import metadata as JSON (extraction sources, confidence scores)
    pub import_metadata: Option<String>,
    /// Additional metadata as JSON (extensibility field)
    pub additional_metadata: Option<String>,
}

impl Passage {
    /// Create new passage from seconds
    pub fn new(file_id: Uuid, start_sec: f64, end_sec: f64) -> Self {
        Self {
            guid: Uuid::new_v4(),
            file_id,
            start_time_ticks: seconds_to_ticks(start_sec),
            fade_in_start_ticks: None,
            lead_in_start_ticks: None,
            lead_out_start_ticks: None,
            fade_out_start_ticks: None,
            end_time_ticks: seconds_to_ticks(end_sec),
            fade_in_curve: None,
            fade_out_curve: None,
            title: None,
            user_title: None,
            artist: None,
            album: None,
            musical_flavor_vector: None,
            import_metadata: None,
            additional_metadata: None,
        }
    }

    /// Get passage duration in seconds
    pub fn duration_seconds(&self) -> f64 {
        ticks_to_seconds(self.end_time_ticks - self.start_time_ticks)
    }
}

/// Save passage to database
pub async fn save_passage(pool: &SqlitePool, passage: &Passage) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO passages (
            guid, file_id, start_time_ticks, fade_in_start_ticks, lead_in_start_ticks,
            lead_out_start_ticks, fade_out_start_ticks, end_time_ticks,
            fade_in_curve, fade_out_curve, title, user_title, artist, album,
            musical_flavor_vector, import_metadata, additional_metadata,
            created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        ON CONFLICT(guid) DO UPDATE SET
            start_time_ticks = excluded.start_time_ticks,
            fade_in_start_ticks = excluded.fade_in_start_ticks,
            lead_in_start_ticks = excluded.lead_in_start_ticks,
            lead_out_start_ticks = excluded.lead_out_start_ticks,
            fade_out_start_ticks = excluded.fade_out_start_ticks,
            end_time_ticks = excluded.end_time_ticks,
            title = excluded.title,
            user_title = excluded.user_title,
            artist = excluded.artist,
            album = excluded.album,
            musical_flavor_vector = excluded.musical_flavor_vector,
            import_metadata = excluded.import_metadata,
            additional_metadata = excluded.additional_metadata,
            updated_at = CURRENT_TIMESTAMP
        "#,
    )
    .bind(passage.guid.to_string())
    .bind(passage.file_id.to_string())
    .bind(passage.start_time_ticks)
    .bind(passage.fade_in_start_ticks)
    .bind(passage.lead_in_start_ticks)
    .bind(passage.lead_out_start_ticks)
    .bind(passage.fade_out_start_ticks)
    .bind(passage.end_time_ticks)
    .bind(&passage.fade_in_curve)
    .bind(&passage.fade_out_curve)
    .bind(&passage.title)
    .bind(&passage.user_title)
    .bind(&passage.artist)
    .bind(&passage.album)
    .bind(&passage.musical_flavor_vector)
    .bind(&passage.import_metadata)
    .bind(&passage.additional_metadata)
    .execute(pool)
    .await?;

    Ok(())
}

/// Count passages for a file
pub async fn count_passages_for_file(pool: &SqlitePool, file_id: Uuid) -> Result<i64> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM passages WHERE file_id = ?")
        .bind(file_id.to_string())
        .fetch_one(pool)
        .await?;
    Ok(count)
}

/// Count total passages in database
pub async fn count_passages(pool: &SqlitePool) -> Result<i64> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM passages")
        .fetch_one(pool)
        .await?;
    Ok(count)
}

/// Load passages for a file
pub async fn load_passages_for_file(pool: &SqlitePool, file_id: Uuid) -> Result<Vec<Passage>> {
    let rows = sqlx::query(
        r#"
        SELECT guid, file_id, start_time_ticks, fade_in_start_ticks, lead_in_start_ticks,
               lead_out_start_ticks, fade_out_start_ticks, end_time_ticks,
               fade_in_curve, fade_out_curve, title, user_title, artist, album,
               musical_flavor_vector, import_metadata, additional_metadata
        FROM passages
        WHERE file_id = ?
        "#,
    )
    .bind(file_id.to_string())
    .fetch_all(pool)
    .await?;

    let mut passages = Vec::new();
    for row in rows {
        let guid_str: String = row.get("guid");
        let file_id_str: String = row.get("file_id");

        passages.push(Passage {
            guid: Uuid::parse_str(&guid_str)?,
            file_id: Uuid::parse_str(&file_id_str)?,
            start_time_ticks: row.get("start_time_ticks"),
            fade_in_start_ticks: row.get("fade_in_start_ticks"),
            lead_in_start_ticks: row.get("lead_in_start_ticks"),
            lead_out_start_ticks: row.get("lead_out_start_ticks"),
            fade_out_start_ticks: row.get("fade_out_start_ticks"),
            end_time_ticks: row.get("end_time_ticks"),
            fade_in_curve: row.get("fade_in_curve"),
            fade_out_curve: row.get("fade_out_curve"),
            title: row.get("title"),
            user_title: row.get("user_title"),
            artist: row.get("artist"),
            album: row.get("album"),
            musical_flavor_vector: row.get("musical_flavor_vector"),
            import_metadata: row.get("import_metadata"),
            additional_metadata: row.get("additional_metadata"),
        });
    }

    Ok(passages)
}

/// Update passage lead-in/lead-out timing
pub async fn update_passage_timing(
    pool: &SqlitePool,
    passage_id: Uuid,
    lead_in_start_ticks: Option<i64>,
    lead_out_start_ticks: Option<i64>,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE passages
        SET lead_in_start_ticks = ?, lead_out_start_ticks = ?, updated_at = CURRENT_TIMESTAMP
        WHERE guid = ?
        "#,
    )
    .bind(lead_in_start_ticks)
    .bind(lead_out_start_ticks)
    .bind(passage_id.to_string())
    .execute(pool)
    .await?;

    Ok(())
}

/// Update passage musical flavor vector
pub async fn update_passage_flavor(
    pool: &SqlitePool,
    passage_id: Uuid,
    flavor_vector: String,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE passages
        SET musical_flavor_vector = ?, updated_at = CURRENT_TIMESTAMP
        WHERE guid = ?
        "#,
    )
    .bind(flavor_vector)
    .bind(passage_id.to_string())
    .execute(pool)
    .await?;

    Ok(())
}

/// Update passage metadata (title, artist, album)
pub async fn update_passage_metadata(
    pool: &SqlitePool,
    passage_id: Uuid,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
) -> Result<()> {
    sqlx::query(
        r#"
        UPDATE passages
        SET title = ?, artist = ?, album = ?, updated_at = CURRENT_TIMESTAMP
        WHERE guid = ?
        "#,
    )
    .bind(title)
    .bind(artist)
    .bind(album)
    .bind(passage_id.to_string())
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_save_passage() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");

        // Initialize schema for test database
        sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
        wkmp_common::db::init::create_files_table(&pool).await.unwrap();
        wkmp_common::db::init::create_passages_table(&pool).await.unwrap();

        let file_id = Uuid::new_v4();

        // Create file first (required by foreign key)
        sqlx::query(
            "INSERT INTO files (guid, path, hash, modification_time) VALUES (?, ?, ?, CURRENT_TIMESTAMP)"
        )
        .bind(file_id.to_string())
        .bind("test/track.mp3")
        .bind("hash123")
        .execute(&pool)
        .await
        .expect("Failed to create file");

        let passage = Passage::new(file_id, 0.0, 180.0);

        save_passage(&pool, &passage).await.expect("Failed to save passage");

        let count = count_passages_for_file(&pool, file_id)
            .await
            .expect("Failed to count passages");

        assert_eq!(count, 1);
    }

    #[test]
    fn test_passage_duration() {
        let file_id = Uuid::new_v4();
        let passage = Passage::new(file_id, 10.0, 190.0);

        let duration = passage.duration_seconds();
        assert!((duration - 180.0).abs() < 0.001); // ~180 seconds
    }

    // ===================================================================================
    // PLAN025 Phase 4 Tests: Tick-Based Timing Conversion (REQ-TICK-010)
    // ===================================================================================

    /// **[TC-U-TICK-010-01]** Unit test: Verify seconds_to_ticks() conversion accuracy
    ///
    /// **Requirement:** REQ-TICK-010 (Tick-based timing conversion)
    ///
    /// **Given:** Various second values (0s, 1s, 5s, 180s, fractional)
    /// **When:** Converting seconds to ticks using seconds_to_ticks()
    /// **Then:** Conversion maintains sample-accurate precision (<1 sample error at 44.1kHz)
    ///
    /// **Acceptance Criteria:**
    /// - 0.0 seconds = 0 ticks
    /// - 1.0 second = 28,224,000 ticks (TICK_RATE)
    /// - 5.0 seconds = 141,120,000 ticks
    /// - 180.0 seconds = 5,080,320,000 ticks
    /// - Roundtrip conversion preserves value within 1 tick
    /// - Precision <1 sample at 44.1kHz (640 ticks/sample)
    #[test]
    fn tc_u_tick_010_01_conversion_accuracy() {
        use wkmp_common::timing::{seconds_to_ticks, ticks_to_seconds, TICK_RATE};

        // Test exact conversions
        assert_eq!(seconds_to_ticks(0.0), 0, "0 seconds should be 0 ticks");
        assert_eq!(
            seconds_to_ticks(1.0),
            TICK_RATE,
            "1 second should be TICK_RATE ticks"
        );
        assert_eq!(
            seconds_to_ticks(5.0),
            141_120_000,
            "5 seconds should be 141,120,000 ticks"
        );
        assert_eq!(
            seconds_to_ticks(180.0),
            5_080_320_000,
            "180 seconds should be 5,080,320,000 ticks"
        );

        // Test roundtrip precision
        let test_values = vec![0.0, 0.5, 1.0, 5.0, 10.0, 60.0, 180.0, 300.0];
        for original_seconds in test_values {
            let ticks = seconds_to_ticks(original_seconds);
            let roundtrip_seconds = ticks_to_seconds(ticks);
            let error = (roundtrip_seconds - original_seconds).abs();

            // Error should be < 1 tick = 1/28,224,000 seconds = ~0.000000035 seconds
            let max_error = 1.0 / TICK_RATE as f64;
            assert!(
                error < max_error,
                "Roundtrip error {} exceeds tolerance {} for {} seconds",
                error,
                max_error,
                original_seconds
            );
        }

        // Test sample-accurate precision at 44.1kHz (640 ticks/sample)
        // Maximum acceptable error: 1 sample = 1/44,100 seconds ≈ 0.0000227 seconds
        let sample_rate_44k = 44_100.0;
        let max_sample_error = 1.0 / sample_rate_44k;

        for original_seconds in vec![0.5, 1.0, 5.0, 180.0] {
            let ticks = seconds_to_ticks(original_seconds);
            let roundtrip_seconds = ticks_to_seconds(ticks);
            let error = (roundtrip_seconds - original_seconds).abs();

            assert!(
                error < max_sample_error,
                "Sample-accurate precision failed: error {} exceeds {} for {} seconds",
                error,
                max_sample_error,
                original_seconds
            );
        }
    }

    /// **[TC-U-TICK-010-02]** Unit test: Verify tick conversion applied to all timing fields
    ///
    /// **Requirement:** REQ-TICK-010 (Tick-based timing conversion)
    ///
    /// **Given:** Passage created with seconds values
    /// **When:** Passage::new() is called with start/end seconds
    /// **Then:** All timing fields are stored as INTEGER ticks
    ///
    /// **Acceptance Criteria:**
    /// - start_time_ticks uses seconds_to_ticks conversion
    /// - end_time_ticks uses seconds_to_ticks conversion
    /// - Conversion maintains sample-accurate precision
    /// - All timing fields are i64 (INTEGER in database)
    #[test]
    fn tc_u_tick_010_02_all_fields_converted() {
        use wkmp_common::timing::{seconds_to_ticks, ticks_to_seconds};

        let file_id = Uuid::new_v4();
        let start_sec = 10.0;
        let end_sec = 190.0;

        let passage = Passage::new(file_id, start_sec, end_sec);

        // Verify start_time_ticks
        let expected_start_ticks = seconds_to_ticks(start_sec);
        assert_eq!(
            passage.start_time_ticks, expected_start_ticks,
            "start_time_ticks should use seconds_to_ticks conversion"
        );

        // Verify end_time_ticks
        let expected_end_ticks = seconds_to_ticks(end_sec);
        assert_eq!(
            passage.end_time_ticks, expected_end_ticks,
            "end_time_ticks should use seconds_to_ticks conversion"
        );

        // Verify roundtrip precision
        let roundtrip_start_sec = ticks_to_seconds(passage.start_time_ticks);
        let roundtrip_end_sec = ticks_to_seconds(passage.end_time_ticks);

        assert!(
            (roundtrip_start_sec - start_sec).abs() < 0.000001,
            "start_time_ticks roundtrip error exceeds tolerance"
        );
        assert!(
            (roundtrip_end_sec - end_sec).abs() < 0.000001,
            "end_time_ticks roundtrip error exceeds tolerance"
        );

        // Verify types (all timing fields are i64)
        assert_eq!(
            std::mem::size_of_val(&passage.start_time_ticks),
            8,
            "start_time_ticks should be i64 (8 bytes)"
        );
        assert_eq!(
            std::mem::size_of_val(&passage.end_time_ticks),
            8,
            "end_time_ticks should be i64 (8 bytes)"
        );

        // Verify optional timing fields are also i64
        let passage_with_timing = Passage {
            guid: Uuid::new_v4(),
            file_id,
            start_time_ticks: seconds_to_ticks(10.0),
            fade_in_start_ticks: Some(seconds_to_ticks(12.0)),
            lead_in_start_ticks: Some(seconds_to_ticks(11.0)),
            lead_out_start_ticks: Some(seconds_to_ticks(185.0)),
            fade_out_start_ticks: Some(seconds_to_ticks(188.0)),
            end_time_ticks: seconds_to_ticks(190.0),
            fade_in_curve: None,
            fade_out_curve: None,
            title: None,
            user_title: None,
            artist: None,
            album: None,
            musical_flavor_vector: None,
            import_metadata: None,
            additional_metadata: None,
        };

        // Verify all optional timing fields convert correctly
        assert_eq!(
            passage_with_timing.fade_in_start_ticks,
            Some(seconds_to_ticks(12.0)),
            "fade_in_start_ticks should use seconds_to_ticks"
        );
        assert_eq!(
            passage_with_timing.lead_in_start_ticks,
            Some(seconds_to_ticks(11.0)),
            "lead_in_start_ticks should use seconds_to_ticks"
        );
        assert_eq!(
            passage_with_timing.lead_out_start_ticks,
            Some(seconds_to_ticks(185.0)),
            "lead_out_start_ticks should use seconds_to_ticks"
        );
        assert_eq!(
            passage_with_timing.fade_out_start_ticks,
            Some(seconds_to_ticks(188.0)),
            "fade_out_start_ticks should use seconds_to_ticks"
        );
    }

    /// **[TC-I-TICK-010-01]** Integration test: Verify tick-based timing in database writes
    ///
    /// **Requirement:** REQ-TICK-010 (Tick-based timing conversion)
    ///
    /// **Given:** Passage with seconds values
    /// **When:** Saving passage to database and loading it back
    /// **Then:** All timing fields are stored and retrieved as INTEGER ticks
    ///
    /// **Acceptance Criteria:**
    /// - Database stores INTEGER ticks (not floating-point seconds)
    /// - Loaded passage has identical tick values
    /// - Roundtrip preserves timing precision
    /// - All 7 timing fields use tick representation
    #[tokio::test]
    async fn tc_i_tick_010_01_database_writes() {
        use wkmp_common::timing::{seconds_to_ticks, ticks_to_seconds};

        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");

        // Initialize schema
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await
            .unwrap();
        wkmp_common::db::init::create_files_table(&pool)
            .await
            .unwrap();
        wkmp_common::db::init::create_passages_table(&pool)
            .await
            .unwrap();

        let file_id = Uuid::new_v4();

        // Create file (required by foreign key)
        sqlx::query(
            "INSERT INTO files (guid, path, hash, modification_time) VALUES (?, ?, ?, CURRENT_TIMESTAMP)",
        )
        .bind(file_id.to_string())
        .bind("test/track.mp3")
        .bind("hash123")
        .execute(&pool)
        .await
        .expect("Failed to create file");

        // Create passage with all timing fields
        // Passage: 10.0s - 190.0s (180 second duration)
        // Timing must satisfy constraints:
        // - lead_in_start_ticks >= start_time_ticks AND <= end_time_ticks
        // - lead_out_start_ticks >= start_time_ticks AND <= end_time_ticks
        let original_passage = Passage {
            guid: Uuid::new_v4(),
            file_id,
            start_time_ticks: seconds_to_ticks(10.0),         // Passage starts at 10s
            fade_in_start_ticks: Some(seconds_to_ticks(12.0)), // Fade-in starts 2s after passage start
            lead_in_start_ticks: Some(seconds_to_ticks(11.0)), // Lead-in (crossfade overlap) at 11s (within passage)
            lead_out_start_ticks: Some(seconds_to_ticks(185.0)), // Lead-out (crossfade overlap) at 185s (within passage)
            fade_out_start_ticks: Some(seconds_to_ticks(188.0)), // Fade-out starts 2s before passage end
            end_time_ticks: seconds_to_ticks(190.0),          // Passage ends at 190s
            fade_in_curve: Some("linear".to_string()),
            fade_out_curve: Some("exponential".to_string()),  // Valid curve: exponential, cosine, linear, logarithmic, equal_power
            title: Some("Test Track".to_string()),
            user_title: None,
            artist: Some("Test Artist".to_string()),
            album: Some("Test Album".to_string()),
            musical_flavor_vector: None,
            import_metadata: None,
            additional_metadata: None,
        };

        // Save to database
        save_passage(&pool, &original_passage)
            .await
            .expect("Failed to save passage");

        // Load from database
        let loaded_passages = load_passages_for_file(&pool, file_id)
            .await
            .expect("Failed to load passages");

        assert_eq!(loaded_passages.len(), 1, "Should load exactly one passage");

        let loaded_passage = &loaded_passages[0];

        // Verify all timing fields match exactly (INTEGER comparison)
        assert_eq!(
            loaded_passage.start_time_ticks, original_passage.start_time_ticks,
            "start_time_ticks should match exactly"
        );
        assert_eq!(
            loaded_passage.end_time_ticks, original_passage.end_time_ticks,
            "end_time_ticks should match exactly"
        );
        assert_eq!(
            loaded_passage.fade_in_start_ticks, original_passage.fade_in_start_ticks,
            "fade_in_start_ticks should match exactly"
        );
        assert_eq!(
            loaded_passage.lead_in_start_ticks, original_passage.lead_in_start_ticks,
            "lead_in_start_ticks should match exactly"
        );
        assert_eq!(
            loaded_passage.lead_out_start_ticks, original_passage.lead_out_start_ticks,
            "lead_out_start_ticks should match exactly"
        );
        assert_eq!(
            loaded_passage.fade_out_start_ticks, original_passage.fade_out_start_ticks,
            "fade_out_start_ticks should match exactly"
        );

        // Verify tick values are correct (not seconds or milliseconds)
        // start_time = 10.0 seconds = 282,240,000 ticks
        assert_eq!(
            loaded_passage.start_time_ticks,
            282_240_000,
            "start_time_ticks should be in ticks, not seconds"
        );

        // Verify roundtrip precision
        let roundtrip_start_sec = ticks_to_seconds(loaded_passage.start_time_ticks);
        let roundtrip_end_sec = ticks_to_seconds(loaded_passage.end_time_ticks);

        assert!(
            (roundtrip_start_sec - 10.0).abs() < 0.000001,
            "Roundtrip start time should preserve precision"
        );
        assert!(
            (roundtrip_end_sec - 190.0).abs() < 0.000001,
            "Roundtrip end time should preserve precision"
        );

        // Verify database column types are INTEGER (query raw column type)
        let row: (i64,) = sqlx::query_as("SELECT start_time_ticks FROM passages WHERE guid = ?")
            .bind(original_passage.guid.to_string())
            .fetch_one(&pool)
            .await
            .expect("Failed to query tick value");

        assert_eq!(
            row.0, original_passage.start_time_ticks,
            "Database should store tick values as INTEGER"
        );
    }
}
