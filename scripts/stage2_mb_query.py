#!/usr/bin/env python3
"""
Stage 2: MusicBrainz Query Prototype

Queries MusicBrainz to verify Stage 2 algorithm on actual fallback files.
Respects MB rate limit (1 request/second).
"""

import json
import sys
import os
import time
import urllib.parse
import urllib.request
from collections import defaultdict
from difflib import SequenceMatcher

# Ensure UTF-8 output
sys.stdout.reconfigure(encoding='utf-8')

# MusicBrainz API settings
MB_API_BASE = "https://musicbrainz.org/ws/2"
MB_USER_AGENT = "WKMP-Stage2-Prototype/1.0 (research)"
MB_RATE_LIMIT = 1.1  # seconds between requests

def jaro_winkler_sim(s1, s2):
    """Simple similarity using SequenceMatcher as approximation."""
    if not s1 or not s2:
        return 0.0
    return SequenceMatcher(None, s1.lower().strip(), s2.lower().strip()).ratio()

def query_musicbrainz(artist, title, limit=10):
    """
    Query MusicBrainz for recordings matching artist + title.
    Returns list of recordings with: mbid, title, artist, duration_ms, releases
    """
    # Build query
    query_parts = []
    if artist:
        query_parts.append(f'artist:"{artist}"')
    if title:
        query_parts.append(f'recording:"{title}"')

    query = " AND ".join(query_parts)
    encoded_query = urllib.parse.quote(query)

    url = f"{MB_API_BASE}/recording?query={encoded_query}&limit={limit}&fmt=json"

    req = urllib.request.Request(url)
    req.add_header("User-Agent", MB_USER_AGENT)

    try:
        with urllib.request.urlopen(req, timeout=30) as response:
            data = json.loads(response.read().decode('utf-8'))

            recordings = []
            for rec in data.get('recordings', []):
                # Extract duration (in milliseconds)
                duration_ms = rec.get('length')

                # Extract releases (albums) this recording appears on
                releases = []
                for release in rec.get('releases', []):
                    releases.append({
                        'title': release.get('title', ''),
                        'date': release.get('date', ''),
                        'country': release.get('country', '')
                    })

                # Extract artist
                artist_credit = rec.get('artist-credit', [])
                artist_name = ''
                if artist_credit:
                    artist_name = artist_credit[0].get('name', '')

                recordings.append({
                    'mbid': rec.get('id', ''),
                    'title': rec.get('title', ''),
                    'artist': artist_name,
                    'duration_ms': duration_ms,
                    'duration_secs': duration_ms / 1000.0 if duration_ms else None,
                    'releases': releases,
                    'score': rec.get('score', 0)
                })

            return recordings
    except Exception as e:
        return {'error': str(e)}

def stage2_match(file_info, mb_results, duration_tolerance=3.0, album_sim_threshold=0.70):
    """
    Apply Stage 2 matching algorithm to MB results.

    Returns: (status, matched_mbid, details)
    - status: 'ACCEPT', 'AMBIGUOUS', 'NO_MATCH', 'ERROR'
    """
    if isinstance(mb_results, dict) and 'error' in mb_results:
        return ('ERROR', None, mb_results['error'])

    if not mb_results:
        return ('NO_MATCH', None, 'No recordings found in MusicBrainz')

    file_duration = file_info.get('duration_secs', 0)
    file_album = file_info.get('id3_album', '')

    # Find recordings that match duration AND album
    matches = []

    for rec in mb_results:
        rec_duration = rec.get('duration_secs')

        # Check duration match
        if rec_duration is None:
            continue

        duration_diff = abs(rec_duration - file_duration)
        if duration_diff > duration_tolerance:
            continue

        # Duration matches - now check album
        album_matched = False
        matched_release = None

        for release in rec.get('releases', []):
            release_title = release.get('title', '')
            if jaro_winkler_sim(file_album, release_title) >= album_sim_threshold:
                album_matched = True
                matched_release = release_title
                break

        if album_matched:
            matches.append({
                'mbid': rec['mbid'],
                'title': rec['title'],
                'artist': rec['artist'],
                'duration_secs': rec_duration,
                'duration_diff': duration_diff,
                'matched_album': matched_release
            })

    if len(matches) == 0:
        # Try duration-only match as fallback info
        duration_only = [r for r in mb_results if r.get('duration_secs') and abs(r['duration_secs'] - file_duration) <= duration_tolerance]
        return ('NO_MATCH', None, f'No album+duration match. Duration-only matches: {len(duration_only)}')
    elif len(matches) == 1:
        return ('ACCEPT', matches[0]['mbid'], matches[0])
    else:
        return ('AMBIGUOUS', None, f'{len(matches)} recordings match duration+album')

