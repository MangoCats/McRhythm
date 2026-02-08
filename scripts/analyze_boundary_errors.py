#!/usr/bin/env python3
"""
Analyze boundary detection errors from cascade refinement test output
to identify systematic patterns and propose additional refinement strategies.
"""

import re
import sys
from collections import defaultdict
from dataclasses import dataclass
from typing import List, Tuple

@dataclass
class Track:
    number: int
    title: str
    expected: float
    detected: float
    error: float
    within_tolerance: bool

@dataclass
class Album:
    artist: str
    title: str
    mbid: str
    rank: int
    match_pct: float
    tracks: List[Track]

def parse_track_line(line: str) -> Track:
    """Parse a track detail line."""
    # Example: "    1 The Long Run                      221.23s      0.00s    218.00s     -3.23s          ✓"
    match = re.match(r'\s+(\d+)\s+(.+?)\s+(\d+\.\d+)s\s+\d+\.\d+s\s+(\d+\.\d+)s\s+([+-]\d+\.\d+)s\s+(✓|✗)', line)
    if not match:
        return None

    num, title, expected, detected, error, status = match.groups()
    return Track(
        number=int(num),
        title=title.strip(),
        expected=float(expected),
        detected=float(detected),
        error=float(error),
        within_tolerance=(status == '✓')
    )

def parse_output(filename: str) -> List[Album]:
    """Parse cascade refinement output and extract album/track data."""
    albums = []
    current_album = None
    current_tracks = []

    with open(filename, 'r', encoding='utf-8') as f:
        for line in f:
            # Match album header
            album_match = re.match(r'--- Rank #(\d+): (.+?) - (.+?) ---', line)
            if album_match:
                # Save previous album
                if current_album and current_tracks:
                    current_album.tracks = current_tracks
                    albums.append(current_album)

                rank, artist, title = album_match.groups()
                current_album = Album(
                    artist=artist,
                    title=title,
                    mbid="",
                    rank=int(rank),
                    match_pct=0.0,
                    tracks=[]
                )
                current_tracks = []
                continue

            # Match MBID
            mbid_match = re.search(r'Release MBID: ([a-f0-9-]+)', line)
            if mbid_match and current_album:
                current_album.mbid = mbid_match.group(1)
                continue

            # Match match percentage
            match_pct_match = re.search(r'Match %: ([\d.]+)%', line)
            if match_pct_match and current_album:
                current_album.match_pct = float(match_pct_match.group(1))
                continue

            # Match track line
            track = parse_track_line(line)
            if track and current_album:
                current_tracks.append(track)

    # Save last album
    if current_album and current_tracks:
        current_album.tracks = current_tracks
        albums.append(current_album)

    return albums

def analyze_error_patterns(album: Album) -> dict:
    """Analyze error patterns for a single album."""
    tracks = album.tracks
    if not tracks:
        return {}

    errors = [t.error for t in tracks]

    # Identify consecutive over/under allocations
    consecutive_over = []
    consecutive_under = []
    current_over = []
    current_under = []

    for i, track in enumerate(tracks):
        if track.error > 30:  # Significant over-allocation
            current_over.append(i)
            if current_under:
                consecutive_under.append(current_under)
                current_under = []
        elif track.error < -30:  # Significant under-allocation
            current_under.append(i)
            if current_over:
                consecutive_over.append(current_over)
                current_over = []
        else:
            if current_over:
                consecutive_over.append(current_over)
                current_over = []
            if current_under:
                consecutive_under.append(current_under)
                current_under = []

    if current_over:
        consecutive_over.append(current_over)
    if current_under:
        consecutive_under.append(current_under)

    # Check for complementary errors (one track over, next track under by similar amount)
    complementary_pairs = []
    for i in range(len(tracks) - 1):
        err1 = tracks[i].error
        err2 = tracks[i+1].error
        # Complementary if opposite signs and similar magnitudes
        if err1 > 30 and err2 < -30:
            if abs(abs(err1) - abs(err2)) < 20:
                complementary_pairs.append((i, i+1, err1, err2))
        elif err1 < -30 and err2 > 30:
            if abs(abs(err1) - abs(err2)) < 20:
                complementary_pairs.append((i, i+1, err1, err2))

    # Calculate total error surplus/deficit
    total_error = sum(errors)

    return {
        'consecutive_over': consecutive_over,
        'consecutive_under': consecutive_under,
        'complementary_pairs': complementary_pairs,
        'total_error': total_error,
        'mean_abs_error': sum(abs(e) for e in errors) / len(errors),
        'tracks_within_tolerance': sum(1 for t in tracks if t.within_tolerance),
        'total_tracks': len(tracks)
    }

