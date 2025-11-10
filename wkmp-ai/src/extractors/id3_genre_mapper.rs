//! ID3 Genre Mapper (Tier 1)
//!
//! Maps ID3 genre tags to musical flavor characteristics.
//! Provides heuristic mappings from genre strings to quantitative flavor values.
//!
//! # Implementation
//! - TASK-011: ID3 Genre Mapper (PLAN024)
//! - Confidence: 0.5 (lowest - genre tags subjective/unreliable)
//!
//! # Architecture
//! Implements `SourceExtractor` trait for integration with parallel extraction pipeline.
//! Extracts genre from ID3 tags and maps to musical flavor using predefined genre→flavor mappings.
//!
//! # Confidence Rationale
//! - Genre tags are user-editable and subjective
//! - No standardized genre taxonomy
//! - Often inaccurate or overly broad
//! - Lowest confidence among all extractors (0.5)
//! - Provides complementary data for fusion layer

use crate::types::{
    ExtractionError, ExtractionResult, FlavorExtraction, PassageContext, SourceExtractor,
};
use async_trait::async_trait;
use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use lofty::tag::Accessor;
use std::collections::HashMap;
use std::path::Path;
use tracing::debug;

/// ID3 Genre Mapper
///
/// Maps ID3 genre tags to musical flavor characteristics.
/// Uses predefined mappings from common genres to quantitative flavor values.
///
/// # Confidence
/// Base confidence: 0.5
/// - Genre tags are subjective and user-editable
/// - No standardized genre taxonomy
/// - Often inaccurate or inconsistent
/// - Lowest confidence among all extractors
/// - Still useful for fusion with higher-confidence sources
///
/// # Genre Mappings
/// Maps common genres to characteristics:
/// - Tempo (slow=0.0, fast=1.0)
/// - Energy (calm=0.0, energetic=1.0)
/// - Danceability (0.0-1.0)
/// - Acoustic/Electronic balance
/// - Mood characteristics
///
/// # Example
/// ```rust,ignore
/// use wkmp_ai::extractors::id3_genre_mapper::ID3GenreMapper;
/// use wkmp_ai::types::{SourceExtractor, PassageContext};
///
/// let mapper = ID3GenreMapper::new();
/// let result = mapper.extract(&passage_ctx).await?;
///
/// if let Some(flavor) = result.musical_flavor {
///     println!("Danceability: {}", flavor.characteristics.get("danceability").unwrap_or(&0.0));
/// }
/// ```
pub struct ID3GenreMapper {
    /// Base confidence for genre-derived flavors
    base_confidence: f32,
    /// Genre→Flavor mapping table
    genre_mappings: HashMap<String, GenreCharacteristics>,
}

/// Genre characteristics (flavor values for a specific genre)
#[derive(Debug, Clone)]
struct GenreCharacteristics {
    tempo: f32,         // 0.0 = slow, 1.0 = fast
    energy: f32,        // 0.0 = calm, 1.0 = energetic
    danceability: f32,  // 0.0 = not danceable, 1.0 = very danceable
    acoustic: f32,      // 0.0 = electronic, 1.0 = acoustic
    happy: f32,         // 0.0 = sad, 1.0 = happy
    aggressive: f32,    // 0.0 = peaceful, 1.0 = aggressive
}

impl ID3GenreMapper {
    /// Create new ID3 Genre Mapper with default confidence (0.5)
    pub fn new() -> Self {
        Self {
            base_confidence: 0.5,
            genre_mappings: Self::build_genre_mappings(),
        }
    }

