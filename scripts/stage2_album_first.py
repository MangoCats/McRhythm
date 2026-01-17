#!/usr/bin/env python3
"""
Stage 2: Album-First MusicBrainz Query

Improved algorithm that searches by album first, then matches tracks.
This gives more accurate duration matching since tracks come from the same release.
"""

import json
import sys
import os
import time
import urllib.parse
import urllib.request
from collections import defaultdict
from difflib import SequenceMatcher

sys.stdout.reconfigure(encoding='utf-8')

MB_API_BASE = "https://musicbrainz.org/ws/2"
MB_USER_AGENT = "WKMP-Stage2-AlbumFirst/1.0 (research)"
MB_RATE_LIMIT = 1.1

def jaro_winkler_sim(s1, s2):
    if not s1 or not s2:
        return 0.0
    return SequenceMatcher(None, s1.lower().strip(), s2.lower().strip()).ratio()

def query_mb_releases(artist, album, limit=5):
    """Search for releases by artist + album name."""
    query = f'artist:"{artist}" AND release:"{album}"'
    encoded = urllib.parse.quote(query)
    url = f"{MB_API_BASE}/release?query={encoded}&limit={limit}&fmt=json"

    req = urllib.request.Request(url)
    req.add_header("User-Agent", MB_USER_AGENT)

    try:
        with urllib.request.urlopen(req, timeout=30) as response:
            data = json.loads(response.read().decode('utf-8'))
            return data.get('releases', [])
    except Exception as e:
        return {'error': str(e)}

def get_release_tracks(release_id):
    """Get all tracks from a specific release."""
    url = f"{MB_API_BASE}/release/{release_id}?inc=recordings&fmt=json"

    req = urllib.request.Request(url)
    req.add_header("User-Agent", MB_USER_AGENT)

    try:
        with urllib.request.urlopen(req, timeout=30) as response:
            data = json.loads(response.read().decode('utf-8'))

            tracks = []
            for medium in data.get('media', []):
                for track in medium.get('tracks', []):
                    rec = track.get('recording', {})
                    tracks.append({
                        'position': track.get('position'),
                        'title': rec.get('title', ''),
                        'mbid': rec.get('id', ''),
                        'duration_ms': rec.get('length'),
                        'duration_secs': rec.get('length') / 1000.0 if rec.get('length') else None
                    })
            return tracks
    except Exception as e:
        return {'error': str(e)}

def stage2_album_first(file_info, duration_tolerance=3.0, title_sim_threshold=0.70):
    """
    Album-first Stage 2 matching:
    1. Search MB for the release (artist + album)
    2. Get tracks from that release
    3. Match by title + duration
    """
    artist = file_info.get('search_artist', '')
    album = file_info.get('id3_album', '')
    title = file_info.get('id3_title', '')
    file_duration = file_info.get('duration_secs', 0)

    if not artist or not album or not title:
        return ('MISSING_DATA', None, 'Missing artist, album, or title')

    # Step 1: Search for the release
    releases = query_mb_releases(artist, album)
    time.sleep(MB_RATE_LIMIT)

    if isinstance(releases, dict) and 'error' in releases:
        return ('ERROR', None, releases['error'])

    if not releases:
        return ('NO_RELEASE', None, f'No release found for "{artist}" - "{album}"')

    # Step 2: Try each release until we find a track match
    for release in releases[:3]:  # Try top 3 releases
        release_id = release.get('id')
        release_title = release.get('title', '')

        # Check album title similarity
        if jaro_winkler_sim(album, release_title) < 0.70:
            continue

        tracks = get_release_tracks(release_id)
        time.sleep(MB_RATE_LIMIT)

        if isinstance(tracks, dict) and 'error' in tracks:
            continue

        # Step 3: Find matching track by title + duration
        matches = []
        for track in tracks:
            track_title = track.get('title', '')
            track_duration = track.get('duration_secs')

            # Check title similarity
            title_sim = jaro_winkler_sim(title, track_title)
            if title_sim < title_sim_threshold:
                continue

            # Check duration
            if track_duration is None:
                continue

            duration_diff = abs(track_duration - file_duration)
            if duration_diff > duration_tolerance:
                continue

            matches.append({
                'mbid': track['mbid'],
                'title': track_title,
                'duration_secs': track_duration,
                'duration_diff': duration_diff,
                'title_sim': title_sim,
                'release_id': release_id,
                'release_title': release_title
            })

        if len(matches) == 1:
            return ('ACCEPT', matches[0]['mbid'], matches[0])
        elif len(matches) > 1:
            # Multiple matches - try to pick best by title similarity
            best = max(matches, key=lambda m: m['title_sim'])
            if best['title_sim'] >= 0.90:
                return ('ACCEPT', best['mbid'], best)
            return ('AMBIGUOUS', None, f'{len(matches)} tracks match in release')

    return ('NO_MATCH', None, f'No matching track found in {len(releases)} releases')