def print_album_analysis(album: Album):
    """Print detailed analysis for an album."""
    patterns = analyze_error_patterns(album)

    print(f"\n{'='*80}")
    print(f"Album: {album.artist} - {album.title}")
    print(f"Rank: #{album.rank}, Match: {album.match_pct}%, "
          f"Within Tolerance: {patterns['tracks_within_tolerance']}/{patterns['total_tracks']}")
    print(f"Mean Absolute Error: {patterns['mean_abs_error']:.1f}s")
    print(f"Total Net Error: {patterns['total_error']:+.1f}s")
    print(f"{'='*80}")

    # Print track details
    print(f"\n{'#':<3} {'Track':<40} {'Expected':>10} {'Detected':>10} {'Error':>10} {'Status':<3}")
    print("-" * 80)
    for track in album.tracks:
        status = 'OK' if track.within_tolerance else 'BAD'
        print(f"{track.number:<3} {track.title[:40]:<40} {track.expected:>10.1f}s {track.detected:>10.1f}s "
              f"{track.error:>+10.1f}s {status:<3}")

    # Identify patterns
    print(f"\n--- ERROR PATTERNS ---")

    if patterns['complementary_pairs']:
        print(f"\nComplementary Error Pairs (boundary misplacement):")
        for idx1, idx2, err1, err2 in patterns['complementary_pairs']:
            print(f"  Track {idx1+1} ({album.tracks[idx1].title[:30]}): {err1:+.1f}s")
            print(f"  Track {idx2+1} ({album.tracks[idx2].title[:30]}): {err2:+.1f}s")
            print(f"  -> Boundary between tracks {idx1+1}-{idx2+1} likely misplaced")
            print()

    if patterns['consecutive_over']:
        print(f"\nConsecutive Over-Allocations:")
        for group in patterns['consecutive_over']:
            track_nums = [i+1 for i in group]
            errors = [album.tracks[i].error for i in group]
            print(f"  Tracks {track_nums}: {errors}")

    if patterns['consecutive_under']:
        print(f"\nConsecutive Under-Allocations:")
        for group in patterns['consecutive_under']:
            track_nums = [i+1 for i in group]
            errors = [album.tracks[i].error for i in group]
            print(f"  Tracks {track_nums}: {errors}")

    print()

def suggest_refinement_strategies(albums: List[Album]):
    """Suggest refinement strategies based on observed patterns."""
    print(f"\n{'='*80}")
    print("PROPOSED REFINEMENT STRATEGIES")
    print(f"{'='*80}\n")

    # Analyze all albums to find common patterns
    complementary_count = 0
    cascade_count = 0
    systematic_offset_count = 0

    for album in albums:
        if album.rank != 1:  # Only analyze best match
            continue

        patterns = analyze_error_patterns(album)

        if patterns['complementary_pairs']:
            complementary_count += len(patterns['complementary_pairs'])

        if patterns['consecutive_over'] or patterns['consecutive_under']:
            cascade_count += 1

        if abs(patterns['total_error']) > 30:
            systematic_offset_count += 1

    print(f"Summary across {len([a for a in albums if a.rank == 1])} albums (best matches only):")
    print(f"  - Albums with complementary error pairs: {complementary_count} pairs found")
    print(f"  - Albums with cascade patterns: {cascade_count}")
    print(f"  - Albums with systematic offset: {systematic_offset_count}")
    print()

    print("STRATEGY 1: Complementary Error Correction")
    print("-" * 80)
    print("Pattern: One track over-allocated, next track under-allocated by similar amount")
    print("Cause: Boundary misplaced between two tracks")
    print("Solution:")
    print("  - Detect pairs where track[i].error > +30s AND track[i+1].error < -30s")
    print("  - AND abs(track[i].error) ~= abs(track[i+1].error) (within 20s)")
    print("  - Search for better boundary between tracks i and i+1")
    print("  - Use energy minimum detection in +/-30s window around expected boundary")
    print("  - Accept if BOTH tracks move closer to tolerance")
    print()

    print("STRATEGY 2: Cascade Refinement (Already Implemented)")
    print("-" * 80)
    print("Pattern: Multiple consecutive tracks with large errors (>30s)")
    print("Status: [OK] Implemented with full-album validation")
    print("Effectiveness: Fixed Eagles (+30% improvement)")
    print()

    print("STRATEGY 3: Systematic Offset Correction")
    print("-" * 80)
    print("Pattern: Total album duration differs from MusicBrainz by >30s")
    print("Cause: First boundary misplaced, propagating error throughout album")
    print("Solution:")
    print("  - Calculate cumulative error from start")
    print("  - If cumulative error grows monotonically, suspect first boundary")
    print("  - Search for better first boundary in +/-60s window")
    print("  - Recalculate all subsequent boundaries based on energy minima")
    print()

    print("STRATEGY 4: Fine-Grained Boundary Adjustment")
    print("-" * 80)
    print("Pattern: Tracks with 10-30s errors (just outside tolerance)")
    print("Solution:")
    print("  - For tracks with 10s < |error| < 30s:")
    print("  - Search +/-15s window around detected boundary")
    print("  - Use energy minimum + zero-crossing detection")
    print("  - Only accept if moves within tolerance WITHOUT breaking adjacent tracks")
    print()

def main():
    if len(sys.argv) < 2:
        print("Usage: python analyze_boundary_errors.py <cascade_refinement_output.txt>")
        sys.exit(1)

    filename = sys.argv[1]
    albums = parse_output(filename)

    print(f"Parsed {len(albums)} album entries from {filename}")

    # Group by unique album (artist + title)
    unique_albums = {}
    for album in albums:
        key = (album.artist, album.title)
        if key not in unique_albums or album.rank == 1:
            unique_albums[key] = album

    # Analyze only best matches (rank #1) with <75% match
    problem_albums = [a for a in unique_albums.values() if a.rank == 1 and a.match_pct < 75.0]
    problem_albums.sort(key=lambda a: a.match_pct)

    print(f"\nAnalyzing {len(problem_albums)} albums with <75% match quality (best matches only):\n")

    for album in problem_albums:
        print_album_analysis(album)

    suggest_refinement_strategies(list(unique_albums.values()))

if __name__ == '__main__':
    main()