    /// Build genre→flavor mapping table
    ///
    /// These are heuristic mappings based on typical genre characteristics.
    /// Values are approximate and subjective.
    fn build_genre_mappings() -> HashMap<String, GenreCharacteristics> {
        let mut mappings = HashMap::new();

        // Rock/Metal
        mappings.insert("rock".to_string(), GenreCharacteristics {
            tempo: 0.65, energy: 0.75, danceability: 0.50,
            acoustic: 0.30, happy: 0.60, aggressive: 0.50,
        });
        mappings.insert("metal".to_string(), GenreCharacteristics {
            tempo: 0.80, energy: 0.95, danceability: 0.40,
            acoustic: 0.10, happy: 0.40, aggressive: 0.90,
        });
        mappings.insert("punk".to_string(), GenreCharacteristics {
            tempo: 0.85, energy: 0.90, danceability: 0.50,
            acoustic: 0.20, happy: 0.50, aggressive: 0.70,
        });

        // Electronic/Dance
        mappings.insert("electronic".to_string(), GenreCharacteristics {
            tempo: 0.70, energy: 0.75, danceability: 0.80,
            acoustic: 0.05, happy: 0.65, aggressive: 0.30,
        });
        mappings.insert("house".to_string(), GenreCharacteristics {
            tempo: 0.70, energy: 0.75, danceability: 0.90,
            acoustic: 0.00, happy: 0.75, aggressive: 0.20,
        });
        mappings.insert("techno".to_string(), GenreCharacteristics {
            tempo: 0.75, energy: 0.80, danceability: 0.85,
            acoustic: 0.00, happy: 0.60, aggressive: 0.40,
        });
        mappings.insert("trance".to_string(), GenreCharacteristics {
            tempo: 0.75, energy: 0.80, danceability: 0.85,
            acoustic: 0.00, happy: 0.80, aggressive: 0.25,
        });
        mappings.insert("edm".to_string(), GenreCharacteristics {
            tempo: 0.75, energy: 0.85, danceability: 0.90,
            acoustic: 0.00, happy: 0.85, aggressive: 0.30,
        });

        // Hip-Hop/Rap
        mappings.insert("hip-hop".to_string(), GenreCharacteristics {
            tempo: 0.55, energy: 0.70, danceability: 0.75,
            acoustic: 0.10, happy: 0.55, aggressive: 0.50,
        });
        mappings.insert("rap".to_string(), GenreCharacteristics {
            tempo: 0.60, energy: 0.70, danceability: 0.70,
            acoustic: 0.10, happy: 0.50, aggressive: 0.55,
        });

        // Jazz/Blues
        mappings.insert("jazz".to_string(), GenreCharacteristics {
            tempo: 0.50, energy: 0.45, danceability: 0.40,
            acoustic: 0.70, happy: 0.60, aggressive: 0.15,
        });
        mappings.insert("blues".to_string(), GenreCharacteristics {
            tempo: 0.40, energy: 0.40, danceability: 0.35,
            acoustic: 0.60, happy: 0.30, aggressive: 0.20,
        });

        // Classical
        mappings.insert("classical".to_string(), GenreCharacteristics {
            tempo: 0.50, energy: 0.50, danceability: 0.10,
            acoustic: 0.95, happy: 0.50, aggressive: 0.30,
        });

        // Pop
        mappings.insert("pop".to_string(), GenreCharacteristics {
            tempo: 0.65, energy: 0.70, danceability: 0.75,
            acoustic: 0.30, happy: 0.80, aggressive: 0.20,
        });

        // Country
        mappings.insert("country".to_string(), GenreCharacteristics {
            tempo: 0.55, energy: 0.55, danceability: 0.50,
            acoustic: 0.70, happy: 0.65, aggressive: 0.15,
        });

        // Reggae
        mappings.insert("reggae".to_string(), GenreCharacteristics {
            tempo: 0.40, energy: 0.50, danceability: 0.65,
            acoustic: 0.50, happy: 0.75, aggressive: 0.10,
        });

        // Folk/Acoustic
        mappings.insert("folk".to_string(), GenreCharacteristics {
            tempo: 0.45, energy: 0.40, danceability: 0.30,
            acoustic: 0.90, happy: 0.60, aggressive: 0.10,
        });
        mappings.insert("acoustic".to_string(), GenreCharacteristics {
            tempo: 0.45, energy: 0.35, danceability: 0.25,
            acoustic: 0.95, happy: 0.55, aggressive: 0.05,
        });

        // R&B/Soul
        mappings.insert("r&b".to_string(), GenreCharacteristics {
            tempo: 0.50, energy: 0.55, danceability: 0.65,
            acoustic: 0.30, happy: 0.60, aggressive: 0.20,
        });
        mappings.insert("soul".to_string(), GenreCharacteristics {
            tempo: 0.50, energy: 0.55, danceability: 0.60,
            acoustic: 0.40, happy: 0.65, aggressive: 0.15,
        });

        // Ambient/Chill
        mappings.insert("ambient".to_string(), GenreCharacteristics {
            tempo: 0.20, energy: 0.20, danceability: 0.10,
            acoustic: 0.20, happy: 0.50, aggressive: 0.05,
        });
        mappings.insert("chillout".to_string(), GenreCharacteristics {
            tempo: 0.30, energy: 0.30, danceability: 0.20,
            acoustic: 0.30, happy: 0.70, aggressive: 0.05,
        });

        mappings
    }