def classify_file(r, folder_artists):
    """Classify a file into Stage 2 tier."""
    conf = r.get('acoustid_confidence') or 0
    artist_sim = r.get('artist_similarity') or 0
    title_sim = r.get('title_similarity') or 0

    # Stage 1 accept
    if conf >= 0.80 and artist_sim >= 0.70:
        if not (artist_sim >= 0.80 and title_sim < 0.80):
            return 'accept'

    # SAWT
    if conf >= 0.80 and artist_sim >= 0.80 and title_sim < 0.80:
        return 'sawt'

    # Check folder inference
    folder = os.path.dirname(r.get('file_path', ''))
    if folder in folder_artists:
        inferred = folder_artists[folder]
        id3_artist = r.get('id3_artist', '')
        if jaro_winkler_sim(inferred, id3_artist) >= 0.70:
            return 'folder_infer'

    return 'id3_only'

def get_search_artist(r, tier, folder_artists):
    """Get the artist to use for MB search based on tier."""
    if tier == 'sawt':
        return r.get('acoustid_artist', '') or r.get('id3_artist', '')
    elif tier == 'folder_infer':
        folder = os.path.dirname(r.get('file_path', ''))
        return folder_artists.get(folder, r.get('id3_artist', ''))
    else:
        return r.get('id3_artist', '')

