//! MBID Identification Cascade
//!
//! **[SPEC-EMBID-001]** Three-stage MBID resolution:
//! - Stage 0: Embedded MusicBrainz Recording ID (ID3 tags)
//! - Stage 1: ContextualMatcher (artist+title → MusicBrainz search)
//! - Stage 2: AcoustID fingerprint (handled externally by PassageFingerprinter)
//!
//! Returns `None` when Stages 0+1 fail, signaling the caller to proceed with Stage 2.

use super::contextual_matcher::ContextualMatcher;
use super::metadata_merger::MergedMetadata;
use super::passage_song_matcher::MbidResolution;
use crate::matching::confidence_tier::assign_tier;
use crate::matching::ConfidenceTier;

/// Minimum ContextualMatcher score to accept as a match
const CONTEXTUAL_MATCH_THRESHOLD: f32 = 0.85;

/// MBID Identification Cascade
///
/// Resolves MusicBrainz Recording IDs using a metadata-first strategy.
/// Stage 0 (embedded MBID) is local-only and handles ~98% of Picard-tagged files.
/// Stage 1 (ContextualMatcher) requires network access to MusicBrainz.
pub struct MbidIdentificationCascade {
    contextual_matcher: Option<ContextualMatcher>,
}

impl MbidIdentificationCascade {
    /// Create cascade with optional ContextualMatcher
    ///
    /// ContextualMatcher creation can fail (requires MusicBrainzClient).
    /// When unavailable, Stage 1 is skipped gracefully.
    pub fn new() -> Self {
        let contextual_matcher = match ContextualMatcher::new() {
            Ok(matcher) => Some(matcher),
            Err(e) => {
                tracing::debug!(
                    error = ?e,
                    "ContextualMatcher unavailable, Stage 1 disabled"
                );
                None
            }
        };

        Self {
            contextual_matcher,
        }
    }

    /// Resolve MBID for a single-track file
    ///
    /// **Cascade:**
    /// 1. Stage 0: Check embedded MBID in metadata tags
    /// 2. Stage 1: ContextualMatcher (artist+title → MusicBrainz search)
    /// 3. Returns None → caller should try AcoustID (Stage 2)
    pub async fn resolve_single_track(
        &self,
        metadata: &MergedMetadata,
    ) -> Option<MbidResolution> {
        // Stage 0: Embedded MusicBrainz Recording ID
        if let Some(ref mbid) = metadata.recording_mbid {
            let tier = assign_tier(true, metadata.isrc.is_some(), None, false);
            tracing::info!(
                mbid = %mbid,
                tier = %tier.name(),
                has_isrc = metadata.isrc.is_some(),
                "Stage 0: Embedded MBID found"
            );
            return Some(MbidResolution {
                mbid: mbid.clone(),
                tier,
                score: tier.confidence_score(),
                source: "Embedded MBID".to_string(),
            });
        }

        // Stage 1: ContextualMatcher (artist+title → MusicBrainz search)
        if let Some(ref matcher) = self.contextual_matcher {
            let artist = metadata.artist.as_deref().unwrap_or("");
            let title = metadata.title.as_deref().unwrap_or("");

            if artist.is_empty() || title.is_empty() {
                tracing::debug!(
                    has_artist = !artist.is_empty(),
                    has_title = !title.is_empty(),
                    "Stage 1 skipped: insufficient metadata for ContextualMatcher"
                );
            }

            if !artist.is_empty() && !title.is_empty() {
                let duration = metadata.duration_ticks as f32 / 28_224_000.0;
                let duration_secs = if duration > 0.0 {
                    Some(duration)
                } else {
                    None
                };

                match matcher
                    .match_single_segment(artist, title, duration_secs)
                    .await
                {
                    Ok(candidates) if !candidates.is_empty() => {
                        let top = &candidates[0];
                        if top.match_score >= CONTEXTUAL_MATCH_THRESHOLD {
                            tracing::info!(
                                mbid = %top.recording_mbid,
                                score = top.match_score,
                                artist = %artist,
                                title = %title,
                                "Stage 1: ContextualMatcher found MBID"
                            );
                            return Some(MbidResolution {
                                mbid: top.recording_mbid.clone(),
                                tier: ConfidenceTier::Tier3,
                                score: top.match_score,
                                source: "ContextualMatcher".to_string(),
                            });
                        }
                        tracing::debug!(
                            top_score = top.match_score,
                            threshold = CONTEXTUAL_MATCH_THRESHOLD,
                            "Stage 1: Best candidate below threshold"
                        );
                    }
                    Ok(_) => {
                        tracing::debug!("Stage 1: No candidates found");
                    }
                    Err(e) => {
                        tracing::debug!(error = ?e, "Stage 1: ContextualMatcher failed");
                    }
                }
            }
        }

        // No resolution — caller should try AcoustID (Stage 2)
        tracing::debug!("MBID cascade: no resolution from Stage 0 or 1");
        None
    }