    /// Extract genre from audio file
    ///
    /// # Arguments
    /// * `file_path` - Path to audio file
    ///
    /// # Returns
    /// Genre string (lowercase, normalized)
    ///
    /// # Errors
    /// Returns error if file cannot be read or has no genre tag
    fn extract_genre(&self, file_path: &Path) -> Result<String, ExtractionError> {
        // Probe file to determine format
        let tagged_file = Probe::open(file_path)
            .map_err(|e| ExtractionError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?
            .read()
            .map_err(|e| {
                ExtractionError::Parse(format!("Failed to read audio file tags: {}", e))
            })?;

        // Get primary tag
        let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());

        let Some(tag) = tag else {
            return Err(ExtractionError::NotAvailable(
                "No tags found in audio file".to_string(),
            ));
        };

        // Extract genre
        let Some(genre_str) = tag.genre() else {
            return Err(ExtractionError::NotAvailable(
                "No genre tag found".to_string(),
            ));
        };

        // Normalize: lowercase, trim
        let genre_normalized = genre_str.to_lowercase().trim().to_string();

        debug!(
            file_path = ?file_path,
            genre = %genre_normalized,
            "Extracted genre from ID3 tags"
        );

        Ok(genre_normalized)
    }

    /// Map genre to musical flavor
    ///
    /// # Arguments
    /// * `genre` - Genre string (normalized)
    ///
    /// # Returns
    /// Musical flavor characteristics based on genre mapping
    fn map_genre_to_flavor(&self, genre: &str) -> FlavorExtraction {
        // Try exact match first
        if let Some(characteristics) = self.genre_mappings.get(genre) {
            return self.characteristics_to_flavor(characteristics, genre);
        }

        // Try substring matching for compound genres (e.g., "heavy metal" → "metal")
        for (known_genre, characteristics) in &self.genre_mappings {
            if genre.contains(known_genre.as_str()) {
                debug!(
                    genre = %genre,
                    matched = %known_genre,
                    "Matched genre by substring"
                );
                return self.characteristics_to_flavor(characteristics, genre);
            }
        }

        // No match found - return neutral characteristics
        debug!(genre = %genre, "Unknown genre, using neutral characteristics");
        let neutral = GenreCharacteristics {
            tempo: 0.5,
            energy: 0.5,
            danceability: 0.5,
            acoustic: 0.5,
            happy: 0.5,
            aggressive: 0.5,
        };
        self.characteristics_to_flavor(&neutral, genre)
    }

    /// Convert GenreCharacteristics to FlavorExtraction
    fn characteristics_to_flavor(
        &self,
        characteristics: &GenreCharacteristics,
        genre: &str,
    ) -> FlavorExtraction {
        let mut flavor_map = HashMap::new();

        flavor_map.insert("genre_tempo".to_string(), characteristics.tempo);
        flavor_map.insert("genre_energy".to_string(), characteristics.energy);
        flavor_map.insert("genre_danceability".to_string(), characteristics.danceability);
        flavor_map.insert("genre_acoustic".to_string(), characteristics.acoustic);
        flavor_map.insert("genre_happy".to_string(), characteristics.happy);
        flavor_map.insert("genre_aggressive".to_string(), characteristics.aggressive);

        FlavorExtraction {
            characteristics: flavor_map,
            confidence: self.base_confidence,
            source: format!("ID3Genre:{}", genre),
        }
    }
}

