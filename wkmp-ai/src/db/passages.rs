//! Passage database operations
//!
//! **[AIA-DB-010]** Passage persistence with tick-based timing

use anyhow::Result;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;
use wkmp_common::timing::{seconds_to_ticks, ticks_to_seconds};

/// Passage record
#[derive(Debug, Clone)]
pub struct Passage {
    pub guid: Uuid,
    pub file_id: Uuid,
    pub start_time_ticks: i64,
    pub fade_in_start_ticks: Option<i64>,
    pub lead_in_start_ticks: Option<i64>,
    pub lead_out_start_ticks: Option<i64>,
    pub fade_out_start_ticks: Option<i64>,
    pub end_time_ticks: i64,
    pub fade_in_curve: Option<String>,
    pub fade_out_curve: Option<String>,
    pub title: Option<String>,
    pub user_title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub musical_flavor_vector: Option<String>,
    pub import_metadata: Option<String>,
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

        crate::db::schema::initialize_schema(&pool)
            .await
            .expect("Schema initialization failed");

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
}
