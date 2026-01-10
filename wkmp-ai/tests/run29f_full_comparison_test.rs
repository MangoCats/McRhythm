//! Run29f Full Baseline Comparison Test (200 Albums)
//!
//! **Purpose:** Comprehensive validation of current album matching against run29f baseline
//!
//! Tests all 200 albums from run29f and reports any differences in:
//! - Expected track count
//! - Matched MBID
//! - Match success/failure
//!
//! Results are written to JSON file for detailed analysis.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::info;
use wkmp_ai::matching::album_matcher::AlbumMatcher;
use wkmp_ai::services::{MusicBrainzClient, AcousticBrainzClient};
use wkmp_ai::matching::types::AlbumMatchResult;

/// Get path to Music library
fn get_music_library_path() -> PathBuf {
    let library = std::env::var("WKMP_TEST_LIBRARY").unwrap_or_else(|_| {
        if cfg!(windows) {
            r"C:\Users\Mango Cat\Music".to_string()
        } else {
            std::env::var("HOME")
                .map(|h| format!("{}/Music", h))
                .unwrap_or_else(|_| "/tmp".to_string())
        }
    });
    PathBuf::from(library)
}

/// Initialize tracing subscriber with file-based logging for real-time progress
fn init_tracing() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        // Create timestamped log file
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let log_path = format!("test_run29f_full_{}.log", timestamp);

        let log_file = std::fs::File::create(&log_path)
            .expect("Failed to create log file");

        tracing_subscriber::fmt()
            .with_writer(log_file)
            .with_ansi(false)  // No color codes in file
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "wkmp_ai=debug".into()),  // Changed to debug for Stage 6 visibility
            )
            .try_init()
            .ok();

        // Print to stderr so it's visible immediately
        eprintln!("{}", "=".repeat(80));
        eprintln!("Test progress being written to: {}", log_path);
        eprintln!("Monitor in real-time with: Get-Content {} -Wait -Tail 50", log_path);
        eprintln!("{}", "=".repeat(80));
    });
}

/// Create persistent database pool for MusicBrainz caching
async fn create_persistent_db_pool() -> Result<sqlx::SqlitePool> {
    use std::fs;

    let cache_dir = PathBuf::from(".cache");
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir)?;
    }

    let db_path = cache_dir.join("run29f_full_test.db");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    let pool = sqlx::SqlitePool::connect(&db_url).await?;
    wkmp_ai::db::release_cache::ensure_tables(&pool).await?;

    Ok(pool)
}