def main():
    # Load results
    results_path = r'C:\Users\Mango Cat\Music\single_song_crossval_results.json'
    with open(results_path, 'r', encoding='utf-8') as f:
        results = json.load(f)

    print(f"Loaded {len(results)} files")

    # Build folder artist mapping from accepted files
    folders = defaultdict(list)
    for r in results:
        folder = os.path.dirname(r.get('file_path', ''))
        folders[folder].append(r)

    folder_artists = {}
    for folder, files in folders.items():
        accepted = []
        for f in files:
            conf = f.get('acoustid_confidence') or 0
            artist_sim = f.get('artist_similarity') or 0
            title_sim = f.get('title_similarity') or 0
            if conf >= 0.80 and artist_sim >= 0.70:
                if not (artist_sim >= 0.80 and title_sim < 0.80):
                    accepted.append(f)

        if accepted:
            artists = set(f.get('acoustid_artist', '').lower() for f in accepted if f.get('acoustid_artist'))
            if len(artists) == 1:
                folder_artists[folder] = accepted[0].get('acoustid_artist', '')

    # Classify all files
    tiers = {'accept': [], 'sawt': [], 'folder_infer': [], 'id3_only': []}
    for r in results:
        tier = classify_file(r, folder_artists)
        tiers[tier].append(r)

    print(f"\nClassification:")
    print(f"  Stage 1 Accept: {len(tiers['accept'])}")
    print(f"  SAWT: {len(tiers['sawt'])}")
    print(f"  Folder Infer: {len(tiers['folder_infer'])}")
    print(f"  ID3 Only: {len(tiers['id3_only'])}")

    # Get fallback files (non-accept)
    fallback = tiers['sawt'] + tiers['folder_infer'] + tiers['id3_only']
    print(f"\nTotal fallback files: {len(fallback)}")

    # Process a sample (or all if --all flag)
    sample_size = 100
    if len(sys.argv) > 1 and sys.argv[1] == '--all':
        sample = fallback
        print(f"\nProcessing ALL {len(sample)} fallback files...")
    else:
        # Stratified sample
        sample = []
        for tier_name in ['sawt', 'folder_infer', 'id3_only']:
            tier_files = tiers[tier_name]
            tier_sample_size = min(len(tier_files), int(sample_size * len(tier_files) / len(fallback)) + 1)
            sample.extend(tier_files[:tier_sample_size])
        sample = sample[:sample_size]
        print(f"\nProcessing sample of {len(sample)} files...")

    # Track results
    results_by_tier = {
        'sawt': {'total': 0, 'accept': 0, 'ambiguous': 0, 'no_match': 0, 'error': 0},
        'folder_infer': {'total': 0, 'accept': 0, 'ambiguous': 0, 'no_match': 0, 'error': 0},
        'id3_only': {'total': 0, 'accept': 0, 'ambiguous': 0, 'no_match': 0, 'error': 0}
    }

    detailed_results = []

    # Process each file
    for i, r in enumerate(sample):
        tier = classify_file(r, folder_artists)
        if tier == 'accept':
            continue

        # Get search parameters
        search_artist = get_search_artist(r, tier, folder_artists)
        search_title = r.get('id3_title', '')

        # Skip if missing required data
        if not search_title or not r.get('duration_secs'):
            results_by_tier[tier]['total'] += 1
            results_by_tier[tier]['error'] += 1
            continue

        # Query MusicBrainz
        print(f"\r[{i+1}/{len(sample)}] Querying: {search_artist} - {search_title[:30]}...", end='', flush=True)

        mb_results = query_musicbrainz(search_artist, search_title)

        # Apply Stage 2 matching
        status, mbid, details = stage2_match(r, mb_results)

        # Record result
        results_by_tier[tier]['total'] += 1
        if status == 'ACCEPT':
            results_by_tier[tier]['accept'] += 1
        elif status == 'AMBIGUOUS':
            results_by_tier[tier]['ambiguous'] += 1
        elif status == 'NO_MATCH':
            results_by_tier[tier]['no_match'] += 1
        else:
            results_by_tier[tier]['error'] += 1

        detailed_results.append({
            'file_path': r.get('file_path', ''),
            'tier': tier,
            'search_artist': search_artist,
            'search_title': search_title,
            'file_album': r.get('id3_album', ''),
            'file_duration': r.get('duration_secs', 0),
            'status': status,
            'matched_mbid': mbid,
            'details': str(details)[:200]
        })

        # Rate limiting
        time.sleep(MB_RATE_LIMIT)

    print("\n")

    # Print summary
    print("=" * 70)
    print("STAGE 2 ACTUAL RESULTS")
    print("=" * 70)
    print()

    total_processed = 0
    total_accept = 0

    for tier_name in ['sawt', 'folder_infer', 'id3_only']:
        tier_results = results_by_tier[tier_name]
        total = tier_results['total']
        if total == 0:
            continue

        accept = tier_results['accept']
        ambiguous = tier_results['ambiguous']
        no_match = tier_results['no_match']
        error = tier_results['error']

        total_processed += total
        total_accept += accept

        success_rate = 100 * accept / total if total > 0 else 0

        print(f"{tier_name.upper()}:")
        print(f"  Total:     {total}")
        print(f"  ACCEPT:    {accept} ({success_rate:.1f}%)")
        print(f"  AMBIGUOUS: {ambiguous}")
        print(f"  NO_MATCH:  {no_match}")
        print(f"  ERROR:     {error}")
        print()

    print("-" * 70)
    overall_rate = 100 * total_accept / total_processed if total_processed > 0 else 0
    print(f"OVERALL: {total_accept}/{total_processed} = {overall_rate:.1f}% success rate")
    print()

    # Save detailed results
    output_path = r'c:\Users\Mango Cat\Dev\McRhythm\stage2_results.json'
    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump({
            'summary': results_by_tier,
            'detailed': detailed_results
        }, f, indent=2, ensure_ascii=False)

    print(f"Detailed results saved to: {output_path}")

    # Show sample failures for analysis
    print()
    print("=" * 70)
    print("SAMPLE NO_MATCH CASES (for analysis)")
    print("=" * 70)
    no_matches = [r for r in detailed_results if r['status'] == 'NO_MATCH'][:10]
    for nm in no_matches:
        print(f"  {nm['tier']}: {nm['search_artist']} - {nm['search_title']}")
        print(f"    Album: {nm['file_album']}, Duration: {nm['file_duration']:.1f}s")
        print(f"    Details: {nm['details']}")
        print()

if __name__ == '__main__':
    main()
