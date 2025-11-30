//! Edition Grouping
//!
//! **[PLAN030]** Groups MusicBrainz releases into editions by track count
//! and duration pattern. Editions with identical track patterns are
//! considered equivalent for matching purposes.

use std::collections::HashMap;

use crate::matching::types::{Edition, MBReleaseDetails};

/// Key for grouping editions by track pattern
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EditionKey {
    /// Number of tracks in this edition
    track_count: usize,
    /// Duration signature (rounded durations joined)
    duration_signature: String,
}

impl EditionKey {
    /// Create a new edition key from track count and durations
    ///
    /// The duration signature rounds each duration to nearest 5 seconds
    /// to group editions with slightly different timings together.
    fn new(track_count: usize, durations_ms: &[u32]) -> Self {
        // Create signature by rounding durations to nearest 5 seconds
        let signature: Vec<String> = durations_ms
            .iter()
            .map(|ms| {
                let secs = ms / 1000;
                let rounded = (secs / 5) * 5;
                rounded.to_string()
            })
            .collect();

        Self {
            track_count,
            duration_signature: signature.join("-"),
        }
    }
}

/// Group releases into editions by track pattern
///
/// Creates unique editions based on track count and duration signature.
/// Multiple releases with identical track patterns are collapsed into
/// a single edition.
///
/// # Arguments
/// * `releases` - MusicBrainz releases with track details
///
/// # Returns
/// Vector of unique editions
pub fn group_into_editions(releases: &[MBReleaseDetails]) -> Vec<Edition> {
    let mut edition_map: HashMap<EditionKey, Edition> = HashMap::new();

    for release in releases {
        // Get total tracks and durations across all media
        let mut track_count = 0;
        let mut durations_ms: Vec<u32> = Vec::new();
        let mut recording_mbids: Vec<String> = Vec::new();

        for media in &release.media {
            for track in &media.tracks {
                track_count += 1;
                durations_ms.push(track.length.unwrap_or(0));
                if let Some(ref recording) = track.recording {
                    recording_mbids.push(recording.id.clone());
                }
            }
        }

        let key = EditionKey::new(track_count, &durations_ms);

        edition_map.entry(key).or_insert_with(|| Edition {
            release_mbid: release.id.clone(),
            title: release.title.clone(),
            artist: extract_artist_from_release(release),
            artist_credit: None,
            country: release.country.clone(),
            status: release.status.clone(),
            track_count,
            track_durations: durations_ms.iter().map(|ms| *ms as f64 / 1000.0).collect(),
            recording_mbids,
            name_distance_rank: None,
            name_distance_score: None,
            durations: durations_ms,
        });
    }

    edition_map.into_values().collect()
}

/// Extract artist name from release details
///
/// Attempts to get artist from first recording's artist credit.
/// Returns "Unknown Artist" if not available.
fn extract_artist_from_release(release: &MBReleaseDetails) -> String {
    // Try to get artist from first recording
    for media in &release.media {
        for track in &media.tracks {
            if let Some(ref recording) = track.recording {
                if let Some(ref artist_credit) = recording.artist_credit {
                    if let Some(first_credit) = artist_credit.first() {
                        return first_credit.name.clone();
                    }
                }
            }
        }
    }

    "Unknown Artist".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matching::types::{MBArtist, MBArtistCredit, MBMedia, MBRecording, MBTrack};

    fn create_test_release(id: &str, title: &str, track_durations_ms: &[u32]) -> MBReleaseDetails {
        let tracks: Vec<MBTrack> = track_durations_ms
            .iter()
            .enumerate()
            .map(|(i, &duration)| MBTrack {
                id: format!("track-{}", i),
                number: format!("{}", i + 1),
                title: format!("Track {}", i + 1),
                length: Some(duration),
                position: Some(i + 1),
                recording: Some(MBRecording {
                    id: format!("recording-{}", i),
                    title: Some(format!("Track {}", i + 1)),
                    length: Some(duration),
                    artist_credit: Some(vec![MBArtistCredit {
                        name: "Test Artist".to_string(),
                        artist: MBArtist {
                            name: "Test Artist".to_string(),
                        },
                    }]),
                }),
            })
            .collect();

        MBReleaseDetails {
            id: id.to_string(),
            title: title.to_string(),
            status: Some("Official".to_string()),
            country: Some("US".to_string()),
            date: None,
            barcode: None,
            media: vec![MBMedia {
                format: Some("CD".to_string()),
                track_count: tracks.len(),
                tracks,
            }],
        }
    }

    #[test]
    fn test_group_by_track_count() {
        let releases = vec![
            create_test_release("rel-1", "Album 1", &[180000, 240000, 200000]),
            create_test_release("rel-2", "Album 2", &[180000, 240000]), // Different track count
        ];

        let editions = group_into_editions(&releases);

        // Should create 2 editions (different track counts)
        assert_eq!(editions.len(), 2);
    }

    #[test]
    fn test_group_identical_patterns() {
        let releases = vec![
            create_test_release("rel-1", "Album US", &[180000, 240000, 200000]),
            create_test_release("rel-2", "Album UK", &[180000, 240000, 200000]), // Same pattern
        ];

        let editions = group_into_editions(&releases);

        // Should create only 1 edition (identical patterns grouped)
        assert_eq!(editions.len(), 1);
    }

    #[test]
    fn test_group_similar_durations() {
        // Durations within 5 seconds should be grouped together
        let releases = vec![
            create_test_release("rel-1", "Album 1", &[180000, 240000, 200000]),
            create_test_release("rel-2", "Album 2", &[182000, 241000, 203000]), // Within 5s
        ];

        let editions = group_into_editions(&releases);

        // Should create only 1 edition (rounded to same signature)
        assert_eq!(editions.len(), 1);
    }

    #[test]
    fn test_multi_disc_handling() {
        // Create a multi-disc release
        let tracks_disc1: Vec<MBTrack> = (0..5)
            .map(|i| MBTrack {
                id: format!("track-d1-{}", i),
                number: format!("{}", i + 1),
                title: format!("Disc 1 Track {}", i + 1),
                length: Some(200000),
                position: Some(i + 1),
                recording: None,
            })
            .collect();

        let tracks_disc2: Vec<MBTrack> = (0..5)
            .map(|i| MBTrack {
                id: format!("track-d2-{}", i),
                number: format!("{}", i + 1),
                title: format!("Disc 2 Track {}", i + 1),
                length: Some(200000),
                position: Some(i + 1),
                recording: None,
            })
            .collect();

        let multi_disc = MBReleaseDetails {
            id: "multi-disc".to_string(),
            title: "Double Album".to_string(),
            status: Some("Official".to_string()),
            country: None,
            date: None,
            barcode: None,
            media: vec![
                MBMedia {
                    format: Some("CD".to_string()),
                    track_count: 5,
                    tracks: tracks_disc1,
                },
                MBMedia {
                    format: Some("CD".to_string()),
                    track_count: 5,
                    tracks: tracks_disc2,
                },
            ],
        };

        let editions = group_into_editions(&[multi_disc]);

        assert_eq!(editions.len(), 1);
        assert_eq!(editions[0].track_count, 10); // 5 + 5 tracks
    }

    #[test]
    fn test_extract_artist() {
        let release = create_test_release("rel-1", "Album", &[180000]);
        let artist = extract_artist_from_release(&release);
        assert_eq!(artist, "Test Artist");
    }
}
