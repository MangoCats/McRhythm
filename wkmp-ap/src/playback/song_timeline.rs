//! Song timeline boundary detection
//!
//! Manages song boundaries within a passage and detects when playback
//! crosses from one song to another (or into/out of gaps).
//!
//! **Traceability:**
//! - [ARCH-SNGC-041] Song Timeline Data Structure
//! - [ARCH-SNGC-042] Efficient Boundary Detection Algorithm
//! - [REV002] Event-driven architecture update

use uuid::Uuid;

/// Song timeline entry representing a song or gap within a passage
///
/// Each entry represents a time range within the passage. Entries may
/// overlap (e.g., when multiple songs play simultaneously) or have gaps
/// between them (no song playing).
///
/// **Traceability:** [ARCH-SNGC-041] Song Timeline Data Structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SongTimelineEntry {
    /// Song UUID, or None for gaps between songs
    pub song_id: Option<Uuid>,

    /// Start time within passage (milliseconds)
    pub start_time_ms: u64,

    /// End time within passage (milliseconds)
    pub end_time_ms: u64,
}

/// Song timeline for a passage
///
/// Maintains a sorted list of song timeline entries and efficiently
/// detects when playback crosses song boundaries.
///
/// **Design:**
/// - Entries sorted by `start_time_ms` ascending
/// - Current index cached for O(1) typical-case performance
/// - Handles gaps, single songs, and multi-song passages
///
/// **Traceability:** [ARCH-SNGC-041] Song Timeline Data Structure
#[derive(Debug, Clone)]
pub struct SongTimeline {
    /// Sorted list of timeline entries (by start_time_ms ascending)
    entries: Vec<SongTimelineEntry>,

    /// Cached index of currently playing entry
    ///
    /// This index is updated by `check_boundary()` to enable O(1)
    /// typical-case performance (when position advances linearly).
    ///
    /// - None: Not initialized yet (first call to check_boundary)
    /// - Some(index): Currently at this entry index
    /// - Some(entries.len()): In a gap between songs
    current_index: Option<usize>,
}

impl SongTimeline {
    /// Create new song timeline from entries
    ///
    /// Entries will be sorted by `start_time_ms` ascending.
    ///
    /// # Arguments
    /// * `entries` - Timeline entries (will be sorted)
    ///
    /// # Returns
    /// New `SongTimeline` with sorted entries and index uninitialized
    pub fn new(mut entries: Vec<SongTimelineEntry>) -> Self {
        // Sort by start_time_ms ascending
        entries.sort_by_key(|e| e.start_time_ms);

        Self {
            entries,
            current_index: None, // Not initialized until first check_boundary call
        }
    }

    /// Check if position crossed a song boundary
    ///
    /// **Algorithm:**
    /// 1. Check cached current_index first (O(1) hot path)
    /// 2. If position outside current entry, search all entries (O(n) cold path)
    /// 3. Update cached index and return boundary crossing status
    ///
    /// **Traceability:** [ARCH-SNGC-042] Efficient Boundary Detection Algorithm
    ///
    /// # Arguments
    /// * `position_ms` - Current playback position in milliseconds
    ///
    /// # Returns
    /// Tuple `(crossed_boundary, new_song_id)`:
    /// - `crossed_boundary`: true if moved to different song/gap
    /// - `new_song_id`: UUID of new song, or None if in gap
    ///
    /// # Examples
    /// ```
    /// use wkmp_ap::playback::song_timeline::{SongTimeline, SongTimelineEntry};
    /// use uuid::Uuid;
    ///
    /// let song_id = Uuid::new_v4();
    /// let entries = vec![
    ///     SongTimelineEntry {
    ///         song_id: Some(song_id),
    ///         start_time_ms: 0,
    ///         end_time_ms: 10000,
    ///     }
    /// ];
    ///
    /// let mut timeline = SongTimeline::new(entries);
    ///
    /// // First check at start
    /// let (crossed, song) = timeline.check_boundary(0);
    /// assert_eq!(crossed, false); // First check never counts as crossing
    /// assert_eq!(song, Some(song_id));
    ///
    /// // Advance within same song
    /// let (crossed, song) = timeline.check_boundary(5000);
    /// assert_eq!(crossed, false);
    /// assert_eq!(song, Some(song_id));
    ///
    /// // Advance past song end (into gap)
    /// let (crossed, song) = timeline.check_boundary(11000);
    /// assert_eq!(crossed, true);
    /// assert_eq!(song, None);
    /// ```
    pub fn check_boundary(&mut self, position_ms: u64) -> (bool, Option<Uuid>) {
        if self.entries.is_empty() {
            self.current_index = Some(0); // Initialize even for empty timeline
            return (false, None);
        }

        // HOT PATH: Check current cached entry first (typical case)
        if let Some(current_idx) = self.current_index {
            if current_idx < self.entries.len() {
                let entry = &self.entries[current_idx];

                if position_ms >= entry.start_time_ms && position_ms < entry.end_time_ms {
                    // Still in same entry, no boundary crossed
                    return (false, entry.song_id);
                }
            }
        }

        // COLD PATH: Position changed entries - search for new entry
        let old_index = self.current_index;

        for (i, entry) in self.entries.iter().enumerate() {
            if position_ms >= entry.start_time_ms && position_ms < entry.end_time_ms {
                self.current_index = Some(i);

                // Crossed boundary if:
                // 1. old_index was Some (not first call), AND
                // 2. We're at a different index
                let crossed = match old_index {
                    Some(old_i) => i != old_i,
                    None => false, // First call never counts as crossing
                };

                return (crossed, entry.song_id);
            }
        }

        // Position is in a gap (not within any song)
        let new_index = self.entries.len(); // Mark as "gap"

        // Crossed boundary if we were previously in a song entry
        let crossed = match old_index {
            Some(old_i) if old_i < self.entries.len() => true, // Was in song, now in gap
            Some(old_i) if old_i == self.entries.len() => false, // Was in gap, still in gap
            None => false, // First call
            _ => false,
        };

        self.current_index = Some(new_index);
        (crossed, None)
    }

