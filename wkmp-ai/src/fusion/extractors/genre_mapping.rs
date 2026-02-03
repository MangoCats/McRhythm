// Genre → Musical Flavor Characteristics Mapping
//
// PLAN023: CRITICAL-001 Resolution - Maps ID3 genre strings to AcousticBrainz-compatible characteristics
// Confidence: 0.3 (low quality compared to computed features)

use std::collections::HashMap;

/// Helper macro for creating hashmaps
macro_rules! hashmap {
    ($($key:expr => $val:expr),* $(,)?) => {{
        let mut map = HashMap::new();
        $(map.insert($key.to_string(), $val);)*
        map
    }};
}

/// Maps a genre string to musical flavor characteristics
///
/// Returns a HashMap of characteristic paths to values (0.0-1.0).
/// Characteristics are normalized within their categories (sum to 1.0).
///
/// # Arguments
/// * `genre` - ID3 genre string (case-insensitive)
///
/// # Returns
/// * HashMap of "category.subcategory.value" → probability
/// * Empty HashMap if genre not recognized
pub fn map_genre_to_characteristics(genre: &str) -> HashMap<String, f64> {
    let genre_lower = genre.to_lowercase();

    match genre_lower.as_str() {
        // Rock and variants
        "rock" | "classic rock" => hashmap! {
            "danceability.danceable" => 0.3,
            "danceability.not_danceable" => 0.7,
            "mood_aggressive.aggressive" => 0.7,
            "mood_aggressive.not_aggressive" => 0.3,
            "voice_instrumental.instrumental" => 0.2,
            "voice_instrumental.voice" => 0.8,
        },

        "hard rock" | "metal" | "heavy metal" => hashmap! {
            "danceability.danceable" => 0.2,
            "danceability.not_danceable" => 0.8,
            "mood_aggressive.aggressive" => 0.9,
            "mood_aggressive.not_aggressive" => 0.1,
            "voice_instrumental.instrumental" => 0.3,
            "voice_instrumental.voice" => 0.7,
        },

        "punk" | "punk rock" => hashmap! {
            "danceability.danceable" => 0.5,
            "danceability.not_danceable" => 0.5,
            "mood_aggressive.aggressive" => 0.8,
            "mood_aggressive.not_aggressive" => 0.2,
            "voice_instrumental.instrumental" => 0.1,
            "voice_instrumental.voice" => 0.9,
        },

        // Electronic and variants
        "electronic" | "edm" => hashmap! {
            "danceability.danceable" => 0.85,
            "danceability.not_danceable" => 0.15,
            "mood_aggressive.aggressive" => 0.4,
            "mood_aggressive.not_aggressive" => 0.6,
            "voice_instrumental.instrumental" => 0.7,
            "voice_instrumental.voice" => 0.3,
        },

        "house" => hashmap! {
            "danceability.danceable" => 0.95,
            "danceability.not_danceable" => 0.05,
            "mood_aggressive.aggressive" => 0.3,
            "mood_aggressive.not_aggressive" => 0.7,
            "voice_instrumental.instrumental" => 0.8,
            "voice_instrumental.voice" => 0.2,
        },

        "techno" => hashmap! {
            "danceability.danceable" => 0.95,
            "danceability.not_danceable" => 0.05,
            "mood_aggressive.aggressive" => 0.5,
            "mood_aggressive.not_aggressive" => 0.5,
            "voice_instrumental.instrumental" => 0.95,
            "voice_instrumental.voice" => 0.05,
        },

        // Pop
        "pop" => hashmap! {
            "danceability.danceable" => 0.7,
            "danceability.not_danceable" => 0.3,
            "mood_aggressive.aggressive" => 0.2,
            "mood_aggressive.not_aggressive" => 0.8,
            "voice_instrumental.instrumental" => 0.1,
            "voice_instrumental.voice" => 0.9,
        },

        // Jazz
        "jazz" => hashmap! {
            "danceability.danceable" => 0.4,
            "danceability.not_danceable" => 0.6,
            "mood_relaxed.relaxed" => 0.6,
            "mood_relaxed.not_relaxed" => 0.4,
            "voice_instrumental.instrumental" => 0.7,
            "voice_instrumental.voice" => 0.3,
        },

        // Classical
        "classical" | "symphonic" => hashmap! {
            "danceability.danceable" => 0.15,
            "danceability.not_danceable" => 0.85,
            "mood_relaxed.relaxed" => 0.6,
            "mood_relaxed.not_relaxed" => 0.4,
            "voice_instrumental.instrumental" => 0.95,
            "voice_instrumental.voice" => 0.05,
        },

        // Unknown/default: return empty (no assumptions)
        _ => HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genre_mapping_normalization() {
        let genres = vec!["rock", "house", "jazz", "classical"];

        for genre in genres {
            let chars = map_genre_to_characteristics(genre);

            // Group by category
            let mut categories: HashMap<String, Vec<f64>> = HashMap::new();
            for (key, value) in chars {
                let parts: Vec<&str> = key.split('.').collect();
                if parts.len() >= 2 {
                    let category = parts[0].to_string();
                    categories.entry(category).or_insert_with(Vec::new).push(value);
                }
            }

            // Verify each category sums to ~1.0
            for (category, values) in categories {
                let sum: f64 = values.iter().sum();
                assert!(
                    (sum - 1.0).abs() < 0.0001,
                    "Genre '{}' category '{}' does not sum to 1.0 (sum = {})",
                    genre, category, sum
                );
            }
        }
    }

    #[test]
    fn test_unknown_genre_returns_empty() {
        let chars = map_genre_to_characteristics("unknown_genre_xyz");
        assert!(chars.is_empty());
    }

    #[test]
    fn test_case_insensitivity() {
        let lower = map_genre_to_characteristics("rock");
        let upper = map_genre_to_characteristics("ROCK");
        let mixed = map_genre_to_characteristics("RoCk");

        assert_eq!(lower, upper);
        assert_eq!(lower, mixed);
    }
}
