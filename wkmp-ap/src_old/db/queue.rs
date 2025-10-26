//! Queue database operations
//!
//! Provides CRUD operations for the queue table.
//!
//! **Traceability:**
//! - DB-QUEUE-010 (Queue table schema)
//! - SSD-ENG-020 (Queue processing)

use crate::error::{Error, Result};
use sqlx::{Pool, Sqlite};
use uuid::Uuid;
use wkmp_common::db::QueueEntry;

/// Get all queue entries ordered by play_order
///
/// **Traceability:** DB-QUEUE-020
pub async fn get_queue(db: &Pool<Sqlite>) -> Result<Vec<QueueEntry>> {
    let rows = sqlx::query_as::<_, QueueEntry>(
        r#"
        SELECT guid, file_path, passage_guid, play_order,
               start_time_ms, end_time_ms,
               lead_in_point_ms, lead_out_point_ms,
               fade_in_point_ms, fade_out_point_ms,
               fade_in_curve, fade_out_curve
        FROM queue
        ORDER BY play_order ASC
        "#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows)
}

/// Get a single queue entry by ID
///
/// **Traceability:** DB-QUEUE-025
pub async fn get_queue_entry_by_id(db: &Pool<Sqlite>, id: Uuid) -> Result<QueueEntry> {
    let entry = sqlx::query_as::<_, QueueEntry>(
        r#"
        SELECT guid, file_path, passage_guid, play_order,
               start_time_ms, end_time_ms,
               lead_in_point_ms, lead_out_point_ms,
               fade_in_point_ms, fade_out_point_ms,
               fade_in_curve, fade_out_curve
        FROM queue
        WHERE guid = ?
        "#,
    )
    .bind(id.to_string())
    .fetch_optional(db)
    .await?
    .ok_or_else(|| Error::Queue(format!("Queue entry not found: {}", id)))?;

    Ok(entry)
}

/// Calculate next available play_order value
///
/// Returns the next play_order value to use when adding entries.
/// Maintains gaps of 10 between entries for easier reordering.
///
/// **Traceability:** DB-QUEUE-030
pub async fn get_next_play_order(db: &Pool<Sqlite>) -> Result<i64> {
    let result = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT MAX(play_order) FROM queue"
    )
    .fetch_one(db)
    .await?;

    // If queue is empty, start at 10; otherwise add 10 to max
    Ok(result.unwrap_or(0) + 10)
}

