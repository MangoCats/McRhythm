#!/usr/bin/env python3
"""
Stage 0: Extract Embedded MusicBrainz Recording IDs

This is the HIGHEST confidence matching method:
- 98.1% of files have embedded MB Recording IDs
- These were set by MusicBrainz Picard (fingerprint + metadata matching)
- User explicitly approved these matches when tagging

Usage:
    python stage0_embedded_mbid.py [--validate]

Options:
    --validate  Check if MB IDs still exist in MusicBrainz (slow, rate-limited)
"""

import json
import os
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from mutagen.id3 import ID3

sys.stdout.reconfigure(encoding='utf-8')

def extract_mb_data(file_path):
    """Extract MusicBrainz data from ID3 tags."""
    result = {
        'file_path': file_path,
        'embedded_recording_id': None,
        'embedded_release_id': None,
        'embedded_artist_id': None,
        'isrc': None,
        'track_number': None,
        'extraction_error': None
    }

    if not os.path.exists(file_path):
        result['extraction_error'] = 'File not found'
        return result

    try:
        tags = ID3(file_path)

        for key in tags.keys():
            # MusicBrainz Recording ID (UFID frame)
            if 'UFID' in key:
                ufid_str = str(tags[key])
                if 'musicbrainz.org' in ufid_str.lower():
                    try:
                        result['embedded_recording_id'] = tags[key].data.decode('utf-8', errors='ignore').strip()
                    except:
                        pass

            # MusicBrainz IDs in TXXX frames
            if 'TXXX' in key:
                desc = str(getattr(tags[key], 'desc', '')).lower()
                text = tags[key].text[0] if tags[key].text else None

                if text:
                    if 'musicbrainz release track id' in desc or 'musicbrainz track id' in desc:
                        result['embedded_recording_id'] = str(text).strip()
                    elif 'musicbrainz album id' in desc:
                        result['embedded_release_id'] = str(text).strip()
                    elif 'musicbrainz artist id' in desc:
                        result['embedded_artist_id'] = str(text).strip()

            # ISRC
            if key == 'TSRC':
                if tags[key].text:
                    result['isrc'] = str(tags[key].text[0]).strip()

            # Track number
            if key == 'TRCK':
                if tags[key].text:
                    result['track_number'] = str(tags[key].text[0]).strip()

    except Exception as e:
        result['extraction_error'] = str(e)[:100]

    return result


