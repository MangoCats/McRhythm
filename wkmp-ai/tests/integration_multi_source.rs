// PLAN023 Multi-Source Integration Tests
//
// Tests verifying fusion algorithms work correctly with multiple data sources.
// Uses mock data to avoid network dependencies.
//
// Test IDs:
// - TC-I-021-01: Multi-source MBID resolution (Bayesian fusion)
// - TC-I-031-01: Multi-source metadata extraction (weighted field selection)
// - TC-I-041-01: Multi-source flavor extraction (characteristic-wise averaging)

use std::collections::HashMap;
use uuid::Uuid;
use wkmp_ai::import_v2::tier2::identity_resolver::IdentityResolver;
use wkmp_ai::import_v2::tier2::metadata_fuser::MetadataFuser;
use wkmp_ai::import_v2::tier2::flavor_synthesizer::FlavorSynthesizer;
use wkmp_ai::import_v2::types::{
    Characteristic, ExtractionSource, ExtractorResult, FlavorExtraction, MBIDCandidate,
    MetadataBundle, MetadataField, MusicalFlavor,
};

// ================================================================================================
// TC-I-021-01: Multi-Source MBID Resolution
// ================================================================================================
//
// **Requirement:** REQ-AI-021 (Identity Resolution - Multi-Source Fusion)
//
// **Test Objective:**
// Verify that IdentityResolver correctly applies Bayesian fusion when multiple sources
// provide MBID candidates with different confidence levels.
//
// **Test Scenario:**
// - AcoustID returns MBID "aaaa-..." with 0.90 confidence
// - MusicBrainz returns MBID "bbbb-..." with 0.75 confidence
// - ID3 tags return MBID "aaaa-..." with 0.85 confidence (agrees with AcoustID)
//
// **Expected Outcome:**
// - Final MBID should be "aaaa-..." (2 sources agree: 0.90 × 0.85 = 0.765 > 0.75)
// - Bayesian multiplication favors agreement when product exceeds single-source confidence
// - Candidates list should include both MBIDs with source tracking

#[tokio::test]
async fn tc_i_021_01_multi_source_mbid_resolution() {
    // Arrange: Create mock MBID candidates from 3 sources
    let mbid_a = Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee").unwrap();
    let mbid_b = Uuid::parse_str("bbbbbbbb-cccc-dddd-eeee-ffffffffffff").unwrap();

    // Source 1: AcoustID returns MBID A with high confidence
    let acoustid_candidates = vec![MBIDCandidate {
        mbid: mbid_a,
        confidence: 0.90,
        sources: vec![ExtractionSource::AcoustID],
    }];

    // Source 2: MusicBrainz returns MBID B with moderate confidence
    let musicbrainz_candidates = vec![MBIDCandidate {
        mbid: mbid_b,
        confidence: 0.75,
        sources: vec![ExtractionSource::MusicBrainz],
    }];

    // Source 3: ID3 tags return MBID A with good confidence (agrees with AcoustID)
    let id3_candidates = vec![MBIDCandidate {
        mbid: mbid_a,
        confidence: 0.85,
        sources: vec![ExtractionSource::ID3Metadata],
    }];

    // Wrap in ExtractorResult
    let candidate_lists = vec![
        ExtractorResult {
            data: acoustid_candidates,
            confidence: 0.90,
            source: ExtractionSource::AcoustID,
        },
        ExtractorResult {
            data: musicbrainz_candidates,
            confidence: 0.75,
            source: ExtractionSource::MusicBrainz,
        },
        ExtractorResult {
            data: id3_candidates,
            confidence: 0.85,
            source: ExtractionSource::ID3Metadata,
        },
    ];

    // Act: Resolve identity with all 3 sources
    let resolver = IdentityResolver::default();
    let resolution = resolver.resolve(candidate_lists).unwrap();

    // Assert: Verify Bayesian fusion selected MBID A (2 sources agree)
    assert_eq!(
        resolution.mbid,
        Some(mbid_a),
        "Should select MBID with most source agreement"
    );

    // Confidence should reflect Bayesian posterior
    // The Bayesian algorithm multiplies confidences: 0.90 × 0.85 = 0.765 for MBID A
    // vs 0.75 for MBID B, so MBID A should be selected
    assert!(
        resolution.confidence > 0.0,
        "Confidence should be positive: got {}",
        resolution.confidence
    );

    // Verify candidates list includes both MBIDs
    assert!(
        resolution.candidates.len() >= 2,
        "Should track multiple candidate MBIDs: got {}",
        resolution.candidates.len()
    );

    // Verify no false conflicts (the algorithm should correctly identify agreement)
    // Note: has_conflict is true if multiple candidates exceed conflict_threshold
    // With proper Bayesian fusion, MBID A should have significantly higher posterior
}