    /// Resolve MBIDs for passages in an album file
    ///
    /// For album files, embedded MBID is per-file (not per-passage).
    /// This increment returns all-None; album per-passage resolution
    /// is handled by AcoustID fingerprinting (Stage 2).
    ///
    /// Future: release lookup → per-track MBID mapping.
    pub async fn resolve_album_passages(
        &self,
        _metadata: &MergedMetadata,
        passage_count: usize,
    ) -> Vec<Option<MbidResolution>> {
        vec![None; passage_count]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_metadata(
        recording_mbid: Option<&str>,
        isrc: Option<&str>,
        artist: Option<&str>,
        title: Option<&str>,
    ) -> MergedMetadata {
        MergedMetadata {
            artist: artist.map(|s| s.to_string()),
            title: title.map(|s| s.to_string()),
            album: None,
            track_number: None,
            year: None,
            duration_ticks: 28_224_000 * 180, // 180 seconds
            format: "MP3".to_string(),
            sample_rate: Some(44100),
            channels: Some(2),
            file_size_bytes: 5_000_000,
            recording_mbid: recording_mbid.map(|s| s.to_string()),
            isrc: isrc.map(|s| s.to_string()),
        }
    }

    #[tokio::test]
    async fn test_resolve_embedded_mbid_with_isrc() {
        let cascade = MbidIdentificationCascade { contextual_matcher: None };
        let metadata = make_metadata(
            Some("12345678-1234-1234-1234-123456789012"),
            Some("USRC12345678"),
            Some("Artist"),
            Some("Title"),
        );

        let result = cascade.resolve_single_track(&metadata).await;
        assert!(result.is_some());
        let res = result.unwrap();
        assert_eq!(res.mbid, "12345678-1234-1234-1234-123456789012");
        assert_eq!(res.tier, ConfidenceTier::Tier1A);
        assert_eq!(res.score, 1.0);
        assert_eq!(res.source, "Embedded MBID");
    }

    #[tokio::test]
    async fn test_resolve_embedded_mbid_only() {
        let cascade = MbidIdentificationCascade { contextual_matcher: None };
        let metadata = make_metadata(
            Some("12345678-1234-1234-1234-123456789012"),
            None,
            Some("Artist"),
            Some("Title"),
        );

        let result = cascade.resolve_single_track(&metadata).await;
        assert!(result.is_some());
        let res = result.unwrap();
        assert_eq!(res.tier, ConfidenceTier::Tier1B);
        assert_eq!(res.score, 0.98);
    }

    #[tokio::test]
    async fn test_resolve_no_embedded_no_contextual() {
        let cascade = MbidIdentificationCascade { contextual_matcher: None };
        let metadata = make_metadata(None, None, Some("Artist"), Some("Title"));

        let result = cascade.resolve_single_track(&metadata).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_album_returns_all_none() {
        let cascade = MbidIdentificationCascade { contextual_matcher: None };
        let metadata = make_metadata(
            Some("12345678-1234-1234-1234-123456789012"),
            None,
            Some("Artist"),
            Some("Album Title"),
        );

        let result = cascade.resolve_album_passages(&metadata, 12).await;
        assert_eq!(result.len(), 12);
        assert!(result.iter().all(|r| r.is_none()));
    }
}
