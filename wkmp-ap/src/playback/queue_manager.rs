//! Queue Manager
//!
//! Tracks which passages are where in the playback pipeline.
//!
//! **Traceability:**
//! - [SSD-BUF-010] Buffer management strategy
//! - [DB-QUEUE-010] Queue table schema
//! - [SSD-FLOW-010] Playback progression

use crate::db::queue;
use crate::error::{Error, Result};
use crate::state::SharedState;
use sqlx::{Pool, Sqlite};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{warn, debug};
use uuid::Uuid;
use wkmp_common::db::QueueEntry as DbQueueEntry;

/// Queue entry with parsed UUID
#[derive(Debug, Clone)]
pub struct QueueEntry {
    /// Queue entry UUID
    pub queue_entry_id: Uuid,

    /// Passage UUID (may be None for ephemeral passages)
    pub passage_id: Option<Uuid>,

    /// Audio file path
    pub file_path: PathBuf,

    /// Play order for sorting
    pub play_order: i64,

    /// Timing overrides from queue table (all in milliseconds)
    pub start_time_ms: Option<u64>,
    pub end_time_ms: Option<u64>,
    pub lead_in_point_ms: Option<u64>,
    pub lead_out_point_ms: Option<u64>,
    pub fade_in_point_ms: Option<u64>,
    pub fade_out_point_ms: Option<u64>,
    pub fade_in_curve: Option<String>,
    pub fade_out_curve: Option<String>,

    /// Discovered endpoint in ticks (for undefined endpoints)
    /// **[DBD-BUF-065]** Set when buffer manager discovers actual end of passage
    /// **[DBD-COMP-015]** Propagated from buffer to queue for crossfade timing
    pub discovered_end_ticks: Option<i64>,
}

impl QueueEntry {
    /// Convert from database QueueEntry to playback QueueEntry
    fn from_db(db_entry: DbQueueEntry) -> Result<Self> {
        let queue_entry_id = Uuid::parse_str(&db_entry.guid)
            .map_err(|e| Error::Queue(format!("Invalid queue entry UUID: {}", e)))?;

        let passage_id = db_entry
            .passage_guid
            .as_ref()
            .map(|s| Uuid::parse_str(s))
            .transpose()
            .map_err(|e| Error::Queue(format!("Invalid passage UUID: {}", e)))?;

        // Convert optional i64 to optional u64 for timing values
        let start_time_ms = db_entry.start_time_ms.map(|v| v as u64);
        let end_time_ms = db_entry.end_time_ms.map(|v| v as u64);
        let lead_in_point_ms = db_entry.lead_in_point_ms.map(|v| v as u64);
        let lead_out_point_ms = db_entry.lead_out_point_ms.map(|v| v as u64);
        let fade_in_point_ms = db_entry.fade_in_point_ms.map(|v| v as u64);
        let fade_out_point_ms = db_entry.fade_out_point_ms.map(|v| v as u64);

        Ok(Self {
            queue_entry_id,
            passage_id,
            file_path: PathBuf::from(db_entry.file_path),
            play_order: db_entry.play_order,
            start_time_ms,
            end_time_ms,
            lead_in_point_ms,
            lead_out_point_ms,
            fade_in_point_ms,
            fade_out_point_ms,
            fade_in_curve: db_entry.fade_in_curve,
            fade_out_curve: db_entry.fade_out_curve,
            discovered_end_ticks: None, // **[DBD-BUF-065]** Initialized as None
        })
    }
}

/// Queue position tracking
///
/// [SSD-BUF-010] Buffer management strategy:
/// - Current: Currently playing
/// - Next: Next to play (gets full buffer)
/// - Queued: After next (get partial buffers)
///
/// **[ISSUE-10]** Caches total count for O(1) length queries
pub struct QueueManager {
    /// Currently playing passage
    current: Option<QueueEntry>,

    /// Next to play (gets full buffer immediately)
    next: Option<QueueEntry>,

    /// After next (get partial buffers)
    queued: Vec<QueueEntry>,