#[derive(Debug, Serialize, Deserialize)]
struct TrackDetail {
    track_number: usize,
    title: String,
    recording_mbid: String,
    mb_duration_ms: u64,  // MusicBrainz duration in milliseconds
    our_duration_ms: u64,  // Our segmentation duration in milliseconds
    timing_error_ms: i64,  // Difference (our - MB) in milliseconds
    within_tolerance: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ReleaseMetadata {
    release_mbid: String,
    title: String,
    artist: String,
    release_date: Option<String>,
    country: Option<String>,
    label: Option<String>,
    catalog_number: Option<String>,
    barcode: Option<String>,
    format: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ComparisonResult {
    path: String,
    artist: String,
    album: String,
    baseline_tracks: usize,
    baseline_mbid: String,
    current_tracks: Option<usize>,
    current_mbid: Option<String>,
    matched: bool,
    match_percentage: f64,
    tracks_match: bool,
    mbid_match: bool,
    skipped: bool,
    error: Option<String>,
    // AcousticBrainz data availability
    acousticbrainz_total_tracks: usize,
    acousticbrainz_available: usize,
    acousticbrainz_missing: usize,
    // Detailed track listing
    tracks: Vec<TrackDetail>,
    // Release metadata from MusicBrainz
    release_metadata: Option<ReleaseMetadata>,
}

/// Extract track details from match result
fn extract_track_details(match_result: &wkmp_ai::matching::types::AlbumMatchResult) -> Vec<TrackDetail> {
    match_result.tracks.iter().map(|track| {
        TrackDetail {
            track_number: track.track_number,
            title: track.title.clone(),
            recording_mbid: track.recording_mbid.clone(),
            mb_duration_ms: (track.expected_duration * 1000.0) as u64,
            our_duration_ms: (track.detected_duration * 1000.0) as u64,
            timing_error_ms: ((track.detected_duration - track.expected_duration) * 1000.0) as i64,
            within_tolerance: track.within_tolerance,
        }
    }).collect()
}

/// Fetch release metadata from MusicBrainz
async fn fetch_release_metadata(
    client: &wkmp_ai::services::MusicBrainzClient,
    mbid: &str
) -> Option<ReleaseMetadata> {
    match client.lookup_release(mbid).await {
        Ok(release) => {
            // Extract label and catalog number from first label-info
            let (label, catalog_number) = if let Some(label_info) = &release.label_info {
                if let Some(first) = label_info.first() {
                    (
                        first.label.as_ref().map(|l| l.name.clone()),
                        first.catalog_number.clone()
                    )
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            // Extract format from first medium
            let format = release.media.first()
                .and_then(|m| m.format.clone());

            // Extract artist name from artist credits
            let artist = release.artist_credit
                .as_ref()
                .map(|credits| {
                    credits.iter()
                        .map(|c| c.name.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "Unknown Artist".to_string());

            Some(ReleaseMetadata {
                release_mbid: mbid.to_string(),
                title: release.title.clone(),
                artist,
                release_date: release.date.clone(),
                country: release.country.clone(),
                label,
                catalog_number,
                barcode: release.barcode.clone(),
                format,
                status: release.status.clone(),
            })
        }
        Err(e) => {
            eprintln!("Warning: Failed to fetch release metadata for {}: {:?}", mbid, e);
            None
        }
    }
}

/// Format release metadata (format, status, packaging, etc.)
fn format_release_metadata(release: &wkmp_ai::matching::types::MBReleaseDetails) -> Vec<String> {
    let mut metadata = Vec::new();

    // Format(s) from media
    let formats: Vec<String> = release.media.iter()
        .filter_map(|m| m.format.clone())
        .collect();
    if !formats.is_empty() {
        metadata.push(format!("Format: {}", formats.join(", ")));
    }

    // Status
    if let Some(status) = &release.status {
        metadata.push(format!("Status: {}", status));
    }

    // Packaging
    if let Some(packaging) = &release.packaging {
        metadata.push(format!("Packaging: {}", packaging));
    }

    // Language
    if let Some(text_rep) = &release.text_representation {
        if let Some(language) = &text_rep.language {
            metadata.push(format!("Language: {}", language));
        }
    }

    // Labels
    if let Some(label_info) = &release.label_info {
        let labels: Vec<String> = label_info.iter()
            .filter_map(|li| {
                li.label.as_ref().map(|l| {
                    if let Some(cat) = &li.catalog_number {
                        format!("{} ({})", l.name, cat)
                    } else {
                        l.name.clone()
                    }
                })
            })
            .collect();
        if !labels.is_empty() {
            metadata.push(format!("Label(s): {}", labels.join("; ")));
        }
    }

    // Release Events
    if let Some(events) = &release.release_events {
        let events_str: Vec<String> = events.iter()
            .map(|e| {
                let country = e.area.as_ref().map(|a| a.name.as_str()).unwrap_or("Unknown");
                let date = e.date.as_deref().unwrap_or("Unknown date");
                format!("{} ({})", country, date)
            })
            .collect();
        if !events_str.is_empty() {
            metadata.push(format!("Release Events: {}", events_str.join("; ")));
        }
    }

    metadata
}

/// Query AcousticBrainz for all recordings in a release
///
/// Returns (total_tracks, available_count, missing_count, recording_mbids_with_ab_data)
async fn query_acousticbrainz_for_release(
    ab_client: &AcousticBrainzClient,
    mb_client: &MusicBrainzClient,
    release_mbid: &str,
) -> Result<(usize, usize, usize, Vec<String>)> {
    // Fetch release details to get recording MBIDs
    let release = mb_client.lookup_release(release_mbid).await?;

    let mut total_recordings = 0;
    let mut available = 0;
    let mut missing = 0;
    let mut available_mbids = Vec::new();

    // Iterate through all media and tracks
    for medium in &release.media {
        for track in &medium.tracks {
            // Skip tracks without recording information
            if let Some(recording) = &track.recording {
                total_recordings += 1;

                // Query AcousticBrainz for this recording
                match ab_client.lookup_lowlevel(&recording.id).await {
                    Ok(_) => {
                        available += 1;
                        available_mbids.push(recording.id.clone());
                    }
                    Err(_) => {
                        missing += 1;
                    }
                }
            }
        }
    }

    Ok((total_recordings, available, missing, available_mbids))
}

/// Display detailed track listing for album match comparison
async fn display_album_details(
    mb_client: &MusicBrainzClient,
    baseline_artist: &str,
    baseline_album: &str,
    baseline_mbid: &str,
    new_result: &AlbumMatchResult,
    mbid_changed: bool,
) {
    info!("");
    info!("  ═══════════════════════════════════════════════════════════════════════════");
    if mbid_changed {
        info!("  ⚠ MBID CHANGED - Track Listing Comparison");
    } else {
        info!("  ✓ EXACT MATCH - Album Details");
    }
    info!("  ═══════════════════════════════════════════════════════════════════════════");

    // Fetch baseline release details from MusicBrainz
    match mb_client.lookup_release(baseline_mbid).await {
        Ok(baseline_release) => {
            // Display baseline (old) track listing
            println!("\n  📀 BASELINE (run29f): {} - {}", baseline_artist, baseline_album);
            println!("     Release MBID: {}", baseline_mbid);
            println!("     Artist: {}", baseline_release.artist_credit
                .as_ref()
                .and_then(|credits| credits.first())
                .map(|c| c.name.as_str())
                .unwrap_or(baseline_artist));
            println!("     Album: {}", baseline_release.title);

            // Display metadata
            let metadata = format_release_metadata(&baseline_release);
            for item in &metadata {
                println!("     {}", item);
            }

            let mut baseline_total_ms: u64 = 0;
            let mut track_num = 1;

            println!("\n     Tracks:");
            for medium in &baseline_release.media {
                for track in &medium.tracks {
                    let duration_ms = track.length.unwrap_or(0);
                    baseline_total_ms += duration_ms as u64;
                    let duration_secs = duration_ms as f64 / 1000.0;
                    println!("       {:2}. {:50} {:>7.2}s",
                        track_num,
                        track.title,
                        duration_secs);
                    track_num += 1;
                }
            }
            println!("     ───────────────────────────────────────────────────────────────────");
            println!("     Total: {} tracks, {:>7.2}s ({:02}:{:02}:{:02})",
                track_num - 1,
                baseline_total_ms as f64 / 1000.0,
                (baseline_total_ms / 1000) / 3600,
                ((baseline_total_ms / 1000) % 3600) / 60,
                (baseline_total_ms / 1000) % 60);

            // Fetch new match release details for metadata
            let new_match_mbid = new_result.release_mbid.as_deref().unwrap_or("");
            match mb_client.lookup_release(new_match_mbid).await {
                Ok(new_release) => {
                    // Display new match track listing
                    println!("\n  🆕 NEW MATCH: {} - {}",
                        new_result.matched_artist.as_deref().unwrap_or("Unknown"),
                        new_result.matched_album.as_deref().unwrap_or("Unknown"));
                    println!("     Release MBID: {}", new_match_mbid);

                    // Display metadata
                    let metadata = format_release_metadata(&new_release);
                    for item in &metadata {
                        println!("     {}", item);
                    }

                    let mut new_total_ms: u64 = 0;

                    println!("\n     Tracks:");
                    for (idx, track) in new_result.tracks.iter().enumerate() {
                        let duration_ms = (track.expected_duration * 1000.0) as u64;
                        new_total_ms += duration_ms;
                        println!("       {:2}. {:50} {:>7.2}s (detected: {:>7.2}s)",
                            idx + 1,
                            track.title,
                            track.expected_duration,
                            track.detected_duration);
                    }
                    println!("     ───────────────────────────────────────────────────────────────────");
                    println!("     Total: {} tracks, {:>7.2}s ({:02}:{:02}:{:02})",
                        new_result.expected_track_count,
                        new_total_ms as f64 / 1000.0,
                        (new_total_ms / 1000) / 3600,
                        ((new_total_ms / 1000) % 3600) / 60,
                        (new_total_ms / 1000) % 60);

                    // Show duration difference
                    let duration_diff = new_total_ms as i64 - baseline_total_ms as i64;
                    println!("\n  📊 Duration Difference: {:+.2}s ({:+02}:{:02}:{:02})",
                        duration_diff as f64 / 1000.0,
                        duration_diff.abs() / 3600000,
                        (duration_diff.abs() / 60000) % 60,
                        (duration_diff.abs() / 1000) % 60);
                    println!("  ═══════════════════════════════════════════════════════════════════════════\n");
                }
                Err(e) => {
                    println!("  ⚠ Could not fetch new match release details: {:?}", e);
                    println!("  New Match MBID: {}\n", new_match_mbid);
                }
            }
        }
        Err(e) => {
            println!("  ⚠ Could not fetch baseline release details: {:?}", e);
            println!("  MBID: {} -> {}\n", baseline_mbid, new_result.release_mbid.as_deref().unwrap_or(""));
        }
    }
}

/// Run29f full baseline comparison (200 albums)
#[tokio::test]
#[ignore = "Requires actual music library and takes ~6 hours first run, ~40 min cached"]
async fn test_run29f_full_baseline_comparison() -> Result<()> {
    init_tracing();

    info!("");
    info!("=== Run29f Full Baseline Comparison (200 Albums) ===");
    info!("");

    // Create persistent database pool for caching
    let db_pool = create_persistent_db_pool().await?;
    info!("✓ Database pool initialized (.cache/run29f_full_test.db)");
    info!("");

    let music_lib = get_music_library_path();
    info!("Music Library: {:?}", music_lib);
    info!("");

    // run29f baseline: 200 albums
    #[rustfmt::skip]
    let baseline = vec![
        ("38 Special/Anthology.mp3", "38 Special", "Anthology", 34, "4a81fc08-f915-49fc-9aa2-5b708c18f07a"),
        ("Ace of Base/HappyNation.mp3", "Ace of Base", "Happy Nation (U.S. Version) (Remastered)", 16, "142b09aa-a4ed-49dd-9c66-adf5e2f865c1"),
        ("Aerosmith/Pump.mp3", "Aerosmith", "Pump", 10, "5a9be9a5-9efe-43c4-9687-02f80f6461ba"),
        ("Alice In Chains/AliceInChainsGreatestHits.mp3", "Alice In Chains", "Greatest Hits", 10, "37cc6812-0779-496a-b9d8-19fd69e4b2c5"),
        ("Allman Brothers/AtFillmoreEast.mp3", "The Allman Brothers", "At Fillmore East", 13, "29e18ce2-3777-432b-bff6-c446e71b21e7"),
        ("Allman Brothers/EatAPeach.mp3", "Allman Brothers", "Eat a Peach", 18, "bf8885b2-39f8-344e-b860-4be1623de283"),
        ("Ambrosia/TheEssentialsAmbrosia.mp3", "Ambrosia", "The Essentials: Ambrosia", 12, "8db525cb-cda0-4c72-b0af-91a50fd09a37"),
        ("Asia/GoldAsia.mp3", "Asia", "Gold", 36, "155033f7-f321-4ee8-8a24-513b44fd7509"),
        ("BTS/Wings.mp3", "BTS", "Wings", 15, "5365a8ab-8ec5-4e2d-86ab-e985c27c1947"),
        ("Bears' Den/Islands.mp3", "Bear's Den", "Islands", 20, "c23fc400-490b-470d-a379-7821982253c0"),
        ("Beck/Hyperspace.mp3", "Beck", "Hyperspace", 11, "e5527cce-55b1-4c2c-b7db-a633f32dd614"),
        ("Bjork/BodyTalk.mp3", "Robyn", "Body Talk", 15, "afebe204-c664-474e-8bbc-4a4f49a7025c"),
        ("Bjork/Debut.mp3", "Bjork", "Debut", 11, "9a1b3b38-95ae-3a37-852d-6fed0d540109"),
        ("Bjork/Vespertine.mp3", "Bjork", "Vespertine", 12, "29cc7eff-fd8b-4cee-89a2-f36c923086f7"),
        ("Bjork/Volta.mp3", "Bjork", "Volta", 11, "5deb5979-9db8-4967-88f8-a058ba23062b"),
        ("Blackmore's Night/BeyondTheSunset.mp3", "Blackmore's Night", "Beyond the Sunset", 17, "447a9ccd-7ef9-4356-8656-fea20cc746d6"),
        ("Bon Jovi/BonJovi.mp3", "Bon Jovi", "Bon Jovi", 9, "063d9008-65b5-4f94-803c-c5510928c7d0"),
        ("Brubeck, Dave/TheBestOfTheDaveBrubeckQuartet.mp3", "Dave Brubeck Quartet", "The Best Of The Dave Brubeck Quartet (1979-2004)", 20, "7fce8bff-0c91-467e-858d-7cca5e0901db"),
        ("Brubeck, Dave/We're All Together Again For The First Time/01 - Truth.mp3", "Dave Brubeck", "We're All Together Again for the First Time", 0, ""),
        ("Brubeck, Dave/We're All Together Again For The First Time/04 - Take Five.mp3", "Dave Brubeck", "We're All Together Again for the First Time", 0, ""),
        ("Buffett, Jimmy/Banana Wind/12 - False Echoes.mp3", "Jimmy Buffett", "Banana Wind", 0, ""),
        ("Buffett, Jimmy/LifeOnTheFlipSide.mp3", "Jimmy Buffett", "Life On The Flipside", 14, "6c8e0f64-77bd-4fef-9f9d-0665a782ed18"),
        ("Cars/HeartbeatCity.mp3", "The Cars", "Heartbeat City", 10, "589545a8-8b6a-4d24-ba51-b46b4ba11cf6"),
        ("Cars/Panorama.mp3", "The Cars", "Panorama", 10, "48cf3a7a-e8b8-4d3a-940c-9ca1219d45f9"),
        ("Chemical Brothers/DigYourOwnHole.mp3", "Chemical Brothers", "Dig Your Own Hole", 11, "4d4a7479-89bd-4bb8-849e-1fef1ecbbe2e"),
        ("Chemical Brothers/Further.mp3", "The Chemical Brothers", "Further", 9, "f1a30f02-57ad-4a3d-9d9d-47d2b778fab4"),
        ("Chemical Brothers/Surrender.mp3", "The Chemical Brothers", "Surrender", 28, "7898f198-d463-4a37-8299-335118aa359a"),
        ("Chicago/OnlyTheBeginning.mp3", "Chicago", "The Very Best of Chicago: Only the Beginning", 39, "bbe24e96-6989-4f53-8135-0d423dc84723"),
        ("Chumbawumba/Tubthumper.mp3", "Chumbawamba", "Tubthumper", 12, "3f9a83ce-7ad0-4837-bf3e-4e3c4085cbf2"),
        ("Clapton, Eric/LaylaAndOtherAssortedLoveSongs.mp3", "Derek & The Dominos", "Layla And Other Assorted Love Songs", 14, "fe1a6c3e-7f2e-4ab1-aaa8-8dbfa79593e3"),
        ("Corea, Chick/LightAsAFeather.mp3", "Chick Corea", "Light as a Feather", 6, "3891a371-d614-400a-951e-49128e74cad4"),
        ("Costello, Elvis/TheBestOfTheFirst10Years.mp3", "Elvis Costello", "The Best of The First 10 Years", 22, "e2a79ba0-3bea-4e31-a171-b232b58cf4d3"),
        ("Crosby, Stills and Nash/DaylightAgain.mp3", "Crosby, Stills & Nash", "Daylight Again (Deluxe Version 2012)", 15, "4cfb8db4-c10d-4f18-bfb8-80ac3fc234bc"),
        ("Cross, Christopher/ChristopherCross.mp3", "Christopher Cross", "Christopher Cross", 9, "cad93e08-5ebf-4b55-b2a0-73f5a594e9fb"),
        ("Crystal Method, The/Tweekend.mp3", "The Crystal Method", "Tweekend", 12, "6f07e2c1-9dba-40eb-bfc2-d0324431eea6"),
        ("Crystal Method, The/Vegas.mp3", "The Crystal Method", "Vegas", 10, "819e253c-575c-4b3c-9bdb-8a0cfc1a8bb0"),
        ("Daft Punk/Homework.mp3", "Daft Punk", "Homework", 16, "ab98d218-73ea-45bd-a050-3902eb683d3f"),
        ("Daft Punk/TronLegacyReconfigured.mp3", "Daft Punk", "TRON: Legacy Reconfigured", 15, "ef6ad525-3595-430b-a0a4-f080054ff2c1"),
        ("Deep Purple/MachineHead.mp3", "Deep Purple", "Machine Head", 7, "e72bf4f9-8451-44d8-9289-ffabae1ed255"),
        ("Def Leppard/Pyromania.mp3", "Def Leppard", "Pyromania", 25, "995799b9-a4bc-4f2f-b5fe-25441e6ab6bd"),
        ("Devo/FreedomOfChoice.mp3", "Devo", "Freedom Of Choice", 12, "977d7c01-a90c-4815-b01e-4b0fbb139a6c"),
        ("Dido/StillOnMyMind.mp3", "Dido", "Still On My Mind (Deluxe Edition)", 20, "674cdedb-f7b2-4608-923f-ca374d28fae7"),
        ("Dolby, Thomas/AMapOfTheFloatingCity.mp3", "Thomas Dolby", "A Map Of The Floating City", 11, "2bba6e34-e429-4a8e-8f23-b99588167ebd"),
        ("Dolby, Thomas/AliensAteMyBuick.mp3", "Thomas Dolby", "Aliens Ate My Buick", 8, "7f0f6b34-411d-4435-93d4-1c7efe8c6ee5"),
        ("Doobie Brothers/MinuteByMinute.mp3", "The Doobie Brothers", "Minute By Minute", 10, "3109e073-08d3-46c4-a8cb-39c16653bc0b"),
        ("Doobie Brothers/TakinItToTheStreets.mp3", "The Doobie Brothers", "Takin' It to the Streets", 9, "e3242055-fa53-4623-9bf3-f33ff060c93c"),
        ("Doobie Brothers/TheCaptainAndMe.mp3", "The Doobie Brothers", "The Captain And Me", 11, "1cb672c8-787e-42a8-b92d-996c92a1d529"),
        ("Doobie Brothers/ToulouseStreet.mp3", "The Doobie Brothers", "Toulouse Street", 10, "da36e2c5-b226-4c77-a14c-3ea509c251c4"),
        ("Duran Duran/Rio.mp3", "Duran Duran", "Rio", 29, "681c9f3a-9132-48a3-8711-a5e070df5cb5"),
        ("Eagles/Desperado.mp3", "Eagles", "Desperado", 11, "b969689e-e2bf-344a-bcfb-b68a8cb2822a"),
        ("Eagles/Eagles.mp3", "Eagles", "Eagles (2013 Remaster)", 10, "ae4862d5-3c1c-44dd-bd50-9e2858f6a17e"),
        ("Eagles/OnTheBorder.mp3", "Eagles", "On the Border", 10, "c9f395b1-95ba-409a-a43d-a282f648435d"),
        ("Eagles/OneOfTheseNights.mp3", "Eagles", "One of These Nights", 9, "f3f15edb-2e64-4432-a75a-6f76ab73784c"),
        ("Eagles/TheLongRun.mp3", "Eagles", "The Long Run", 10, "59870c2f-e9e0-42bf-9399-2ef13896127c"),
        ("Estefan, Gloria/GreatestHits.mp3", "Gloria Estefan", "Greatest Hits", 14, "ad89c7d0-c457-4df1-b9d4-517ac37f4cec"),
        ("Fatboy Slim/WhyTryHarder.mp3", "Fatboy Slim", "The Greatest Hits - Why Try Harder", 18, "148ca65a-b481-3f76-8f76-c120efb3a5fb"),
        ("Fish, Samantha/BlackWindHowlin.mp3", "Samantha Fish", "Black Wind Howlin'", 12, "5d92407c-5235-495e-b4fa-22ef3dc7ef85"),
        ("Fixx/BeautifulFriction.mp3", "The Fixx", "Beautiful Friction", 11, "158283ff-17d3-4416-b05c-df4f2123cc5b"),
        ("Fluke/Oto.mp3", "Fluke", "Oto", 8, "ce2229e5-e521-4f17-a3e4-7634a539b0f3"),
        ("Fluke/Puppy.mp3", "Fluke", "Puppy", 0, ""),
        ("Fluke/Risotto.mp3", "Fluke", "Risotto", 10, "516fe470-06d5-4221-b84f-e563124f0f31"),
        ("Fluke/SixWheelsOnMyWagon.mp3", "Fluke", "Six Wheels On My Wagon", 12, "f2509a40-2602-44cd-9ed8-dc1765b8d02e"),
        ("Foghat/FoolForTheCity.mp3", "Foghat", "Fool For The City", 7, "879e7805-75a8-410a-acf4-d510825bfde1"),
        ("Foreigner/4.mp3", "Foreigner", "4", 10, "a7ca4cfe-8cab-4e8f-a746-cd18cb5dc904"),
        ("Foreigner/DoubleVision.mp3", "Foreigner", "Double Vision", 12, "d9621e1c-3717-496a-aef8-d339b00defe4"),
        ("Foreigner/Foreigner.mp3", "Foreigner", "Foreigner (Expanded)", 14, "cc98c8d2-839e-4923-b32a-138ff4301a31"),
        ("Foreigner/HeadGames.mp3", "Foreigner", "Head Games", 11, "051ce00a-69d6-4120-be71-2c51e309cf44"),
        ("Frampton, Peter/FramptonComesAlive.mp3", "Peter Frampton", "Frampton Comes Alive", 14, "db5756fa-91e2-46e6-bc64-b6c3e1fd2232"),
        ("Georgia Satellites/Essentials-GeorgiaSatellites.mp3", "Georgia Satellites", "Essentials", 12, "86b62f25-1bab-41d8-b197-bc3176457f7f"),
        ("Go Gos, The/BeautyAndTheBeat.mp3", "The Go-Go's", "Beauty And The Beat", 11, "dc9722f2-f176-4d4c-9740-4e49fee401db"),
        ("Golden Earring/Cut.mp3", "Golden Earring", "Cut", 8, "caefa0d5-846f-3e6d-bbda-9631c6c9a46c"),
        ("Grand Funk Railroad/CloserToHome.mp3", "Grand Funk Railroad", "Closer To Home", 13, "94a2fff5-3143-4baa-aa6a-ce3eae8002e8"),
        ("Groove Armada/WhiteLight.mp3", "Groove Armada", "White Light", 9, "a8486272-421e-4355-8531-0d0c50f80368"),
        ("Guns 'n Roses/AppetiteForDestruction.mp3", "Guns N' Roses", "Appetite For Destruction", 12, "72cb3561-7056-4c7e-a308-4750ee79b495"),
        ("Hall and Oates/TheVeryBestofDarylHallJohnOates.mp3", "Hall and Oates", "The Very Best of Daryl Hall / John Oates", 18, "9dc99dff-98db-4ae0-b9d5-2cc058e64831"),
        ("Hooverphonic/LiveAtTheAncienneBelgique.mp3", "Hooverphonic", "Live at the Ancienne Belgique", 21, "218cc7d0-321a-436b-89f4-85f49bdd5031"),
        ("Howe, Steve/Anthology1.mp3", "Steve Howe", "Anthology", 24, "0b87503d-c9bb-4720-929f-4e7e909b8d2d"),
        ("Humpback Whales/SongsOfTheHumpbackWhale.mp3", "Humpback Whales", "Songs of the Humpback Whale", 5, "e110c6a6-4441-4ed9-9803-7c6144058081"),
        ("Hush Sound, The/LikeVines.mp3", "The Hush Sound", "Like Vines", 12, "85cedbdc-146e-4b29-8ded-a69aa6d42ea7"),
        ("Hyper/WeControl.mp3", "Hyper", "We Control", 10, "04f66abd-f007-3cbb-8801-d05b52d53fe4"),
        ("Imagine Dragons/NightVisions.mp3", "Imagine Dragons", "Night Visions", 16, "c4f161a9-448d-49c3-948f-8ec1eb69872f"),
        ("Jackson, Michael/Thriller.mp3", "Michael Jackson", "Thriller", 9, "4e2bfe06-f483-4f7b-b12a-f74e2d1aa3f2"),
        ("James Gang/Funk49.mp3", "James Gang", "Funk #49", 10, "55be9910-ca58-4e61-9e14-2751812044ff"),
        ("Jethro Tull/Aqualung.mp3", "Jethro Tull", "Aqualung", 14, "e592b4ee-849a-4ba8-947e-6680ab6bb7ee"),
        ("Jethro Tull/Thick_as_a_Brick/(Jethro_Tull)Thick_as_a_Brick-01-Thick_as_a_Brick.mp3", "Jethro Tull", "Thick as a Brick", 1, "b0ce0010-7ea8-4441-ac8b-2f2c94f7a592"),
        ("Jewel/Lullaby.mp3", "Jewel", "Lullaby", 17, "4b2b73dc-aedb-4915-be86-46aee0456e22"),
        ("John, Elton/CaptainFantasticAndTheBrownDirtCowboy.mp3", "Elton John", "Captain Fantastic And The Brown Dirt Cowboy", 26, "8070817c-be59-4d4b-982a-b6bf4dba8e36"),
        ("John, Elton/Caribou.mp3", "Elton John", "Caribou", 14, "22bf0c08-4d7a-4bab-9705-dea9057ecc79"),
        ("John, Elton/GoodbyeYellowBrickRoad.mp3", "Elton John", "Goodbye Yellow Brick Road", 53, "816ab66c-413f-4347-914e-59f4d957559d"),
        ("Johnson, Jack/InBetweenDreams.mp3", "Jack Johnson", "In Between Dreams", 14, "ed015bf0-5184-4bf0-8ada-c8aaf716051f"),
        ("Johnson, Jack/ToTheSea.mp3", "Jack Johnson", "To The Sea", 13, "293fb30c-6ddd-446d-89ee-a46611f8ba85"),
        ("Journey/TrialByFire.mp3", "Journey", "Trial By Fire", 16, "0588dde0-f221-4495-86b0-257485cac990"),
        ("KISS/GreatestKiss.mp3", "KISS", "Greatest Kiss", 20, "c7db22af-6ff7-44ee-a73c-eca6b8e0e02e"),
        ("Kansas/ThePreludeImplicit.mp3", "Kansas", "The Prelude Implicit (Deluxe Edition)", 12, "f2f9fe46-1367-465e-a936-a4ec5b1d829a"),
        ("Katrina and the Waves/KatrinaAndTheWaves.mp3", "Katrina & The Waves", "Katrina & The Waves", 10, "8fecaba6-a4d5-4b00-a770-5d3b8c4541be"),
        ("Kihn, Gregs Band/BestOfBeserkley.mp3", "The Greg Kihn Band", "Greg Kihn Band \"Best Of Beserkley\" '75-'84", 21, "7a0f0e90-6df1-452c-9f18-c818f4b2143b"),
        ("King Crimson/ThePowerToBelieve.mp3", "King Crimson", "The Power To Believe", 11, "a9282fd5-dbf5-33c1-b8d2-e86beedc097e"),
        ("Knack, The/GetTheKnack.mp3", "The Knack", "Get The Knack", 12, "bb901332-240d-4e98-a310-b6e079786a20"),
        ("Knopfler, Marc/Privateering.mp3", "Mark Knopfler", "Privateering", 25, "ad62022f-9dab-4202-8a5a-8bf563e371b2"),
        ("Kraftwerk/TransEuropeExpress.mp3", "Kraftwerk", "Trans Europe Express", 8, "579efd5e-1acb-48f0-a031-b7f43dcdeae3"),
        ("Lady Gaga/Chromatica.mp3", "Lady Gaga", "Chromatica", 16, "8e9428b9-244d-41c3-b622-2a70fd418e4a"),
        ("Led Zeppelin/LedZeppelin.mp3", "Led Zeppelin", "Led Zeppelin (Deluxe Edition)", 17, "7f171293-c1a3-4101-b7cc-74d354762622"),
        ("Led Zeppelin/LedZeppelinII.mp3", "Led Zeppelin", "Led Zeppelin II (Deluxe Edition)", 17, "c4889ad4-55d4-4d23-8f27-5353750585fe"),
        ("Led Zeppelin/LedZeppelinIII.mp3", "Led Zeppelin", "Led Zeppelin III (Deluxe Edition)", 19, "972e6e2c-a116-4d02-8cfa-fd81a140985d"),
        ("Lewis, Huey and the News/GreatestHitsHueyLewisAndTheNews.mp3", "Huey Lewis & The News", "Greatest Hits: Huey Lewis And The News", 21, "c2773ade-48c3-48e2-a3fc-975ae5c61e2b"),
        ("Lorde/Melodrama.mp3", "Lorde", "Melodrama", 11, "0024c5d8-8b54-459f-ab43-afeab54030a8"),
        ("M83/Junk.mp3", "M83", "Junk", 15, "e54c3103-9ef1-423c-abe0-eaa10c9711b2"),
        ("Marley, Bob and the Wailers/CatchAFire.mp3", "Bob Marley & The Wailers", "Catch A Fire", 20, "111b07d0-87db-4baf-aa8f-686224367fe7"),
        ("Marley, Bob and the Wailers/Exodus40.mp3", "Bob Marley & the Wailers", "Exodus 40", 28, "6c31422a-dcc9-49a5-9f04-370be3944af5"),
        ("Massive Attack/Mezzanine.mp3", "Massive Attack", "Mezzanine", 19, "aca552aa-7329-4659-8281-d68de02d6e4f"),
        ("Mayall, John/AHardRoad.mp3", "John Mayall", "A Hard Road", 28, "175832da-e7d6-456c-8d8f-432c46efec5e"),
        ("McDonald, Michael/IfThatsWhatItTakes.mp3", "Michael McDonald", "If That's What It Takes", 10, "2288ef08-d0fb-4205-8f42-521072afa908"),
        ("Mellencamp, John/AmericanFool.mp3", "John Mellencamp", "American Fool", 10, "8e38148f-38bb-46a8-8239-ed89504fc72f"),
        ("Men at Work/BusinessAsUsual.mp3", "Men At Work", "Business As Usual", 10, "4bbecc8f-9880-4713-b831-01ab28a55d25"),
        ("Metallica/Ride the Lightning/08 - The Call of Ktulu.mp3", "Metallica", "Ride the Lightning", 0, ""),
        ("Miller, Steve's Band/FlyLikeAnEagle.mp3", "Steve Miller Band", "Fly Like An Eagle", 12, "0fdb51ab-a213-48d1-82de-4ea6f32c7e17"),
        ("Moby/ExtremeWays.mp3", "Moby", "Extreme Ways (Bourne's Legacy)", 9, "5a0d65d8-2b14-4ee9-b50e-20947019324e"),
        ("Moby/WaitForMe.mp3", "Moby", "Wait For Me", 16, "a4155a38-18fb-4f72-92eb-76baee112ff7"),
        ("Molly Hatchet/GreatestHitsMollyHatchet.mp3", "Molly Hatchet", "Greatest Hits", 15, "0569b674-549a-44eb-a9f8-f035f61b90e4"),
        ("Money, Eddie/TheBestOfEddieMoney.mp3", "Eddie Money", "The Best of Eddie Money", 16, "803b7d87-f356-41d5-8b5f-f658500bf5fe"),
        ("Nine Inch Nails/PrettyHateMachine.mp3", "Nine Inch Nails", "Pretty Hate Machine", 10, "2b8dcce9-3b43-4614-871d-65bca9b5fa79"),
        ("No Doubt/TragicKingdom.mp3", "No Doubt", "Tragic Kingdom", 14, "d0806294-f924-35c5-8293-188e4df5461f"),
        ("Nova, Heather/300DaysAtSea.mp3", "Heather Nova", "300 Days At Sea", 12, "bbdf91a8-deec-4c61-8ff1-0b0386f619cb"),
        ("Nova, Heather/LiveFromTheMilkyWay.mp3", "Heather Nova", "Live From The Milky Way", 6, "5b64e270-0163-4ca3-8002-9a8f30c137f6"),
        ("Nova, Heather/Pearl.mp3", "Heather Nova", "Pearl", 11, "b893e129-28b3-4299-8eeb-980ed4c5ac1a"),
        ("Nova, Heather/South.mp3", "Heather Nova", "South", 13, "16155ae3-82d6-4a57-99b1-7f106959e27d"),
        ("Oasis/WhatsTheStoryMorningGlory.mp3", "Oasis", "(What's The Story) Morning Glory?", 40, "4bd265cd-2d78-4329-8545-912db2f4f152"),
        ("Ocasek, Ric/Nexterday.mp3", "Ric Ocasek", "Nexterday", 11, "c9feea8d-9c2f-4b3d-a041-31dedd7d067c"),
        ("Outlaws/BestOfGreenGrassAndHighTides.mp3", "The Outlaws", "Best Of...Green Grass & High Tides", 16, "547214e2-7466-4cce-ad1c-caf695155ff6"),
        ("Paramore/Paramore.mp3", "Paramore", "Paramore (Deluxe Edition)", 29, "906b7b1c-b990-4d57-9837-1ae6cb459682"),
        ("Pet Shop Boys/Essential.mp3", "Pet Shop Boys", "Essential", 13, "682fbcce-9686-44fd-9b21-307055a80f0e"),
        ("Petty, Tom and the Heartbreakers/FullMoonFever.mp3", "Tom Petty", "Full Moon Fever", 12, "aad16a07-d11b-42d0-a187-cc48d3ccb198"),
        ("Phildel/Ritual.mp3", "Delerium", "Ritual", 1, "44de9934-5adf-4b0b-a11b-f93aab60303d"),
        ("Pink/GreatestHitsSoFar.mp3", "Pink", "Greatest Hits...So Far!!!", 18, "d475b2b2-4af5-40dc-be11-31f5bd8a6c06"),
        ("Police/OutlandosDAmour.mp3", "The Police", "Outlandos D'Amour", 10, "8be3709f-25f8-4314-a39a-6df38798c35b"),
        ("Police/RegattaDeBlanc.mp3", "The Police", "Reggatta De Blanc", 11, "8e272060-3fb9-4596-b964-6f49825c9f09"),
        ("Police/Synchronicity.mp3", "The Police", "Synchronicity", 11, "c12f364e-02fa-4702-b441-43b5a0a0fafe"),
        ("Police/ZenyattaMondatta.mp3", "The Police", "Zenyatta Mondatta", 11, "0c9b3bf0-cd38-4900-93ad-d0ef9ccf0eaa"),
        ("Portugal. the Man/Woodstock.mp3", "Portugal, The Man", "Woodstock", 10, "b8544f08-448e-4b1e-8060-cdc362146b14"),
        ("Purim, Flora/ButterflyDreams.mp3", "Flora Purim", "Butterfly Dreams", 8, "bad2260c-ccac-4646-80c2-b7e7be102503"),
        ("REO Speedwagon/YouCanTuneAPianoButYouCantTunaFish.mp3", "R.E.O. Speedwagon", "You Can Tune a Piano But You Can't Tuna Fish", 9, "e09a65ef-48db-42bc-8b86-b02e8a26bca6"),
        ("Radiohead/Amnesiac.mp3", "Radiohead", "Amnesiac", 11, "fd29f8d9-e1ea-4ca9-a241-ed9998d661fe"),
        ("Radiohead/KidA.mp3", "Radiohead", "Kid A", 11, "94ccfe26-223a-4591-8aaa-e985c146e4f2"),
        ("Radiohead/PabloHoney.mp3", "Radiohead", "Pablo Honey", 12, "e71bf938-ab69-43ed-a06d-bbee7418e328"),
        ("Rafferty, Gerry/CityToCity.mp3", "Gerry Rafferty", "City To City", 10, "0ce2cdf9-763f-47ed-8157-24f717bfb2fc"),
        ("Ram Jam/TheVeryBestOfRamJam.mp3", "Ram Jam", "The Very Best Of Ram Jam", 20, "54e31b51-bf6b-4a1e-a7c9-0c19d3eb8360"),
        ("Red Ryder/BreakingCurfew.mp3", "Red Rider", "Breaking Curfew", 9, "8aad6480-2da6-4817-a98c-c33468dc8536"),
        ("Red Ryder/DontFightIt.mp3", "Red Rider", "Don't Fight It", 10, "eead700e-539c-43db-830f-e34451fe0789"),
        ("Robyn/BodyTalk.mp3", "Robyn", "Body Talk", 15, "afebe204-c664-474e-8bbc-4a4f49a7025c"),
        ("Rock Candy Funk Party/GrooveIsKing.mp3", "Rock Candy Funk Party", "Groove is King", 16, "a6856c63-2d00-4ab5-9b20-34c35d665642"),
        ("Rolling Stones/ExileOnMainStreet.mp3", "The Rolling Stones", "Exile On Main Street", 28, "c35d07f9-2944-4fc4-b4ce-033b3361ef16"),
        ("Rolling Stones/LetItBleed.mp3", "The Rolling Stones", "Let It Bleed", 9, "0808d7b6-bec6-4f30-9c50-c42f78c63b85"),
        ("Ruby Suns, The/SeaLion.mp3", "The Ruby Suns", "Sea Lion", 10, "52e9c142-9ec3-481c-8b3d-5451235184ab"),
        ("Rundgren, Todd/TheVeryBestOfToddRundgren.mp3", "Todd Rundgren", "The Very Best Of Todd Rundgren", 16, "baa868c8-20f1-4c40-9dc4-2c2fde33cc97"),
        ("Runga, Bic/Birds.mp3", "Bic Runga", "Birds", 16, "cde7699d-c380-436e-914d-5832da0f7ba9"),
        ("Rush/ExitStageLeft.mp3", "Rush", "Exit Stage Left", 13, "8a220fba-f0e7-3d09-b802-62183c1c991a"),
        ("Rush/TestForEcho.mp3", "Rush", "Test For Echo", 11, "f7b62ace-d263-47cb-884d-e76e6b988c12"),
        ("Santana/InvitationToIllumination.mp3", "Carlos Santana", "Invitation to Illumination", 16, "22f174da-271b-415c-8be3-41156c42fbfe"),
        ("Santana/Santana IV/04 - Fillmore East.mp3", "Santana", "Santana IV", 0, ""),
        ("Santana/Santana/11 - Soul Sacrifice (live from Woodstock).mp3", "Santana", "Santana", 0, ""),
        ("Score, The/Atlas.mp3", "The Score", "Atlas", 14, "f49f03c6-389e-4286-a640-1e806cf1471a"),
        ("Scorpions/Love At First Sting.mp3", "Scorpions", "Love At First Sting", 9, "835f47f9-c232-49b2-9f1f-8e989a7655a0"),
        ("Seals and Crofts/SummerBreeze.mp3", "Seals and Crofts", "Summer Breeze", 10, "53453085-ae92-3e4d-aed7-852413deff2e"),
        ("Smash Mouth/AstroLounge.mp3", "Smash Mouth", "Astro Lounge", 15, "3bce4b08-fdb7-4b65-beed-504256ddcfd4"),
        ("Smash Mouth/FushYuMang.mp3", "Smash Mouth", "Fush Yu Mang", 16, "478c9ba3-ffe6-48b1-8f80-0b36bc873c0e"),
        ("Sneaker Pimps/BecomingX.mp3", "Sneaker Pimps", "Becoming X", 12, "0e60ad28-3654-464b-befc-96eb7ee0ff88"),
        ("Soft Cell/NonStopEroticCabaret.mp3", "Soft Cell", "Non-Stop Erotic Cabaret", 12, "991d6a55-402b-446a-a3de-2c69b15fc675"),
        ("Southern Pacific/SouthernPacific.mp3", "Southern Pacific", "Southern Pacific", 10, "8761aec6-395a-4b2c-a0f2-caee9967b235"),
        ("Squier, Billy/DontSayNo.mp3", "Billy Squier", "Don't Say No", 10, "364594e9-0833-350e-8e20-ea11aaff380c"),
        ("Steely Dan/Donald Fagen/SunkenCondos.mp3", "Donald Fagen", "Sunken Condos", 9, "f919828d-1ad3-4119-b2f2-1a70a5c388de"),
        ("Steely Dan/Donald Fagen/TheNightfly.mp3", "Donald Fagen", "The Nightfly", 8, "534437ce-1c5c-434f-ad10-fee0f38f8d4b"),
        ("Steely Dan/Gaucho.mp3", "Steely Dan", "Gaucho", 7, "14a6af0e-43e7-4a40-8344-ba4813974d29"),
        ("Steely Dan/TheRoyalScam.mp3", "Steely Dan", "The Royal Scam", 9, "dce549ff-4e52-4370-af56-16147d3d0465"),
        ("Steppenwolf/TheABCDunhillSingles.mp3", "Steppenwolf", "The ABC/Dunhill Singles", 38, "65c3fc43-285d-4181-ac1e-c7522fce76cf"),
        ("Stone Temple Pilots/Purple.mp3", "Stone Temple Pilots", "Purple", 40, "5180aa8e-2983-4e60-8ece-4762c894c221"),
        ("Stray Cats/BuildForSpeed.mp3", "The Stray Cats", "Built For Speed", 12, "718ea0db-d4dc-4265-83ff-75c1b1f366d9"),
        ("Tears For Fears/SongsFromTheBigChair.mp3", "Tears for Fears", "Songs from the Big Chair", 33, "72deb637-0dd6-452b-be0d-d51c94906d5c"),
        ("Tedeschi Trucks Band/LetMeGetBy.mp3", "Tedeschi Trucks Band", "Let Me Get By", 18, "c0ca82f7-cee1-4025-ac82-00af5793a1a1"),
        ("Thin Lizzie/LiveAndDangerous.mp3", "Thin Lizzy", "Live And Dangerous", 17, "afa896a2-a2d8-48c7-92f8-e70beed7e562"),
        ("Thorogood, George and The Destroyers/TheBaddestOfGeorgeThorogoodAndTheDestroyers.mp3", "George Thorogood and the Destroyers", "The Baddest of George Thorogood and the Destroyers", 12, "09299763-df6c-4fa6-b67d-481dbaeeeb23"),
        ("Thorpe, Billy/ChildrenOfTheSunRevisited.mp3", "Billy Thorpe", "Children Of The Sun...Revisited", 9, "9d4b0e99-d694-40bb-af1b-d2789cba5f0f"),
        ("Toto/TotoIV.mp3", "Toto", "Toto IV", 10, "9ed50b02-f1a5-4487-853a-e92a5388fc3e"),
        ("Traffic/JohnBarleycornMustDie.mp3", "Traffic", "John Barlerycorn Must Die", 16, "8fe4ddcc-ad23-4ae8-b75e-4059e30458f0"),
        ("U2/RattleAndHum.mp3", "U2", "Rattle and Hum", 17, "8768afcd-54cf-4fe3-908d-6e170d6b6666"),
        ("Van Halen/1984.mp3", "Van Halen", "1984", 9, "0b44a5d7-bff0-4bbb-8561-d3ec8c98fe5d"),
        ("Van Halen/FairWarning.mp3", "Van Halen", "Fair Warning", 9, "d16beb98-d83a-4b08-8899-4b0c6d159411"),
        ("Various/GuardiansOfTheGalaxy.mp3", "Various", "Guardians of the Galaxy", 12, "94d3e761-411e-4a11-b20c-4121f9355195"),
        ("Various/Moana.mp3", "Disney", "Moana Soundtrack", 59, "49980cd6-0aea-471b-8971-dc3bb0a2e713"),
        ("Various/NativeAmericanFluteLullabies.mp3", "Jessita Reyes", "Native American Flute Lullabies", 18, "f2c9e523-02b9-4e70-84a9-dbc2bdabde68"),
        ("Various/TheGreatestShowman.mp3", "Various", "The Greatest Showman (Soundtrack)", 6, "39ea37a7-689d-458f-9301-d4b9f382957e"),
        ("Various/TombRaider.mp3", "Various", "Tomb Raider", 16, "a2ed250a-7395-3538-8999-1eca99a70031"),
        ("Vega, Susan/TalesFromTheRealmOfTheQueenOfPentacles.mp3", "Suzanne Vega", "Tales from the Realm of the Queen of Pentacles", 10, "8401654f-f024-4cda-8b72-32fc7ba0e4cb"),
        ("Walsh, Joe/ButSerioulyFolks.mp3", "Joe Walsh", "But Seriously, Folks", 8, "a07e3fc0-276f-32cc-9cdb-097763668b35"),
        ("Wham/MakeItBig.mp3", "Wham!", "Make It Big", 8, "196877c6-9047-429c-bcda-cdaa98cb8401"),
        ("Who, The/WhosNext.mp3", "The Who", "Who's Next", 29, "8f0c939a-fd54-43c0-a3ed-7abba0a7a880"),
        ("Wings/BandOnTheRun.mp3", "Paul McCartney and Wings", "Band On The Run", 9, "372e400f-f2c9-4388-8d9e-afce84e4cb5a"),
        ("Yankovic, Weird Al/TheEssentialWeirdAlYankovic.mp3", "\"Weird Al\" Yankovic", "The Essential \"Weird Al\" Yankovic", 38, "494d8b86-e7cf-4914-bb6d-2f1b8ce738fd"),
        ("Z.Z. Top/ZZTopsFirstAlbum.mp3", "ZZ Top", "ZZ Top's First Album", 10, "21ea6d8e-560c-4d46-bd76-c2e9d2e308d8"),
        ("Zevon, Warren/ExcitableBoy.mp3", "Warren Zevon", "Excitable Boy", 13, "d8b95c1c-3ea5-4f5f-ba10-c0903423842f"),
        ("Zombie, Rob/BestOfRobZombie.mp3", "Rob Zombie", "Best Of/20th Century", 12, "9afe84eb-f2ee-3389-b099-a6f1c1cee838"),
    ];

    info!("Testing {} albums from run29f baseline", baseline.len());
    info!("");

    // Create single matcher with caching
    let mb_client = MusicBrainzClient::new()?;
    let config = wkmp_ai::matching::album_matcher::AlbumMatcherConfig::default();
    let matcher = AlbumMatcher::with_pool(config, mb_client, db_pool);

    // Create second client for displaying baseline release details
    let display_client = MusicBrainzClient::new()?;

    // Create AcousticBrainz client for querying musical flavor data
    let ab_client = AcousticBrainzClient::new()?;
    info!("✓ AcousticBrainz client initialized");
    info!("");

    let mut results = Vec::new();
    let mut tested = 0;
    let mut skipped = 0;
    let mut exact_matches = 0;
    let mut track_count_changes = 0;
    let mut mbid_changes = 0;
    let mut failures = 0;

    // AcousticBrainz statistics
    let mut ab_total_recordings = 0;
    let mut ab_available_recordings = 0;
    let mut ab_missing_recordings = 0;
    let mut ab_albums_queried = 0;

    for (idx, (path, artist, album, baseline_tracks, baseline_mbid)) in baseline.iter().enumerate() {
        // Skip single songs (expected_tracks = 0)
        if *baseline_tracks == 0 {
            let file_path = music_lib.join(path);
            info!("⊘ SKIP {}/{}: {} (single song - baseline track count = 0)", idx + 1, baseline.len(), album);
            info!("   File: {}", file_path.display());
            skipped += 1;
            results.push(ComparisonResult {
                path: path.to_string(),
                artist: artist.to_string(),
                album: album.to_string(),
                baseline_tracks: *baseline_tracks,
                baseline_mbid: baseline_mbid.to_string(),
                current_tracks: None,
                current_mbid: None,
                matched: false,
                match_percentage: 0.0,
                tracks_match: false,
                mbid_match: false,
                skipped: true,
                error: Some("Single song, skipped".to_string()),
                acousticbrainz_total_tracks: 0,
                acousticbrainz_available: 0,
                acousticbrainz_missing: 0,
                tracks: Vec::new(),
                release_metadata: None,
            });
            continue;
        }

        let file_path = music_lib.join(path);

        if !file_path.exists() {
            info!("⊘ SKIP {}/{}: {} (file not found)", idx + 1, baseline.len(), album);
            info!("   File: {}", file_path.display());
            skipped += 1;
            results.push(ComparisonResult {
                path: path.to_string(),
                artist: artist.to_string(),
                album: album.to_string(),
                baseline_tracks: *baseline_tracks,
                baseline_mbid: baseline_mbid.to_string(),
                current_tracks: None,
                current_mbid: None,
                matched: false,
                match_percentage: 0.0,
                tracks_match: false,
                mbid_match: false,
                skipped: true,
                error: Some("File not found".to_string()),
                acousticbrainz_total_tracks: 0,
                acousticbrainz_available: 0,
                acousticbrainz_missing: 0,
                tracks: Vec::new(),
                release_metadata: None,
            });
            continue;
        }

        info!("Testing {}/{}: {} - {}", idx + 1, baseline.len(), artist, album);
        info!("   File: {}", file_path.display());

        match matcher.match_album(&file_path, Some(artist), Some(album)).await {
            Ok(result) => {
                tested += 1;

                let tracks_match = result.matched && result.expected_track_count == *baseline_tracks;
                let mbid_match = result.matched &&
                    result.release_mbid.as_ref().map(|m| m == *baseline_mbid).unwrap_or(false);

                if tracks_match && mbid_match {
                    exact_matches += 1;
                    info!("✓ EXACT MATCH");
                    // Display album details even for exact match
                    display_album_details(
                        &display_client,
                        artist,
                        album,
                        baseline_mbid,
                        &result,
                        false,  // mbid_changed = false
                    ).await;
                } else if result.matched {
                    if !tracks_match {
                        track_count_changes += 1;
                        info!("⚠ TRACK COUNT: {} -> {}", baseline_tracks, result.expected_track_count);
                    }
                    if !mbid_match {
                        mbid_changes += 1;
                        info!("⚠ MBID CHANGED");
                        // Display detailed track listing comparison
                        display_album_details(
                            &display_client,
                            artist,
                            album,
                            baseline_mbid,
                            &result,
                            true,  // mbid_changed = true
                        ).await;
                    }
                } else {
                    failures += 1;
                    info!("✗ FAILED");
                }

                // Extract track details
                let track_details = extract_track_details(&result);

                // Fetch release metadata if matched
                let release_meta = if result.matched {
                    if let Some(mbid) = &result.release_mbid {
                        fetch_release_metadata(&display_client, mbid).await
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Query AcousticBrainz for matched albums
                let (ab_total, ab_available, ab_missing) = if result.matched {
                    if let Some(release_mbid) = &result.release_mbid {
                        match query_acousticbrainz_for_release(&ab_client, &display_client, release_mbid).await {
                            Ok((total, available, missing, available_mbids)) => {
                                ab_albums_queried += 1;
                                ab_total_recordings += total;
                                ab_available_recordings += available;
                                ab_missing_recordings += missing;
                                info!("  🎵 AcousticBrainz: {}/{} recordings have musical flavor data", available, total);
                                if !available_mbids.is_empty() {
                                    info!("     Available recordings: {}", available_mbids.join(", "));
                                }
                                (total, available, missing)
                            }
                            Err(e) => {
                                info!("  ⚠ AcousticBrainz query failed: {:?}", e);
                                (0, 0, 0)
                            }
                        }
                    } else {
                        (0, 0, 0)
                    }
                } else {
                    (0, 0, 0)
                };

                results.push(ComparisonResult {
                    path: path.to_string(),
                    artist: artist.to_string(),
                    album: album.to_string(),
                    baseline_tracks: *baseline_tracks,
                    baseline_mbid: baseline_mbid.to_string(),
                    current_tracks: Some(result.expected_track_count),
                    current_mbid: result.release_mbid.clone(),
                    matched: result.matched,
                    match_percentage: result.match_percentage,
                    tracks_match,
                    mbid_match,
                    skipped: false,
                    error: None,
                    acousticbrainz_total_tracks: ab_total,
                    acousticbrainz_available: ab_available,
                    acousticbrainz_missing: ab_missing,
                    tracks: track_details,
                    release_metadata: release_meta,
                });
            }
            Err(e) => {
                tested += 1;
                failures += 1;
                info!("✗ ERROR: {:?}", e);
                results.push(ComparisonResult {
                    path: path.to_string(),
                    artist: artist.to_string(),
                    album: album.to_string(),
                    baseline_tracks: *baseline_tracks,
                    baseline_mbid: baseline_mbid.to_string(),
                    current_tracks: None,
                    current_mbid: None,
                    matched: false,
                    match_percentage: 0.0,
                    tracks_match: false,
                    mbid_match: false,
                    skipped: false,
                    error: Some(format!("{:?}", e)),
                    acousticbrainz_total_tracks: 0,
                    acousticbrainz_available: 0,
                    acousticbrainz_missing: 0,
                    tracks: Vec::new(),
                    release_metadata: None,
                });
            }
        }
    }

    // Write results to JSON file
    let output_path = "run29f_comparison_results.json";
    let json = serde_json::to_string_pretty(&results)?;
    std::fs::write(output_path, json)?;
    info!("");
    info!("✓ Detailed results written to {}", output_path);
    info!("");

    info!("=== Comparison Summary ===");
    info!("Total albums: {}", baseline.len());
    info!("Tested: {}", tested);
    info!("Skipped: {}", skipped);
    info!("Exact matches: {} ({:.1}%)", exact_matches,
        (exact_matches as f64 / tested as f64) * 100.0);
    info!("Track count changes: {} ({:.1}%)", track_count_changes,
        (track_count_changes as f64 / tested as f64) * 100.0);
    info!("MBID changes: {} ({:.1}%)", mbid_changes,
        (mbid_changes as f64 / tested as f64) * 100.0);
    info!("Failures: {} ({:.1}%)", failures,
        (failures as f64 / tested as f64) * 100.0);

    // AcousticBrainz summary statistics
    info!("");
    info!("=== AcousticBrainz Coverage Summary ===");
    info!("Albums queried: {}", ab_albums_queried);
    info!("Total recordings examined: {}", ab_total_recordings);
    info!("Recordings with AcousticBrainz data: {}", ab_available_recordings);
    info!("Recordings missing AcousticBrainz data: {}", ab_missing_recordings);
    if ab_total_recordings > 0 {
        let availability_pct = (ab_available_recordings as f64 / ab_total_recordings as f64) * 100.0;
        info!("AcousticBrainz availability: {:.1}%", availability_pct);
    } else {
        info!("AcousticBrainz availability: N/A (no recordings queried)");
    }

    info!("");
    info!("✓ Run29f full comparison test COMPLETE");
    info!("  See {} for detailed results", output_path);

    Ok(())
}