    /// Get current song ID at position (without boundary check)
    ///
    /// This is a read-only query that doesn't update cached state.
    ///
    /// # Arguments
    /// * `position_ms` - Playback position in milliseconds
    ///
    /// # Returns
    /// Song UUID if position within a song, None if in gap
    pub fn get_current_song(&self, position_ms: u64) -> Option<Uuid> {
        for entry in &self.entries {
            if position_ms >= entry.start_time_ms && position_ms < entry.end_time_ms {
                return entry.song_id;
            }
        }
        None
    }

    /// Get number of entries in timeline
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if timeline is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_timeline() {
        let mut timeline = SongTimeline::new(vec![]);

        let (crossed, song_id) = timeline.check_boundary(1000);
        assert_eq!(crossed, false);
        assert_eq!(song_id, None);

        assert_eq!(timeline.get_current_song(1000), None);
        assert!(timeline.is_empty());
        assert_eq!(timeline.len(), 0);
    }

    #[test]
    fn test_single_song() {
        let song_id = Uuid::new_v4();
        let entries = vec![SongTimelineEntry {
            song_id: Some(song_id),
            start_time_ms: 1000,
            end_time_ms: 5000,
        }];

        let mut timeline = SongTimeline::new(entries);

        // Before song starts (gap)
        let (crossed, current) = timeline.check_boundary(500);
        assert_eq!(crossed, false); // First check
        assert_eq!(current, None);

        // Song starts (boundary crossing)
        let (crossed, current) = timeline.check_boundary(1000);
        assert_eq!(crossed, true);
        assert_eq!(current, Some(song_id));

        // Within song (no boundary)
        let (crossed, current) = timeline.check_boundary(3000);
        assert_eq!(crossed, false);
        assert_eq!(current, Some(song_id));

        // Song ends (boundary to gap)
        let (crossed, current) = timeline.check_boundary(5000);
        assert_eq!(crossed, true);
        assert_eq!(current, None);
    }

    #[test]
    fn test_multiple_songs_with_gaps() {
        let song1 = Uuid::new_v4();
        let song2 = Uuid::new_v4();
        let song3 = Uuid::new_v4();

        let entries = vec![
            SongTimelineEntry {
                song_id: Some(song1),
                start_time_ms: 0,
                end_time_ms: 10000,
            },
            SongTimelineEntry {
                song_id: Some(song2),
                start_time_ms: 15000, // 5-second gap
                end_time_ms: 25000,
            },
            SongTimelineEntry {
                song_id: Some(song3),
                start_time_ms: 25000, // No gap
                end_time_ms: 35000,
            },
        ];

        let mut timeline = SongTimeline::new(entries);

        // Song 1 start
        let (crossed, current) = timeline.check_boundary(0);
        assert_eq!(crossed, false); // First check
        assert_eq!(current, Some(song1));

        // Within song 1
        let (crossed, current) = timeline.check_boundary(5000);
        assert_eq!(crossed, false);
        assert_eq!(current, Some(song1));

        // Gap after song 1
        let (crossed, current) = timeline.check_boundary(12000);
        assert_eq!(crossed, true);
        assert_eq!(current, None);

        // Song 2 start
        let (crossed, current) = timeline.check_boundary(15000);
        assert_eq!(crossed, true);
        assert_eq!(current, Some(song2));

        // Song 3 (immediate transition, no gap)
        let (crossed, current) = timeline.check_boundary(25000);
        assert_eq!(crossed, true);
        assert_eq!(current, Some(song3));
    }