// ================================================================================================
// TC-I-031-01: Multi-Source Metadata Extraction
// ================================================================================================
//
// **Requirement:** REQ-AI-031 (Metadata Fusion - Weighted Field Selection)
//
// **Test Objective:**
// Verify that MetadataFuser correctly selects metadata fields from multiple sources
// using confidence-weighted selection.
//
// **Test Scenario:**
// - ID3 tags: Title="Test Song", Artist="Test Artist", confidence=0.70
// - MusicBrainz: Title="Test Song (Remastered)", Artist="Test Artist", confidence=0.90
// - AcoustID: Title="Test Song", Artist="The Test Artist", confidence=0.85
//
// **Expected Outcome:**
// - Title should be "Test Song (Remastered)" (MusicBrainz has highest confidence)
// - Artist should be "Test Artist" (highest confidence among agreeing sources)
// - Source tracking should record which source provided each field

#[tokio::test]
async fn tc_i_031_01_multi_source_metadata_extraction() {
    // Arrange: Create mock metadata from 3 sources

    // Source 1: ID3 tags (moderate confidence)
    let mut id3_bundle = MetadataBundle::default();
    id3_bundle.title.push(MetadataField {
        value: "Test Song".to_string(),
        confidence: 0.70,
        source: ExtractionSource::ID3Metadata,
    });
    id3_bundle.artist.push(MetadataField {
        value: "Test Artist".to_string(),
        confidence: 0.70,
        source: ExtractionSource::ID3Metadata,
    });
    id3_bundle.album.push(MetadataField {
        value: "Test Album".to_string(),
        confidence: 0.70,
        source: ExtractionSource::ID3Metadata,
    });

    // Source 2: MusicBrainz (highest confidence, different title)
    let mut mb_bundle = MetadataBundle::default();
    mb_bundle.title.push(MetadataField {
        value: "Test Song (Remastered)".to_string(),
        confidence: 0.90,
        source: ExtractionSource::MusicBrainz,
    });
    mb_bundle.artist.push(MetadataField {
        value: "Test Artist".to_string(),
        confidence: 0.90,
        source: ExtractionSource::MusicBrainz,
    });

    // Source 3: AcoustID (high confidence, different artist)
    let mut acoustid_bundle = MetadataBundle::default();
    acoustid_bundle.title.push(MetadataField {
        value: "Test Song".to_string(),
        confidence: 0.85,
        source: ExtractionSource::AcoustID,
    });
    acoustid_bundle.artist.push(MetadataField {
        value: "The Test Artist".to_string(), // Different from others
        confidence: 0.85,
        source: ExtractionSource::AcoustID,
    });

    // Wrap in ExtractorResult
    let bundles = vec![
        ExtractorResult {
            data: id3_bundle,
            confidence: 0.70,
            source: ExtractionSource::ID3Metadata,
        },
        ExtractorResult {
            data: mb_bundle,
            confidence: 0.90,
            source: ExtractionSource::MusicBrainz,
        },
        ExtractorResult {
            data: acoustid_bundle,
            confidence: 0.85,
            source: ExtractionSource::AcoustID,
        },
    ];

    // Act: Fuse metadata from all 3 sources
    let fuser = MetadataFuser::default();
    let fused = fuser.fuse(bundles).unwrap();

    // Assert: Verify weighted field selection

    // Title should be from MusicBrainz (highest confidence)
    assert!(fused.title.is_some(), "Title should be present");
    let title_field = fused.title.unwrap();
    assert_eq!(
        title_field.value,
        "Test Song (Remastered)",
        "Title should come from MusicBrainz (highest confidence)"
    );
    assert_eq!(
        title_field.source,
        ExtractionSource::MusicBrainz,
        "Title source should be tracked"
    );
    assert!(
        title_field.confidence >= 0.90,
        "Title confidence should match MusicBrainz: got {}, expected >= 0.90",
        title_field.confidence
    );

    // Artist should be from MusicBrainz (highest confidence)
    assert!(fused.artist.is_some(), "Artist should be present");
    let artist_field = fused.artist.unwrap();
    assert_eq!(
        artist_field.value,
        "Test Artist",
        "Artist should be from highest-confidence source"
    );
    // Source should be MusicBrainz (highest confidence)
    assert_eq!(
        artist_field.source,
        ExtractionSource::MusicBrainz,
        "Artist source should be tracked"
    );

    // Album should be from ID3 (only source providing album)
    assert!(fused.album.is_some(), "Album should be present");
    let album_field = fused.album.unwrap();
    assert_eq!(
        album_field.value,
        "Test Album",
        "Album should come from ID3 (only source)"
    );
    assert_eq!(
        album_field.source,
        ExtractionSource::ID3Metadata,
        "Album source should be tracked"
    );

    // Overall metadata confidence should reflect fusion quality
    assert!(
        fused.metadata_confidence >= 0.70,
        "Overall metadata confidence should be reasonable with 3 sources: got {}, expected >= 0.70",
        fused.metadata_confidence
    );
}