    /// Cached total count (current + next + queued.len())
    /// [ISSUE-10] Maintained on all queue mutations for O(1) len()
    total_count: usize,
}

impl QueueManager {
    /// Create new empty queue manager
    pub fn new() -> Self {
        Self {
            current: None,
            next: None,
            queued: Vec::new(),
            total_count: 0, // [ISSUE-10] Initialize count
        }
    }

    /// Load queue from database
    ///
    /// [DB-QUEUE-010] Read queue table ordered by play_order
    /// **[REQ-AP-ERR-040]** Validates queue entries and auto-removes invalid ones
    pub async fn load_from_db(db: &Pool<Sqlite>) -> Result<Self> {
        let db_entries = queue::get_queue(db).await?;

        // Convert database entries to playback entries
        let entries: Vec<QueueEntry> = db_entries
            .into_iter()
            .map(QueueEntry::from_db)
            .collect::<Result<Vec<_>>>()?;

        // **[REQ-AP-ERR-040]** Validate entries (returns only valid ones)
        let validated_entries = Self::validate_entries(db, &entries, None).await;

        // Split into current, next, and queued
        let mut manager = Self::new();
        let mut validated_iter = validated_entries.into_iter();

        if let Some(entry) = validated_iter.next() {
            manager.current = Some(entry);
            manager.total_count += 1; // [ISSUE-10] Update count
        }

        if let Some(entry) = validated_iter.next() {
            manager.next = Some(entry);
            manager.total_count += 1; // [ISSUE-10] Update count
        }

        let remaining: Vec<QueueEntry> = validated_iter.collect();
        manager.total_count += remaining.len(); // [ISSUE-10] Add queued count
        manager.queued = remaining;

        Ok(manager)
    }

    /// Validate queue entries
    ///
    /// **[REQ-AP-ERR-040]** Queue validation with auto-removal of invalid entries
    /// **[ERH-QUEUE-010]** Invalid queue entry handling
    ///
    /// # Validation checks:
    /// - File path exists
    /// - Timing constraints: start < end (if both present)
    /// - No invalid values
    ///
    /// # Arguments
    /// - `db`: Database connection pool
    /// - `entries`: Queue entries to validate
    /// - `shared_state`: Optional shared state for event emission
    ///
    /// # Returns
    /// Vector containing only valid entries
    async fn validate_entries(
        db: &Pool<Sqlite>,
        entries: &[QueueEntry],
        shared_state: Option<&Arc<SharedState>>,
    ) -> Vec<QueueEntry> {
        let mut valid_entries = Vec::new();

        for entry in entries {
            if let Some(reason) = Self::validate_entry(entry).await {
                // **[ERH-QUEUE-010]** Log warning and emit event
                warn!(
                    "Invalid queue entry detected: queue_entry_id={}, passage_id={:?}, reason={}",
                    entry.queue_entry_id, entry.passage_id, reason
                );

                // Emit QueueValidationError event if shared_state available
                if let Some(state) = shared_state {
                    state.broadcast_event(wkmp_common::events::WkmpEvent::QueueValidationError {
                        queue_entry_id: entry.queue_entry_id,
                        passage_id: entry.passage_id,
                        validation_error: reason.clone(),
                        timestamp: chrono::Utc::now(),
                    });
                }

                // **[ERH-QUEUE-010]** Remove invalid entry from database (idempotent)
                match queue::remove_from_queue(db, entry.queue_entry_id).await {
                    Ok(was_removed) => {
                        if was_removed {
                            debug!("Auto-removed invalid queue entry {}", entry.queue_entry_id);
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Failed to auto-remove invalid queue entry {}: {}",
                            entry.queue_entry_id, e
                        );
                    }
                }

                debug!("Auto-removed invalid queue entry: {}", entry.queue_entry_id);
            } else {
                // Entry is valid
                valid_entries.push(entry.clone());
            }
        }

        let removed_count = entries.len() - valid_entries.len();
        if removed_count > 0 {
            warn!(
                "Queue validation complete: {} invalid entries auto-removed, {} valid entries remain",
                removed_count,
                valid_entries.len()
            );
        }

        valid_entries
    }