/// Add a passage to the queue
///
/// If play_order is None, appends to end of queue.
/// If play_order is Some(n), inserts at that position (may renumber tail).
///
/// **Traceability:**
/// - DB-QUEUE-040 (Enqueue operation)
/// - DB-QUEUE-050 (Play order management)
#[allow(clippy::too_many_arguments)]
pub async fn enqueue(
    db: &Pool<Sqlite>,
    file_path: String,
    passage_guid: Option<Uuid>,
    play_order: Option<i64>,
    start_time_ms: Option<i64>,
    end_time_ms: Option<i64>,
    lead_in_point_ms: Option<i64>,
    lead_out_point_ms: Option<i64>,
    fade_in_point_ms: Option<i64>,
    fade_out_point_ms: Option<i64>,
    fade_in_curve: Option<String>,
    fade_out_curve: Option<String>,
) -> Result<Uuid> {
    let queue_entry_id = Uuid::new_v4();

    let final_play_order = match play_order {
        Some(order) => {
            // Check if we need to renumber tail entries
            // If there's already an entry at this position, shift everything after it
            let count: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM queue WHERE play_order >= ?"
            )
            .bind(order)
            .fetch_one(db)
            .await?;

            if count > 0 {
                // Renumber tail: add 10 to all entries at or after this position
                sqlx::query(
                    "UPDATE queue SET play_order = play_order + 10 WHERE play_order >= ?"
                )
                .bind(order)
                .execute(db)
                .await?;
            }

            order
        }
        None => {
            // Append to end
            get_next_play_order(db).await?
        }
    };

    // Insert new entry
    sqlx::query(
        r#"
        INSERT INTO queue (
            guid, file_path, passage_guid, play_order,
            start_time_ms, end_time_ms,
            lead_in_point_ms, lead_out_point_ms,
            fade_in_point_ms, fade_out_point_ms,
            fade_in_curve, fade_out_curve
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(queue_entry_id.to_string())
    .bind(file_path)
    .bind(passage_guid.map(|id| id.to_string()))
    .bind(final_play_order)
    .bind(start_time_ms)
    .bind(end_time_ms)
    .bind(lead_in_point_ms)
    .bind(lead_out_point_ms)
    .bind(fade_in_point_ms)
    .bind(fade_out_point_ms)
    .bind(fade_in_curve)
    .bind(fade_out_curve)
    .execute(db)
    .await?;

    Ok(queue_entry_id)
}

/// Remove a queue entry by ID
///
/// **Traceability:** DB-QUEUE-060
pub async fn remove_from_queue(db: &Pool<Sqlite>, queue_entry_id: Uuid) -> Result<()> {
    let result = sqlx::query("DELETE FROM queue WHERE guid = ?")
        .bind(queue_entry_id.to_string())
        .execute(db)
        .await?;

    if result.rows_affected() == 0 {
        return Err(Error::Queue(format!(
            "Queue entry not found: {}",
            queue_entry_id
        )));
    }

    Ok(())
}

/// Clear all entries from the queue
///
/// **Traceability:** DB-QUEUE-070
pub async fn clear_queue(db: &Pool<Sqlite>) -> Result<()> {
    sqlx::query("DELETE FROM queue")
        .execute(db)
        .await?;

    Ok(())
}

/// Reorder a queue entry to a new position
///
/// Changes the play_order of the specified entry.
/// May renumber other entries to maintain gaps.
///
/// **Traceability:** DB-QUEUE-080
pub async fn reorder_queue(
    db: &Pool<Sqlite>,
    queue_entry_id: Uuid,
    new_position: i32,
) -> Result<()> {
    // Convert new_position (0-based index) to play_order value
    // Get all entries to calculate correct play_order
    let entries = get_queue(db).await?;

    if new_position < 0 || new_position as usize >= entries.len() {
        return Err(Error::Queue(format!(
            "Invalid position: {} (queue has {} entries)",
            new_position,
            entries.len()
        )));
    }

    // Find the entry to move
    let _entry = get_queue_entry_by_id(db, queue_entry_id).await?;

    // Calculate target play_order based on position
    let target_play_order = if new_position == 0 {
        // Moving to front: use play_order before first entry
        entries[0].play_order - 10
    } else if new_position as usize == entries.len() - 1 {
        // Moving to end: use play_order after last entry
        entries[entries.len() - 1].play_order + 10
    } else {
        // Moving to middle: use average of neighbors
        let prev = &entries[new_position as usize - 1];
        let next = &entries[new_position as usize];
        (prev.play_order + next.play_order) / 2
    };

    // Check for overflow protection (renumber if approaching limits)
    if target_play_order > 2_000_000_000 || target_play_order < -2_000_000_000 {
        renumber_queue(db).await?;
        // After renumbering, recalculate target position
        // Avoid recursion - just renumber and continue
        let entries = get_queue(db).await?;

        let target_play_order = if new_position == 0 {
            entries[0].play_order - 10
        } else if new_position as usize == entries.len() - 1 {
            entries[entries.len() - 1].play_order + 10
        } else {
            let prev = &entries[new_position as usize - 1];
            let next = &entries[new_position as usize];
            (prev.play_order + next.play_order) / 2
        };

        sqlx::query("UPDATE queue SET play_order = ? WHERE guid = ?")
            .bind(target_play_order)
            .bind(queue_entry_id.to_string())
            .execute(db)
            .await?;

        return Ok(());
    }

    // Update the entry's play_order
    sqlx::query("UPDATE queue SET play_order = ? WHERE guid = ?")
        .bind(target_play_order)
        .bind(queue_entry_id.to_string())
        .execute(db)
        .await?;

    Ok(())
}

/// Renumber all queue entries with even spacing
///
/// Called automatically when play_order values get too large.
/// Assigns play_order values 10, 20, 30, ... in current order.
///
/// **Traceability:** DB-QUEUE-090
async fn renumber_queue(db: &Pool<Sqlite>) -> Result<()> {
    let entries = get_queue(db).await?;

    for (index, entry) in entries.iter().enumerate() {
        let new_order = (index as i64 + 1) * 10;
        sqlx::query("UPDATE queue SET play_order = ? WHERE guid = ?")
            .bind(new_order)
            .bind(&entry.guid)
            .execute(db)
            .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> Pool<Sqlite> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        // Create queue table
        sqlx::query(
            r#"
            CREATE TABLE queue (
                guid TEXT PRIMARY KEY,
                file_path TEXT NOT NULL,
                passage_guid TEXT,
                play_order INTEGER NOT NULL,
                start_time_ms INTEGER,
                end_time_ms INTEGER,
                lead_in_point_ms INTEGER,
                lead_out_point_ms INTEGER,
                fade_in_point_ms INTEGER,
                fade_out_point_ms INTEGER,
                fade_in_curve TEXT,
                fade_out_curve TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_enqueue_and_get_queue() {
        let db = setup_test_db().await;

        // Enqueue first entry
        let _id1 = enqueue(
            &db,
            "test1.mp3".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();

        // Enqueue second entry
        let _id2 = enqueue(
            &db,
            "test2.mp3".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();

        // Get queue
        let queue = get_queue(&db).await.unwrap();
        assert_eq!(queue.len(), 2);
        assert_eq!(queue[0].file_path, "test1.mp3");
        assert_eq!(queue[1].file_path, "test2.mp3");
        assert_eq!(queue[0].play_order, 10);
        assert_eq!(queue[1].play_order, 20);
    }

    #[tokio::test]
    async fn test_remove_from_queue() {
        let db = setup_test_db().await;

        let id = enqueue(
            &db,
            "test.mp3".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();

        remove_from_queue(&db, id).await.unwrap();

        let queue = get_queue(&db).await.unwrap();
        assert_eq!(queue.len(), 0);
    }

    #[tokio::test]
    async fn test_clear_queue() {
        let db = setup_test_db().await;

        // Add multiple entries
        for i in 0..5 {
            enqueue(
                &db,
                format!("test{}.mp3", i),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        }

        clear_queue(&db).await.unwrap();

        let queue = get_queue(&db).await.unwrap();
        assert_eq!(queue.len(), 0);
    }

    #[tokio::test]
    async fn test_get_next_play_order() {
        let db = setup_test_db().await;

        // Empty queue should start at 10
        let order1 = get_next_play_order(&db).await.unwrap();
        assert_eq!(order1, 10);

        // Add an entry
        enqueue(
            &db,
            "test.mp3".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();

        // Next should be 20
        let order2 = get_next_play_order(&db).await.unwrap();
        assert_eq!(order2, 20);
    }
}