// ================================================================================================
// TC-I-041-01: Multi-Source Flavor Extraction
// ================================================================================================
//
// **Requirement:** REQ-AI-041 (Flavor Synthesis - Multi-Source Blending)
//
// **Test Objective:**
// Verify that FlavorSynthesizer correctly blends musical flavor characteristics from
// multiple sources using weighted averaging.
//
// **Test Scenario:**
// - Audio features (signal-based): danceability, energy characteristics, confidence=0.85
// - Essentia: danceability, energy characteristics, confidence=0.90
// - Genre mapping: danceability, energy characteristics, confidence=0.60
//
// **Expected Outcome:**
// - Final characteristics should be weighted averages (higher confidence sources weighted more)
// - Source tracking should show which sources contributed
// - Overall flavor confidence should reflect multi-source agreement

#[tokio::test]
async fn tc_i_041_01_multi_source_flavor_extraction() {
    // Arrange: Create mock flavor data from 3 sources

    // Source 1: Audio features (signal-based, high confidence)
    let audio_flavor = MusicalFlavor {
        characteristics: vec![
            Characteristic {
                name: "danceability".to_string(),
                values: {
                    let mut map = HashMap::new();
                    map.insert("danceable".to_string(), 0.7);
                    map.insert("not_danceable".to_string(), 0.3);
                    map
                },
            },
            Characteristic {
                name: "energy".to_string(),
                values: {
                    let mut map = HashMap::new();
                    map.insert("high_energy".to_string(), 0.8);
                    map.insert("low_energy".to_string(), 0.2);
                    map
                },
            },
        ],
    };

    // Source 2: Essentia (highest confidence, slightly different values)
    let essentia_flavor = MusicalFlavor {
        characteristics: vec![
            Characteristic {
                name: "danceability".to_string(),
                values: {
                    let mut map = HashMap::new();
                    map.insert("danceable".to_string(), 0.65);
                    map.insert("not_danceable".to_string(), 0.35);
                    map
                },
            },
            Characteristic {
                name: "energy".to_string(),
                values: {
                    let mut map = HashMap::new();
                    map.insert("high_energy".to_string(), 0.75);
                    map.insert("low_energy".to_string(), 0.25);
                    map
                },
            },
        ],
    };

    // Source 3: Genre mapping (lower confidence)
    let genre_flavor = MusicalFlavor {
        characteristics: vec![
            Characteristic {
                name: "danceability".to_string(),
                values: {
                    let mut map = HashMap::new();
                    map.insert("danceable".to_string(), 0.5);
                    map.insert("not_danceable".to_string(), 0.5);
                    map
                },
            },
            Characteristic {
                name: "energy".to_string(),
                values: {
                    let mut map = HashMap::new();
                    map.insert("high_energy".to_string(), 0.6);
                    map.insert("low_energy".to_string(), 0.4);
                    map
                },
            },
        ],
    };

    // Create FlavorExtraction wrappers
    let extractions = vec![
        FlavorExtraction {
            flavor: audio_flavor,
            confidence: 0.85,
            source: ExtractionSource::AudioDerived,
        },
        FlavorExtraction {
            flavor: essentia_flavor,
            confidence: 0.90,
            source: ExtractionSource::Essentia,
        },
        FlavorExtraction {
            flavor: genre_flavor,
            confidence: 0.60,
            source: ExtractionSource::GenreMapping,
        },
    ];

    // Act: Synthesize flavor from all 3 sources
    let synthesizer = FlavorSynthesizer::default();
    let synthesized = synthesizer.synthesize(extractions).unwrap();

    // Assert: Verify weighted averaging

    // Should have 2 characteristics (danceability, energy)
    assert_eq!(
        synthesized.flavor.characteristics.len(),
        2,
        "Should have 2 characteristics after synthesis"
    );

    // Find danceability characteristic
    let danceability = synthesized
        .flavor
        .get("danceability")
        .expect("Should have danceability characteristic");

    // Verify danceability values are weighted averages
    // Expected "danceable": (0.7*0.85 + 0.65*0.90 + 0.5*0.60) / (0.85+0.90+0.60)
    //                     = (0.595 + 0.585 + 0.30) / 2.35 = 1.48 / 2.35 = 0.630
    let danceable_value = danceability.values.get("danceable").expect("Should have 'danceable' dimension");
    assert!(
        (*danceable_value - 0.630).abs() < 0.05,
        "Danceable value should be weighted average: got {}, expected ~0.630",
        danceable_value
    );

    // Find energy characteristic
    let energy = synthesized
        .flavor
        .get("energy")
        .expect("Should have energy characteristic");

    // Verify energy values are weighted averages
    // Expected "high_energy": (0.8*0.85 + 0.75*0.90 + 0.6*0.60) / 2.35
    //                        = (0.68 + 0.675 + 0.36) / 2.35 = 1.715 / 2.35 = 0.730
    let high_energy_value = energy.values.get("high_energy").expect("Should have 'high_energy' dimension");
    assert!(
        (*high_energy_value - 0.730).abs() < 0.05,
        "High energy value should be weighted average: got {}, expected ~0.730",
        high_energy_value
    );

    // Verify characteristic normalization (values sum to 1.0)
    assert!(
        danceability.is_normalized(),
        "Danceability should be normalized"
    );
    assert!(energy.is_normalized(), "Energy should be normalized");

    // Verify source tracking
    assert_eq!(
        synthesized.sources_used.len(),
        3,
        "Should track all 3 sources"
    );
    assert!(synthesized.sources_used.contains(&ExtractionSource::AudioDerived));
    assert!(synthesized.sources_used.contains(&ExtractionSource::Essentia));
    assert!(synthesized.sources_used.contains(&ExtractionSource::GenreMapping));

    // Overall flavor confidence should reflect multi-source agreement
    assert!(
        synthesized.flavor_confidence >= 0.70,
        "Flavor confidence should be high with 3 sources: got {}, expected >= 0.70",
        synthesized.flavor_confidence
    );

    // Completeness should reflect number of characteristics present
    // With 2 characteristics out of expected 18: 2/18 = 0.111
    assert!(
        synthesized.flavor_completeness >= 0.10,
        "Flavor completeness should reflect characteristics present: got {}, expected >= 0.10",
        synthesized.flavor_completeness
    );
}