    #[test]
    fn test_forward_seek_across_songs() {
        let song1 = Uuid::new_v4();
        let song2 = Uuid::new_v4();

        let entries = vec![
            SongTimelineEntry {
                song_id: Some(song1),
                start_time_ms: 0,
                end_time_ms: 10000,
            },
            SongTimelineEntry {
                song_id: Some(song2),
                start_time_ms: 10000,
                end_time_ms: 20000,
            },
        ];

        let mut timeline = SongTimeline::new(entries);

        // Start in song 1
        timeline.check_boundary(5000);

        // Seek forward to song 2 (should detect boundary crossing)
        let (crossed, current) = timeline.check_boundary(15000);
        assert_eq!(crossed, true);
        assert_eq!(current, Some(song2));
    }

    #[test]
    fn test_backward_seek() {
        let song1 = Uuid::new_v4();
        let song2 = Uuid::new_v4();

        let entries = vec![
            SongTimelineEntry {
                song_id: Some(song1),
                start_time_ms: 0,
                end_time_ms: 10000,
            },
            SongTimelineEntry {
                song_id: Some(song2),
                start_time_ms: 10000,
                end_time_ms: 20000,
            },
        ];

        let mut timeline = SongTimeline::new(entries);

        // Start in song 2
        timeline.check_boundary(15000);

        // Seek backward to song 1 (should detect boundary crossing)
        let (crossed, current) = timeline.check_boundary(5000);
        assert_eq!(crossed, true);
        assert_eq!(current, Some(song1));
    }

    #[test]
    fn test_unsorted_entries_get_sorted() {
        let song1 = Uuid::new_v4();
        let song2 = Uuid::new_v4();

        // Create entries in reverse order
        let entries = vec![
            SongTimelineEntry {
                song_id: Some(song2),
                start_time_ms: 10000,
                end_time_ms: 20000,
            },
            SongTimelineEntry {
                song_id: Some(song1),
                start_time_ms: 0,
                end_time_ms: 10000,
            },
        ];

        let mut timeline = SongTimeline::new(entries);

        // Should find song1 at position 5000 (even though it was second in input)
        let (_, current) = timeline.check_boundary(5000);
        assert_eq!(current, Some(song1));

        // Should find song2 at position 15000
        let (_, current) = timeline.check_boundary(15000);
        assert_eq!(current, Some(song2));
    }

    #[test]
    fn test_get_current_song_no_state_change() {
        let song_id = Uuid::new_v4();
        let entries = vec![SongTimelineEntry {
            song_id: Some(song_id),
            start_time_ms: 1000,
            end_time_ms: 5000,
        }];

        let timeline = SongTimeline::new(entries);

        // get_current_song should work without mutating state
        assert_eq!(timeline.get_current_song(500), None);
        assert_eq!(timeline.get_current_song(2000), Some(song_id));
        assert_eq!(timeline.get_current_song(6000), None);

        // Verify current_index wasn't changed (should still be None since we didn't call check_boundary)
        assert_eq!(timeline.current_index, None);
    }

    #[test]
    fn test_gap_only_passage() {
        // Passage with no songs at all (entire passage is a gap)
        let timeline = SongTimeline::new(vec![]);

        // All positions should return None
        assert_eq!(timeline.get_current_song(0), None);
        assert_eq!(timeline.get_current_song(10000), None);
    }

    #[test]
    fn test_entry_equality() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let entry1 = SongTimelineEntry {
            song_id: Some(id1),
            start_time_ms: 0,
            end_time_ms: 1000,
        };

        let entry2 = SongTimelineEntry {
            song_id: Some(id1),
            start_time_ms: 0,
            end_time_ms: 1000,
        };

        let entry3 = SongTimelineEntry {
            song_id: Some(id2),
            start_time_ms: 0,
            end_time_ms: 1000,
        };

        assert_eq!(entry1, entry2);
        assert_ne!(entry1, entry3);
    }
}