    /// Validate a single queue entry
    ///
    /// # Returns
    /// - None if entry is valid
    /// - Some(reason) if entry is invalid, with description of validation failure
    async fn validate_entry(entry: &QueueEntry) -> Option<String> {
        // **[ERH-QUEUE-010]** Check file path is not empty
        if entry.file_path.as_os_str().is_empty() {
            return Some("file_path is empty".to_string());
        }

        // **[ERH-QUEUE-010]** Check file exists
        if !entry.file_path.exists() {
            return Some(format!(
                "file does not exist: {}",
                entry.file_path.display()
            ));
        }

        // **[ERH-QUEUE-010]** Check timing constraints
        if let (Some(start), Some(end)) = (entry.start_time_ms, entry.end_time_ms) {
            if start >= end {
                return Some(format!(
                    "invalid timing: start_time ({}) >= end_time ({})",
                    start, end
                ));
            }
        }

        // Entry is valid
        None
    }

    /// Advance to next passage
    ///
    /// [SSD-FLOW-010] Playback progression:
    /// - current <- next
    /// - next <- queued[0]
    /// - queued <- queued[1..]
    ///
    /// Returns the new current passage, or None if queue is empty
    pub fn advance(&mut self) -> Option<QueueEntry> {
        // [ISSUE-10] Count changes: old current is discarded (-1)
        if self.current.is_some() {
            self.total_count -= 1;
        }

        // Move next to current
        self.current = self.next.take();

        // Move first queued to next
        if !self.queued.is_empty() {
            self.next = Some(self.queued.remove(0));
        }

        self.current.clone()
    }

    /// Get current passage
    pub fn current(&self) -> Option<&QueueEntry> {
        self.current.as_ref()
    }

    /// Get next passage
    pub fn next(&self) -> Option<&QueueEntry> {
        self.next.as_ref()
    }

    /// Get queued passages
    pub fn queued(&self) -> &[QueueEntry] {
        &self.queued
    }

    /// Remove specific entry from queue (skip operation)
    ///
    /// Can remove from current, next, or queued.
    /// Returns true if entry was found and removed.
    pub fn remove(&mut self, queue_entry_id: Uuid) -> bool {
        // Check if it's the current passage
        if let Some(ref current) = self.current {
            if current.queue_entry_id == queue_entry_id {
                // Advance to next (advance() handles count update)
                self.advance();
                return true;
            }
        }

        // Check if it's the next passage
        if let Some(ref next) = self.next {
            if next.queue_entry_id == queue_entry_id {
                // Replace next with first queued
                if !self.queued.is_empty() {
                    self.next = Some(self.queued.remove(0));
                } else {
                    self.next = None;
                }
                self.total_count -= 1; // [ISSUE-10] Removed one entry
                return true;
            }
        }

        // Check queued passages
        if let Some(index) = self.queued.iter().position(|e| e.queue_entry_id == queue_entry_id) {
            self.queued.remove(index);
            self.total_count -= 1; // [ISSUE-10] Removed one entry
            return true;
        }

        false
    }

    /// Add entry to queue
    ///
    /// Appends to end of queued list.
    /// Call this after enqueuing to database to keep in-memory state in sync.
    pub fn enqueue(&mut self, entry: QueueEntry) {
        // [ISSUE-10] Increment count for any addition
        self.total_count += 1;

        // If queue is completely empty, set as current
        if self.current.is_none() {
            self.current = Some(entry);
            return;
        }

        // If next is empty, set as next
        if self.next.is_none() {
            self.next = Some(entry);
            return;
        }

        // Otherwise append to queued
        self.queued.push(entry);
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.current = None;
        self.next = None;
        self.queued.clear();
        self.total_count = 0; // [ISSUE-10] Reset count
    }

    /// Check if queue is completely empty
    pub fn is_empty(&self) -> bool {
        self.current.is_none() && self.next.is_none() && self.queued.is_empty()
    }

    /// Get total queue length (current + next + queued)
    /// [ISSUE-10] O(1) cached count instead of O(1) calculation
    pub fn len(&self) -> usize {
        self.total_count
    }