def classify_file(r, folder_artists):
    conf = r.get('acoustid_confidence') or 0
    artist_sim = r.get('artist_similarity') or 0
    title_sim = r.get('title_similarity') or 0

    if conf >= 0.80 and artist_sim >= 0.70:
        if not (artist_sim >= 0.80 and title_sim < 0.80):
            return 'accept'

    if conf >= 0.80 and artist_sim >= 0.80 and title_sim < 0.80:
        return 'sawt'

    folder = os.path.dirname(r.get('file_path', ''))
    if folder in folder_artists:
        inferred = folder_artists[folder]
        id3_artist = r.get('id3_artist', '')
        if jaro_winkler_sim(inferred, id3_artist) >= 0.70:
            return 'folder_infer'

    return 'id3_only'

def get_search_artist(r, tier, folder_artists):
    if tier == 'sawt':
        return r.get('acoustid_artist', '') or r.get('id3_artist', '')
    elif tier == 'folder_infer':
        folder = os.path.dirname(r.get('file_path', ''))
        return folder_artists.get(folder, r.get('id3_artist', ''))
    else:
        return r.get('id3_artist', '')

def main():
    # Use fixed results with accurate durations
    results_path = r'C:\Users\Mango Cat\Music\single_song_crossval_results_fixed.json'
    with open(results_path, 'r', encoding='utf-8') as f:
        results = json.load(f)

    # Count how many durations were fixed
    fixed_count = sum(1 for r in results if r.get('duration_fixed'))

    print(f"Loaded {len(results)} files ({fixed_count} with corrected durations)")

    # Build folder artist mapping
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

    # Classify files
    tiers = {'accept': [], 'sawt': [], 'folder_infer': [], 'id3_only': []}
    for r in results:
        tier = classify_file(r, folder_artists)
        tiers[tier].append(r)

    print(f"\nClassification:")
    print(f"  Stage 1 Accept: {len(tiers['accept'])}")
    print(f"  SAWT: {len(tiers['sawt'])}")
    print(f"  Folder Infer: {len(tiers['folder_infer'])}")
    print(f"  ID3 Only: {len(tiers['id3_only'])}")

    fallback = tiers['sawt'] + tiers['folder_infer'] + tiers['id3_only']
    print(f"\nTotal fallback: {len(fallback)}")

    # Process sample
    sample_size = 200
    if len(sys.argv) > 1 and sys.argv[1] == '--all':
        sample = fallback
    else:
        # Stratified sample
        sample = []
        for tier_name in ['sawt', 'folder_infer', 'id3_only']:
            tier_files = tiers[tier_name]
            n = min(len(tier_files), max(5, int(sample_size * len(tier_files) / len(fallback))))
            sample.extend(tier_files[:n])
        sample = sample[:sample_size]

    print(f"\nProcessing {len(sample)} files with ALBUM-FIRST approach...")
    print()

    results_by_tier = {
        'sawt': {'total': 0, 'accept': 0, 'no_release': 0, 'no_match': 0, 'ambiguous': 0, 'error': 0, 'missing': 0},
        'folder_infer': {'total': 0, 'accept': 0, 'no_release': 0, 'no_match': 0, 'ambiguous': 0, 'error': 0, 'missing': 0},
        'id3_only': {'total': 0, 'accept': 0, 'no_release': 0, 'no_match': 0, 'ambiguous': 0, 'error': 0, 'missing': 0}
    }

    detailed = []

    for i, r in enumerate(sample):
        tier = classify_file(r, folder_artists)
        if tier == 'accept':
            continue

        search_artist = get_search_artist(r, tier, folder_artists)

        file_info = {
            'search_artist': search_artist or '',
            'id3_album': r.get('id3_album', ''),
            'id3_title': r.get('id3_title', ''),
            'duration_secs': r.get('duration_secs', 0)
        }

        album = r.get('id3_album') or ''
        title = r.get('id3_title') or ''
        display_artist = search_artist or '[unknown]'
        print(f"\r[{i+1}/{len(sample)}] {display_artist[:20]} - {album[:20]} - {title[:20]}...", end='', flush=True)

        status, mbid, details = stage2_album_first(file_info)

        results_by_tier[tier]['total'] += 1
        if status == 'ACCEPT':
            results_by_tier[tier]['accept'] += 1
        elif status == 'NO_RELEASE':
            results_by_tier[tier]['no_release'] += 1
        elif status == 'NO_MATCH':
            results_by_tier[tier]['no_match'] += 1
        elif status == 'AMBIGUOUS':
            results_by_tier[tier]['ambiguous'] += 1
        elif status == 'MISSING_DATA':
            results_by_tier[tier]['missing'] += 1
        else:
            results_by_tier[tier]['error'] += 1

        detailed.append({
            'file': r.get('file_path', ''),
            'tier': tier,
            'artist': search_artist,
            'album': r.get('id3_album', ''),
            'title': r.get('id3_title', ''),
            'duration': r.get('duration_secs', 0),
            'status': status,
            'mbid': mbid,
            'details': str(details)[:300]
        })

    print("\n")
    print("=" * 70)
    print("STAGE 2 ALBUM-FIRST RESULTS")
    print("=" * 70)
    print()

    total_processed = 0
    total_accept = 0

    for tier_name in ['sawt', 'folder_infer', 'id3_only']:
        tr = results_by_tier[tier_name]
        total = tr['total']
        if total == 0:
            continue

        accept = tr['accept']
        total_processed += total
        total_accept += accept

        rate = 100 * accept / total
        print(f"{tier_name.upper()}: {accept}/{total} = {rate:.1f}%")
        print(f"  ACCEPT:     {tr['accept']}")
        print(f"  NO_RELEASE: {tr['no_release']}")
        print(f"  NO_MATCH:   {tr['no_match']}")
        print(f"  AMBIGUOUS:  {tr['ambiguous']}")
        print(f"  ERROR:      {tr['error']}")
        print(f"  MISSING:    {tr['missing']}")
        print()

    overall = 100 * total_accept / total_processed if total_processed > 0 else 0
    print("-" * 70)
    print(f"OVERALL: {total_accept}/{total_processed} = {overall:.1f}%")

    # Save results
    output_path = 'stage2_album_first_results.json'
    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump({'summary': results_by_tier, 'detailed': detailed}, f, indent=2, ensure_ascii=False)
    print(f"\nSaved to {output_path}")

    # Show failures
    print()
    print("=" * 70)
    print("SAMPLE FAILURES")
    print("=" * 70)
    failures = [d for d in detailed if d['status'] not in ['ACCEPT']][:10]
    for f in failures:
        print(f"[{f['status']}] {f['artist']} - {f['album']}")
        print(f"  Title: {f['title']}, Duration: {f['duration']:.1f}s")
        print(f"  Details: {f['details']}")
        print()

if __name__ == '__main__':
    main()