impl Default for ID3GenreMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SourceExtractor for ID3GenreMapper {
    fn name(&self) -> &'static str {
        "ID3Genre"
    }

    fn base_confidence(&self) -> f32 {
        self.base_confidence
    }

    async fn extract(&self, ctx: &PassageContext) -> Result<ExtractionResult, ExtractionError> {
        debug!(
            passage_id = %ctx.passage_id,
            file_path = ?ctx.file_path,
            "Mapping ID3 genre to musical flavor"
        );

        let genre = self.extract_genre(&ctx.file_path)?;
        let flavor = self.map_genre_to_flavor(&genre);

        debug!(
            passage_id = %ctx.passage_id,
            genre = %genre,
            feature_count = flavor.characteristics.len(),
            "ID3 genre mapping complete"
        );

        Ok(ExtractionResult {
            metadata: None,
            identity: None,
            musical_flavor: Some(flavor),
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use uuid::Uuid;

    #[test]
    fn test_mapper_name() {
        let mapper = ID3GenreMapper::new();
        assert_eq!(mapper.name(), "ID3Genre");
    }

    #[test]
    fn test_default_confidence() {
        let mapper = ID3GenreMapper::new();
        assert_eq!(mapper.base_confidence(), 0.5);
    }

    #[test]
    fn test_default_trait() {
        let mapper = ID3GenreMapper::default();
        assert_eq!(mapper.base_confidence(), 0.5);
    }

    #[test]
    fn test_genre_mappings_exist() {
        let mapper = ID3GenreMapper::new();
        assert!(!mapper.genre_mappings.is_empty(), "Should have genre mappings");
        assert!(mapper.genre_mappings.contains_key("rock"));
        assert!(mapper.genre_mappings.contains_key("metal"));
        assert!(mapper.genre_mappings.contains_key("electronic"));
        assert!(mapper.genre_mappings.contains_key("jazz"));
        assert!(mapper.genre_mappings.contains_key("classical"));
    }

    #[test]
    fn test_map_known_genre() {
        let mapper = ID3GenreMapper::new();
        let flavor = mapper.map_genre_to_flavor("rock");

        assert_eq!(flavor.confidence, 0.5);
        assert!(flavor.source.contains("rock"));
        assert!(flavor.characteristics.contains_key("genre_energy"));
        assert!(flavor.characteristics.contains_key("genre_danceability"));
    }

    #[test]
    fn test_map_unknown_genre() {
        let mapper = ID3GenreMapper::new();
        let flavor = mapper.map_genre_to_flavor("unknown_genre_xyz");

        assert_eq!(flavor.confidence, 0.5);
        // Should return neutral values (0.5) for unknown genre
        assert_eq!(flavor.characteristics.get("genre_tempo"), Some(&0.5));
        assert_eq!(flavor.characteristics.get("genre_energy"), Some(&0.5));
    }

    #[test]
    fn test_map_compound_genre() {
        let mapper = ID3GenreMapper::new();
        // "heavy metal" should match "metal"
        let flavor = mapper.map_genre_to_flavor("heavy metal");

        assert!(flavor.source.contains("heavy metal"));
        // Should have high energy/aggressive characteristics from metal
        let energy = flavor.characteristics.get("genre_energy").unwrap();
        let aggressive = flavor.characteristics.get("genre_aggressive").unwrap();
        assert!(*energy > 0.8, "Metal should have high energy");
        assert!(*aggressive > 0.7, "Metal should be aggressive");
    }

    #[test]
    fn test_genre_characteristics_ranges() {
        let mapper = ID3GenreMapper::new();

        // Test that all values are in 0-1 range
        for (genre, characteristics) in &mapper.genre_mappings {
            assert!(
                characteristics.tempo >= 0.0 && characteristics.tempo <= 1.0,
                "Genre {} tempo out of range",
                genre
            );
            assert!(
                characteristics.energy >= 0.0 && characteristics.energy <= 1.0,
                "Genre {} energy out of range",
                genre
            );
            assert!(
                characteristics.danceability >= 0.0 && characteristics.danceability <= 1.0,
                "Genre {} danceability out of range",
                genre
            );
        }
    }

    #[tokio::test]
    async fn test_extract_nonexistent_file() {
        let mapper = ID3GenreMapper::new();
        let ctx = PassageContext {
            passage_id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            file_path: PathBuf::from("/nonexistent/file.mp3"),
            start_time_ticks: 0,
            end_time_ticks: 1000000,
            audio_samples: None,
            sample_rate: Some(44100),
            num_channels: Some(2),
            import_session_id: Uuid::new_v4(),
        };

        let result = mapper.extract(&ctx).await;
        assert!(result.is_err(), "Should fail for nonexistent file");
    }

    // Note: Testing with real audio files requires test fixtures
    // Integration tests with test audio files should be added separately
}