    /// Set discovered endpoint for a queue entry
    ///
    /// **[DBD-BUF-065]** Store discovered endpoint in ticks
    /// **[DBD-COMP-015]** Enables crossfade timing with undefined endpoints
    ///
    /// Returns true if the entry was found and updated.
    pub fn set_discovered_endpoint(&mut self, queue_entry_id: Uuid, end_ticks: i64) -> bool {
        // Check current passage
        if let Some(ref mut current) = self.current {
            if current.queue_entry_id == queue_entry_id {
                current.discovered_end_ticks = Some(end_ticks);
                return true;
            }
        }

        // Check next passage
        if let Some(ref mut next) = self.next {
            if next.queue_entry_id == queue_entry_id {
                next.discovered_end_ticks = Some(end_ticks);
                return true;
            }
        }

        // Check queued passages
        if let Some(entry) = self.queued.iter_mut().find(|e| e.queue_entry_id == queue_entry_id) {
            entry.discovered_end_ticks = Some(end_ticks);
            return true;
        }

        false
    }

    /// Get effective end ticks for a queue entry
    ///
    /// **[DBD-BUF-065]** Returns discovered endpoint if available
    /// **[DBD-COMP-015]** Falls back to passage end_time_ticks if no discovery
    ///
    /// Returns None if entry not found or no endpoint information available.
    pub fn get_discovered_endpoint(&self, queue_entry_id: Uuid) -> Option<i64> {
        // Check current passage
        if let Some(ref current) = self.current {
            if current.queue_entry_id == queue_entry_id {
                return current.discovered_end_ticks;
            }
        }

        // Check next passage
        if let Some(ref next) = self.next {
            if next.queue_entry_id == queue_entry_id {
                return next.discovered_end_ticks;
            }
        }

        // Check queued passages
        if let Some(entry) = self.queued.iter().find(|e| e.queue_entry_id == queue_entry_id) {
            return entry.discovered_end_ticks;
        }

        None
    }
}

