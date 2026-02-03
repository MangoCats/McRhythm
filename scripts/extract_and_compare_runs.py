#!/usr/bin/env python3
"""
Extract album results from Run 27 and Run 28 summaries and compare
"""

import re
import json
from collections import defaultdict

def extract_run27_summary(filename):
    """Extract album results from Run 27 summary section"""
    print(f"Extracting Run 27 from {filename}...", flush=True)

    results = {}
    with open(filename, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    # Find summary section (starts after "Sorted by artist")
    in_summary = False
    for line in lines:
        if 'Sorted by artist' in line:
            in_summary = True
            continue

        if not in_summary:
            continue

        if 'Best 5 Albums:' in line:
            break

        # Parse line: "  Artist - Album: threshold → match%"
        match = re.search(r'^\s+(.+?):\s+(-?\d+)dB,\s+([\d.]+)s\s+→\s+([\d.]+)%', line)
        if match:
            album_name = match.group(1)
            threshold = match.group(2)
            min_dur = match.group(3)
            match_pct = float(match.group(4))
            results[album_name] = {
                'match_pct': match_pct,
                'threshold': threshold,
                'min_dur': min_dur,
            }

    print(f"  Found {len(results)} albums", flush=True)
    return results

def extract_run28_json(filename):
    """Extract album results from Run 28 JSON output"""
    print(f"Extracting Run 28 from {filename}...", flush=True)

    with open(filename, 'r', encoding='utf-8') as f:
        data = json.load(f)

    results = {}
    for album in data:
        artist = album.get('artist', '')
        album_name = album.get('album', '')
        key = f"{artist} - {album_name}"

        results[key] = {
            'match_pct': album.get('match_percentage', 0.0),
            'mean_error': album.get('mean_error'),
            'confidence': album.get('confidence'),
            'status': album.get('status'),
            'matched_artist': album.get('matched_artist'),
            'matched_album': album.get('matched_album'),
            'artist_similarity': album.get('artist_similarity'),
            'album_similarity': album.get('album_similarity'),
            'matching_stage': album.get('matching_stage'),
            'threshold': album.get('best_threshold_db'),
            'min_dur': album.get('best_min_duration_secs'),
        }

    print(f"  Found {len(results)} albums", flush=True)
    return results

def compare_results(run27, run28):
    """Compare results between runs"""

    print("\n" + "=" * 100)
    print("ALBUM-BY-ALBUM COMPARISON: RUN 27 vs RUN 28")
    print("=" * 100)
    print()

    # Find all albums
    all_albums = sorted(set(run27.keys()) | set(run28.keys()))

    differences = []
    match_pct_diffs = []
    regressions = []
    improvements = []

    for album in all_albums:
        r27 = run27.get(album, {})
        r28 = run28.get(album, {})

        pct27 = r27.get('match_pct', 0.0)
        pct28 = r28.get('match_pct', 0.0)

        # Check for differences
        if abs(pct28 - pct27) > 0.1:
            diff = pct28 - pct27
            match_pct_diffs.append((album, pct27, pct28, diff))

            if diff < -5:
                regressions.append((album, pct27, pct28, diff, r28))
            elif diff > 5:
                improvements.append((album, pct27, pct28, diff, r28))

    # Print results
    if not match_pct_diffs:
        print("NO DIFFERENCES FOUND - Runs are identical!\n")
    else:
        print(f"FOUND {len(match_pct_diffs)} ALBUMS WITH DIFFERENCES:\n")

        # Show largest changes first
        match_pct_diffs.sort(key=lambda x: abs(x[3]), reverse=True)

        print("TOP 20 LARGEST CHANGES:")
        print()
        for album, pct27, pct28, diff in match_pct_diffs[:20]:
            symbol = "⬇" if diff < 0 else "⬆"
            print(f"{symbol} {album}")
            print(f"   {pct27:.1f}% → {pct28:.1f}% ({diff:+.1f}%)")
            if album in run28:
                r28 = run28[album]
                print(f"   Stage: {r28.get('matching_stage', 'N/A')}, Confidence: {r28.get('confidence', 'N/A')}")
            print()

    # Regressions
    if regressions:
        print("\n" + "=" * 100)
        print(f"REGRESSIONS (>5% decrease): {len(regressions)} albums")
        print("=" * 100)
        print()

        regressions.sort(key=lambda x: x[3])  # Sort by diff (most negative first)
        for album, pct27, pct28, diff, r28 in regressions:
            print(f"⚠  {album}")
            print(f"   {pct27:.1f}% → {pct28:.1f}% ({diff:+.1f}%)")
            print(f"   Stage: {r28.get('matching_stage', 'N/A')}")
            print(f"   Confidence: {r28.get('confidence', 'N/A')}")
            print(f"   Mean error: {r28.get('mean_error', 'N/A')}")
            print()

    # Improvements
    if improvements:
        print("\n" + "=" * 100)
        print(f"IMPROVEMENTS (>5% increase): {len(improvements)} albums")
        print("=" * 100)
        print()

        improvements.sort(key=lambda x: x[3], reverse=True)
        for album, pct27, pct28, diff, r28 in improvements[:10]:
            print(f"✓  {album}")
            print(f"   {pct27:.1f}% → {pct28:.1f}% ({diff:+.1f}%)")
            print(f"   Stage: {r28.get('matching_stage', 'N/A')}")
            print()

    # Summary statistics
    print("\n" + "=" * 100)
    print("SUMMARY STATISTICS")
    print("=" * 100)
    print()

    # Count by confidence (Run 28 only, since Run 27 doesn't have confidence in summary)
    conf_counts = defaultdict(int)
    for album, r28 in run28.items():
        conf = r28.get('confidence')
        if conf in ['Excellent', 'Good', 'Fair', 'Poor']:
            conf_counts[conf] += 1

    print("Run 28 Confidence Distribution:")
    print(f"  Excellent:  {conf_counts['Excellent']:3d}")
    print(f"  Good:       {conf_counts['Good']:3d}")
    print(f"  Fair:       {conf_counts['Fair']:3d}")
    print(f"  Poor:       {conf_counts['Poor']:3d}")
    print()

    # Match percentage distribution
    pct_perfect_27 = sum(1 for r in run27.values() if r.get('match_pct', 0) == 100.0)
    pct_perfect_28 = sum(1 for r in run28.values() if r.get('match_pct', 0) == 100.0)

    pct_good_27 = sum(1 for r in run27.values() if r.get('match_pct', 0) >= 85.0)
    pct_good_28 = sum(1 for r in run28.values() if r.get('match_pct', 0) >= 85.0)

    print("Match Percentage Distribution:")
    print(f"                        Run 27    Run 28    Delta")
    print(f"  100% match:           {pct_perfect_27:3d}       {pct_perfect_28:3d}       {pct_perfect_28 - pct_perfect_27:+3d}")
    print(f"  ≥85% match:           {pct_good_27:3d}       {pct_good_28:3d}       {pct_good_28 - pct_good_27:+3d}")
    print(f"  Total albums:         {len(run27):3d}       {len(run28):3d}")
    print()

    # Overall statistics
    avg_27 = sum(r.get('match_pct', 0) for r in run27.values()) / len(run27) if run27 else 0
    avg_28 = sum(r.get('match_pct', 0) for r in run28.values()) / len(run28) if run28 else 0

    print(f"Average Match Percentage:")
    print(f"  Run 27: {avg_27:.1f}%")
    print(f"  Run 28: {avg_28:.1f}%")
    print(f"  Delta:  {avg_28 - avg_27:+.1f}%")
    print()

    return len(differences) == 0

if __name__ == '__main__':
    import sys

    run27 = extract_run27_summary('album_matcher_output_run27.txt')
    run28 = extract_run28_json('album_matcher_results.json')

    identical = compare_results(run27, run28)

    if identical:
        print("VERIFICATION PASSED: Modular refactoring produces identical results!")
        sys.exit(0)
    else:
        print("VERIFICATION FAILED: Found differences between runs")
        sys.exit(1)