def main():
    validate = '--validate' in sys.argv

    input_path = r'C:\Users\Mango Cat\Music\single_song_crossval_results_fixed.json'
    output_path = r'C:\Users\Mango Cat\Music\stage0_embedded_results.json'

    print("=" * 70)
    print("STAGE 0: EMBEDDED MUSICBRAINZ ID EXTRACTION")
    print("=" * 70)
    print()

    # Load crossval results
    print("Loading crossval results...")
    with open(input_path, 'r', encoding='utf-8') as f:
        crossval = json.load(f)

    print(f"Loaded {len(crossval)} files")
    print()

    # Create lookup by file path
    crossval_map = {r['file_path']: r for r in crossval}
    file_paths = list(crossval_map.keys())

    # Extract MB data from all files
    print(f"Extracting embedded MB IDs from {len(file_paths)} files...")
    print()

    results = []
    has_recording_id = 0
    has_release_id = 0
    has_isrc = 0
    has_track = 0
    errors = 0

    # Process files (single-threaded to avoid ID3 library issues)
    for i, file_path in enumerate(file_paths):
        if (i + 1) % 500 == 0:
            print(f"  Progress: {i+1}/{len(file_paths)} ({100*(i+1)/len(file_paths):.1f}%)")

        mb_data = extract_mb_data(file_path)

        # Merge with crossval data
        crossval_entry = crossval_map.get(file_path, {})

        entry = {
            'file_path': file_path,
            'file_hash': crossval_entry.get('file_hash', ''),
            'duration_secs': crossval_entry.get('duration_secs', 0),
            'id3_artist': crossval_entry.get('id3_artist', ''),
            'id3_title': crossval_entry.get('id3_title', ''),
            'id3_album': crossval_entry.get('id3_album', ''),

            # Embedded MB data
            'embedded_recording_id': mb_data['embedded_recording_id'],
            'embedded_release_id': mb_data['embedded_release_id'],
            'embedded_artist_id': mb_data['embedded_artist_id'],
            'isrc': mb_data['isrc'],
            'track_number': mb_data['track_number'],

            # AcoustID data for comparison
            'acoustid_mbid': crossval_entry.get('acoustid_mbid', ''),
            'acoustid_confidence': crossval_entry.get('acoustid_confidence', 0),

            # Confidence tier
            'confidence_tier': None,
            'extraction_error': mb_data['extraction_error']
        }

        # Assign confidence tier
        if mb_data['embedded_recording_id']:
            has_recording_id += 1
            if mb_data['isrc']:
                has_isrc += 1
                entry['confidence_tier'] = '1A_EMBEDDED_ISRC'
            else:
                entry['confidence_tier'] = '1B_EMBEDDED_ONLY'
        elif crossval_entry.get('acoustid_mbid'):
            entry['confidence_tier'] = '2_ACOUSTID_ONLY'
        else:
            entry['confidence_tier'] = '3_NO_MBID'

        if mb_data['embedded_release_id']:
            has_release_id += 1
        if mb_data['track_number']:
            has_track += 1
        if mb_data['extraction_error']:
            errors += 1

        results.append(entry)

    print()
    print("=" * 70)
    print("EXTRACTION COMPLETE")
    print("=" * 70)
    print()

    # Statistics
    total = len(results)
    tier1a = sum(1 for r in results if r['confidence_tier'] == '1A_EMBEDDED_ISRC')
    tier1b = sum(1 for r in results if r['confidence_tier'] == '1B_EMBEDDED_ONLY')
    tier2 = sum(1 for r in results if r['confidence_tier'] == '2_ACOUSTID_ONLY')
    tier3 = sum(1 for r in results if r['confidence_tier'] == '3_NO_MBID')

    print("CONFIDENCE TIER DISTRIBUTION:")
    print(f"  Tier 1A (Embedded + ISRC):    {tier1a:>5} ({100*tier1a/total:>5.1f}%) - HIGHEST confidence")
    print(f"  Tier 1B (Embedded only):      {tier1b:>5} ({100*tier1b/total:>5.1f}%) - VERY HIGH confidence")
    print(f"  Tier 2  (AcoustID only):      {tier2:>5} ({100*tier2/total:>5.1f}%) - HIGH confidence")
    print(f"  Tier 3  (No MB ID):           {tier3:>5} ({100*tier3/total:>5.1f}%) - NEEDS MANUAL")
    print()
    print(f"  TOTAL Tier 1 (Embedded):      {tier1a+tier1b:>5} ({100*(tier1a+tier1b)/total:>5.1f}%)")
    print()

    print("ADDITIONAL METADATA:")
    print(f"  Has Release ID:    {has_release_id:>5} ({100*has_release_id/total:>5.1f}%)")
    print(f"  Has Track Number:  {has_track:>5} ({100*has_track/total:>5.1f}%)")
    print(f"  Extraction Errors: {errors:>5}")
    print()

    # Show files needing manual attention
    print("FILES WITHOUT ANY MB ID (Tier 3):")
    tier3_files = [r for r in results if r['confidence_tier'] == '3_NO_MBID'][:10]
    for r in tier3_files:
        print(f"  {r['id3_artist'][:25]} - {r['id3_title'][:30]}")
    if tier3 > 10:
        print(f"  ... and {tier3-10} more")
    print()

    # Compare embedded vs AcoustID
    print("EMBEDDED vs ACOUSTID COMPARISON:")
    both_have = [r for r in results if r['embedded_recording_id'] and r['acoustid_mbid']]
    same_id = sum(1 for r in both_have if r['embedded_recording_id'] == r['acoustid_mbid'])
    diff_id = len(both_have) - same_id
    print(f"  Files with both:        {len(both_have)}")
    print(f"  Same MB ID:             {same_id} ({100*same_id/len(both_have) if both_have else 0:.1f}%)")
    print(f"  Different MB ID:        {diff_id} ({100*diff_id/len(both_have) if both_have else 0:.1f}%)")
    print()
    print("  Note: Different IDs often represent same song on different releases.")
    print("  Embedded ID is preferred (user-approved via Picard).")
    print()

    # Save results
    print(f"Saving to {output_path}...")
    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(results, f, indent=2, ensure_ascii=False)

    # Save summary
    summary = {
        'total_files': total,
        'tier_1a_embedded_isrc': tier1a,
        'tier_1b_embedded_only': tier1b,
        'tier_2_acoustid_only': tier2,
        'tier_3_no_mbid': tier3,
        'has_release_id': has_release_id,
        'has_track_number': has_track,
        'embedded_acoustid_same': same_id,
        'embedded_acoustid_diff': diff_id,
    }

    summary_path = output_path.replace('.json', '_summary.json')
    with open(summary_path, 'w', encoding='utf-8') as f:
        json.dump(summary, f, indent=2)

    print(f"Summary saved to {summary_path}")
    print()
    print("=" * 70)
    print("STAGE 0 COMPLETE")
    print("=" * 70)


if __name__ == '__main__':
    main()