impl Default for QueueManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_entry(id: u8, passage_id: Option<Uuid>) -> QueueEntry {
        QueueEntry {
            queue_entry_id: Uuid::from_bytes([id; 16]),
            passage_id,
            file_path: PathBuf::from(format!("test{}.mp3", id)),
            play_order: (id as i64) * 10,
            start_time_ms: None,
            end_time_ms: None,
            lead_in_point_ms: None,
            lead_out_point_ms: None,
            fade_in_point_ms: None,
            fade_out_point_ms: None,
            fade_in_curve: None,
            fade_out_curve: None,
            discovered_end_ticks: None,
        }
    }

    #[test]
    fn test_queue_manager_creation() {
        let manager = QueueManager::new();
        assert!(manager.is_empty());
        assert_eq!(manager.len(), 0);
    }

    #[test]
    fn test_queue_manager_enqueue() {
        let mut manager = QueueManager::new();

        // First entry becomes current
        let entry1 = create_test_entry(1, None);
        manager.enqueue(entry1.clone());
        assert_eq!(manager.len(), 1);
        assert_eq!(manager.current().unwrap().queue_entry_id, entry1.queue_entry_id);
        assert!(manager.next().is_none());

        // Second entry becomes next
        let entry2 = create_test_entry(2, None);
        manager.enqueue(entry2.clone());
        assert_eq!(manager.len(), 2);
        assert_eq!(manager.next().unwrap().queue_entry_id, entry2.queue_entry_id);

        // Third entry goes to queued
        let entry3 = create_test_entry(3, None);
        manager.enqueue(entry3.clone());
        assert_eq!(manager.len(), 3);
        assert_eq!(manager.queued().len(), 1);
        assert_eq!(manager.queued()[0].queue_entry_id, entry3.queue_entry_id);
    }

    #[test]
    fn test_queue_manager_advance() {
        let mut manager = QueueManager::new();

        // Enqueue 3 entries
        let entry1 = create_test_entry(1, None);
        let entry2 = create_test_entry(2, None);
        let entry3 = create_test_entry(3, None);
        manager.enqueue(entry1.clone());
        manager.enqueue(entry2.clone());
        manager.enqueue(entry3.clone());

        // Advance: current=2, next=3, queued=[]
        let current = manager.advance().unwrap();
        assert_eq!(current.queue_entry_id, entry2.queue_entry_id);
        assert_eq!(manager.current().unwrap().queue_entry_id, entry2.queue_entry_id);
        assert_eq!(manager.next().unwrap().queue_entry_id, entry3.queue_entry_id);
        assert_eq!(manager.queued().len(), 0);

        // Advance: current=3, next=None, queued=[]
        let current = manager.advance().unwrap();
        assert_eq!(current.queue_entry_id, entry3.queue_entry_id);
        assert!(manager.next().is_none());

        // Advance: current=None, next=None, queued=[]
        let current = manager.advance();
        assert!(current.is_none());
        assert!(manager.is_empty());
    }

    #[test]
    fn test_queue_manager_remove_current() {
        let mut manager = QueueManager::new();

        let entry1 = create_test_entry(1, None);
        let entry2 = create_test_entry(2, None);
        let entry3 = create_test_entry(3, None);
        manager.enqueue(entry1.clone());
        manager.enqueue(entry2.clone());
        manager.enqueue(entry3.clone());

        // Remove current (should advance)
        let removed = manager.remove(entry1.queue_entry_id);
        assert!(removed);
        assert_eq!(manager.current().unwrap().queue_entry_id, entry2.queue_entry_id);
        assert_eq!(manager.next().unwrap().queue_entry_id, entry3.queue_entry_id);
        assert_eq!(manager.len(), 2);
    }

    #[test]
    fn test_queue_manager_remove_next() {
        let mut manager = QueueManager::new();

        let entry1 = create_test_entry(1, None);
        let entry2 = create_test_entry(2, None);
        let entry3 = create_test_entry(3, None);
        manager.enqueue(entry1.clone());
        manager.enqueue(entry2.clone());
        manager.enqueue(entry3.clone());

        // Remove next
        let removed = manager.remove(entry2.queue_entry_id);
        assert!(removed);
        assert_eq!(manager.current().unwrap().queue_entry_id, entry1.queue_entry_id);
        assert_eq!(manager.next().unwrap().queue_entry_id, entry3.queue_entry_id);
        assert_eq!(manager.len(), 2);
    }

    #[test]
    fn test_queue_manager_remove_queued() {
        let mut manager = QueueManager::new();

        let entry1 = create_test_entry(1, None);
        let entry2 = create_test_entry(2, None);
        let entry3 = create_test_entry(3, None);
        let entry4 = create_test_entry(4, None);
        manager.enqueue(entry1.clone());
        manager.enqueue(entry2.clone());
        manager.enqueue(entry3.clone());
        manager.enqueue(entry4.clone());

        // Remove from queued (entry3)
        let removed = manager.remove(entry3.queue_entry_id);
        assert!(removed);
        assert_eq!(manager.current().unwrap().queue_entry_id, entry1.queue_entry_id);
        assert_eq!(manager.next().unwrap().queue_entry_id, entry2.queue_entry_id);
        assert_eq!(manager.queued().len(), 1);
        assert_eq!(manager.queued()[0].queue_entry_id, entry4.queue_entry_id);
        assert_eq!(manager.len(), 3);
    }

    #[test]
    fn test_queue_manager_remove_not_found() {
        let mut manager = QueueManager::new();

        let entry1 = create_test_entry(1, None);
        manager.enqueue(entry1);

        // Try to remove non-existent entry
        let removed = manager.remove(Uuid::new_v4());
        assert!(!removed);
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_queue_manager_clear() {
        let mut manager = QueueManager::new();

        manager.enqueue(create_test_entry(1, None));
        manager.enqueue(create_test_entry(2, None));
        manager.enqueue(create_test_entry(3, None));

        manager.clear();
        assert!(manager.is_empty());
        assert_eq!(manager.len(), 0);
    }
}
