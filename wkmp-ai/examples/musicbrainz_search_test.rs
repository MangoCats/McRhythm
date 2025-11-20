/// Test different MusicBrainz search strategies to improve lookup success
/// Experiments with wildcards, fuzzy matching, and various query patterns

use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Deserialize)]
struct MBSearchResponse {
    releases: Vec<MBRelease>,
}

#[derive(Debug, Deserialize)]
struct MBRelease {
    id: String,
    title: String,
    #[serde(rename = "artist-credit")]
    artist_credit: Option<Vec<MBArtistCredit>>,
    score: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct MBArtistCredit {
    artist: MBArtist,
}

#[derive(Debug, Deserialize)]
struct MBArtist {
    name: String,
}

struct RateLimiter {
    last_request: std::sync::Mutex<std::time::Instant>,
    min_interval: std::time::Duration,
}

impl RateLimiter {
    fn new(requests_per_second: f64) -> Self {
        let min_interval = Duration::from_secs_f64(1.0 / requests_per_second);
        RateLimiter {
            last_request: std::sync::Mutex::new(std::time::Instant::now() - min_interval),
            min_interval,
        }
    }

    async fn wait(&self) {
        let mut last = self.last_request.lock().unwrap();
        let elapsed = last.elapsed();
        if elapsed < self.min_interval {
            let wait_time = self.min_interval - elapsed;
            drop(last);
            tokio::time::sleep(wait_time).await;
            let mut last = self.last_request.lock().unwrap();
            *last = std::time::Instant::now();
        } else {
            *last = std::time::Instant::now();
        }
    }
}

/// Insert spaces before capital letters in CamelCase strings
fn split_camel_case(s: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, &ch) in chars.iter().enumerate() {
        if i > 0 && ch.is_uppercase() && chars[i-1].is_lowercase() {
            result.push(' ');
        }
        result.push(ch);
    }
    result
}

/// Generate multiple search query variants for better matching
fn generate_search_variants(artist: &str, album: &str) -> Vec<(String, String)> {
    let mut variants = Vec::new();

    // Original query
    variants.push((
        format!("artist:{} AND release:{}", artist, album),
        "Original".to_string()
    ));

    // Quoted exact phrases
    variants.push((
        format!("artist:\"{}\" AND release:\"{}\"", artist, album),
        "Quoted exact".to_string()
    ));

    // Fuzzy matching
    variants.push((
        format!("artist:{}~ AND release:{}~", artist, album),
        "Fuzzy".to_string()
    ));

    // CamelCase splitting
    let album_spaced = split_camel_case(album);
    if album_spaced != album {
        variants.push((
            format!("artist:{} AND release:\"{}\"", artist, album_spaced),
            "CamelCase split".to_string()
        ));

        // Also try fuzzy with split
        variants.push((
            format!("artist:{}~ AND release:\"{}\"~", artist, album_spaced),
            "CamelCase split + fuzzy".to_string()
        ));
    }

    // Wildcard matching for album
    if album.len() > 5 {
        variants.push((
            format!("artist:{} AND release:{}*", artist, &album[..album.len().min(10)]),
            "Wildcard album".to_string()
        ));
    }

    // Try without artist if it looks suspicious (might be album name)
    if artist.len() > 15 || artist.contains("The") {
        variants.push((
            format!("release:{}", album),
            "Album only".to_string()
        ));
    }

    // Swap artist/album in case they're reversed
    variants.push((
        format!("artist:{} AND release:{}", album, artist),
        "Swapped artist/album".to_string()
    ));

    variants
}

async fn test_search(
    artist: &str,
    album: &str,
    rate_limiter: &RateLimiter,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Testing: Artist='{}' Album='{}' ===", artist, album);

    let client = reqwest::Client::builder()
        .user_agent("WKMP-MusicBrainzTest/0.1 (https://github.com/yourusername/wkmp)")
        .timeout(Duration::from_secs(30))
        .build()?;

    let variants = generate_search_variants(artist, album);

    for (i, (query, strategy)) in variants.iter().enumerate() {
        rate_limiter.wait().await;

        let encoded_query = urlencoding::encode(query);
        let search_url = format!(
            "https://musicbrainz.org/ws/2/release/?query={}&fmt=json&limit=5",
            encoded_query
        );

        print!("  Strategy {}/{}: {} ... ", i + 1, variants.len(), strategy);

        match client.get(&search_url).send().await {
            Ok(response) => {
                match response.json::<MBSearchResponse>().await {
                    Ok(search_response) => {
                        if search_response.releases.is_empty() {
                            println!("No results");
                        } else {
                            println!("Found {} results", search_response.releases.len());
                            for (j, release) in search_response.releases.iter().take(3).enumerate() {
                                let artist_name = release.artist_credit
                                    .as_ref()
                                    .and_then(|credits| credits.first())
                                    .map(|credit| credit.artist.name.as_str())
                                    .unwrap_or("Unknown");

                                let score = release.score.unwrap_or(0);
                                println!("    {}. [Score: {}] {} - {}",
                                    j + 1, score, artist_name, release.title);
                            }
                        }
                    }
                    Err(e) => println!("Parse error: {}", e),
                }
            }
            Err(e) => println!("Request error: {}", e),
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rate_limiter = RateLimiter::new(0.9); // Slightly under 1 req/sec to be safe

    println!("=== MusicBrainz Search Strategy Testing ===\n");
    println!("Testing different search patterns for albums that failed lookup:\n");

    // Test cases from failed lookups
    let test_cases = vec![
        ("Crosby Stills and Nash", "DaylightAgain"),
        ("Various", "NativeAmericanFluteLullabies"),
        ("Thorpe Billy", "ChildrenOfTheSunRevisited"),
        ("James Gang", "Funk49"),
        ("Journey", "TrialByFire"),
        ("Ace of Base", "HappyNation"),
        ("Thin Lizzie", "LiveAndDangerous"),
        ("Daft Punk", "TronLegacyReconfigured"),
        ("Kraftwerk", "TransEuropeExpress"),
        ("Knack The", "GetTheKnack"),
        ("Men at Work", "BusinessAsUsual"),
        ("Katrina and the Waves", "KatrinaAndTheWaves"),
        ("Police", "ZenyattaMondatta"),
        ("Toto", "TotoIV"),
        ("Buffett Jimmy", "LifeOnTheFlipSide"),
    ];

    for (artist, album) in test_cases {
        if let Err(e) = test_search(artist, album, &rate_limiter).await {
            eprintln!("Error testing {}/{}: {}", artist, album, e);
        }
    }

    println!("\n=== Testing Complete ===");
    Ok(())
}
