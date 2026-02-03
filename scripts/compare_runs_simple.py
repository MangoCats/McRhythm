#!/usr/bin/env python3
"""
Simple comparison between Run 27 and Run 28 using log parsing
"""

import re
import json

def extract_run27_results(filename):
    """Extract album results from Run 27 log"""
    print(f"Extracting Run 27 from {filename}...")

    with open(filename, 'r', encoding='utf-8') as f:
        content = f.read()

    results = {}

    # Find all album processing sections by album index
    for album_idx in range(1, 201):
        pattern_final = rf'\[A{album_idx}\].*?Parallel processing complete\. Best: ([\d.]+)% from edition \d+ \(mean error: ([\d.]+)s\)'
        pattern_failed = rf'\[A{album_idx}\].*?FAILED: (.+?)$'
        pattern_metadata = rf'\[A{album_idx}\].*?Reconciled metadata.*?artist=\'([^\']+)\'.*?album=\'([^\']+)\''

        # Try to find match percentage
        match_final = re.search(pattern_final, content)
        match_failed = re.search(pattern_failed, content, re.MULTILINE)
        match_metadata = re.search(pattern_metadata, content)

        if match_final:
            match_pct = float(match_final.group(1))
            mean_error = float(match_final.group(2))
            artist = ""
            album = ""

            if match_metadata:
                artist = match_metadata.group(1)
                album = match_metadata.group(2)

            key = f"{artist} - {album}" if artist and album else f"Album_{album_idx}"
            results[key] = {
                'album_idx': album_idx,
                'match_pct': match_pct,
                'mean_error': mean_error,
                'status': 'SUCCESS',
            }
        elif match_failed:
            if match_metadata:
                artist = match_metadata.group(1)
                album = match_metadata.group(2)
                key = f"{artist} - {album}"
            else:
                key = f"Album_{album_idx}"

            results[key] = {
                'album_idx': album_idx,
                'match_pct': 0.0,
                'status': 'FAILED',
                'failure_reason': match_failed.group(1).strip(),
            }

    print(f"  Found {len(results)} albums")
    return results

def extract_run28_results(filename):
    """Extract album results from Run 28 JSON"""
    print(f"Extracting Run 28 from {filename}...")

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
            'matching_stage': album.get('matching_stage'),
        }

    print(f"  Found {len(results)} albums")
    return results

def compare_results(run27, run28):
    """Compare results between runs"""

    print("\n" + "=" * 100)
    print("ALBUM-BY-ALBUM COMPARISON: RUN 27 (Monolithic) vs RUN 28 (Modular)")
    print("=" * 100)
    print()

    all_albums = sorted(set(run27.keys()) | set(run28.keys()))

    regressions = []
    improvements = []
    match_pct_diffs = []

    for album in all_albums:
        r27 = run27.get(album, {})
        r28 = run28.get(album, {})

        pct27 = r27.get('match_pct', 0.0)
        pct28 = r28.get('match_pct', 0.0)

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
        return True
    else:
        print(f"FOUND {len(match_pct_diffs)} ALBUMS WITH DIFFERENCES\n")

        # Sort by absolute difference
        match_pct_diffs.sort(key=lambda x: abs(x[3]), reverse=True)

        print("TOP 30 LARGEST CHANGES:")
        print("-" * 100)
        for i, (album, pct27, pct28, diff) in enumerate(match_pct_diffs[:30], 1):
            arrow = "<--" if diff < 0 else "-->"
            print(f"{i:2d}. {album[:60]:60}")
            print(f"     {pct27:6.1f}% {arrow} {pct28:6.1f}%  (delta: {diff:+6.1f}%)")
            if album in run28:
                r28 = run28[album]
                print(f"     Stage: {r28.get('matching_stage', 'N/A'):30}  Confidence: {r28.get('confidence', 'N/A')}")
            print()

    # Regressions
    if regressions:
        print("\n" + "=" * 100)
        print(f"REGRESSIONS (>5% decrease): {len(regressions)} albums")
        print("=" * 100)
        print()

        regressions.sort(key=lambda x: x[3])
        for i, (album, pct27, pct28, diff, r28) in enumerate(regressions, 1):
            print(f"{i}. {album}")
            print(f"   {pct27:.1f}% --> {pct28:.1f}%  ({diff:+.1f}%)")
            print(f"   Stage: {r28.get('matching_stage', 'N/A')}, Confidence: {r28.get('confidence', 'N/A')}")
            if r28.get('mean_error') is not None:
                print(f"   Mean error: {r28['mean_error']:.2f}s")
            print()

    # Improvements
    if improvements:
        print("\n" + "=" * 100)
        print(f"IMPROVEMENTS (>5% increase): {len(improvements)} albums")
        print("=" * 100)
        print()

        improvements.sort(key=lambda x: x[3], reverse=True)
        for i, (album, pct27, pct28, diff, r28) in enumerate(improvements[:15], 1):
            print(f"{i}. {album}")
            print(f"   {pct27:.1f}% --> {pct28:.1f}%  ({diff:+.1f}%)")
            print(f"   Stage: {r28.get('matching_stage', 'N/A')}")
            print()

    # Summary statistics
    print("\n" + "=" * 100)
    print("SUMMARY STATISTICS")
    print("=" * 100)
    print()

    # Match percentage distribution
    pct_perfect_27 = sum(1 for r in run27.values() if r.get('match_pct', 0) == 100.0)
    pct_perfect_28 = sum(1 for r in run28.values() if r.get('match_pct', 0) == 100.0)

    pct_90_27 = sum(1 for r in run27.values() if r.get('match_pct', 0) >= 90.0)
    pct_90_28 = sum(1 for r in run28.values() if r.get('match_pct', 0) >= 90.0)

    pct_85_27 = sum(1 for r in run27.values() if r.get('match_pct', 0) >= 85.0)
    pct_85_28 = sum(1 for r in run28.values() if r.get('match_pct', 0) >= 85.0)

    pct_fail_27 = sum(1 for r in run27.values() if r.get('match_pct', 0) == 0.0)
    pct_fail_28 = sum(1 for r in run28.values() if r.get('match_pct', 0) == 0.0)

    print("Match Percentage Distribution:")
    print(f"                        Run 27    Run 28    Delta")
    print(f"  100% match:           {pct_perfect_27:3d}       {pct_perfect_28:3d}       {pct_perfect_28 - pct_perfect_27:+3d}")
    print(f"  >=90% match:          {pct_90_27:3d}       {pct_90_28:3d}       {pct_90_28 - pct_90_27:+3d}")
    print(f"  >=85% match:          {pct_85_27:3d}       {pct_85_28:3d}       {pct_85_28 - pct_85_27:+3d}")
    print(f"  0% (failed):          {pct_fail_27:3d}       {pct_fail_28:3d}       {pct_fail_28 - pct_fail_27:+3d}")
    print(f"  Total albums:         {len(run27):3d}       {len(run28):3d}")
    print()

    # Average match percentage
    avg_27 = sum(r.get('match_pct', 0) for r in run27.values()) / len(run27) if run27 else 0
    avg_28 = sum(r.get('match_pct', 0) for r in run28.values()) / len(run28) if run28 else 0

    print(f"Average Match Percentage:")
    print(f"  Run 27: {avg_27:.2f}%")
    print(f"  Run 28: {avg_28:.2f}%")
    print(f"  Delta:  {avg_28 - avg_27:+.2f}%")
    print()

    # Confidence distribution (Run 28 only)
    from collections import defaultdict
    conf_counts = defaultdict(int)
    for r in run28.values():
        conf = r.get('confidence')
        if conf in ['Excellent', 'Good', 'Fair', 'Poor']:
            conf_counts[conf] += 1

    print("Run 28 Confidence Distribution:")
    print(f"  Excellent:  {conf_counts['Excellent']:3d}")
    print(f"  Good:       {conf_counts['Good']:3d}")
    print(f"  Fair:       {conf_counts['Fair']:3d}")
    print(f"  Poor:       {conf_counts['Poor']:3d}")
    print()

    return len(match_pct_diffs) == 0

if __name__ == '__main__':
    import sys

    run27 = extract_run27_results('album_matcher_output_run27.txt')
    run28 = extract_run28_results('album_matcher_results.json')

    identical = compare_results(run27, run28)

    if identical:
        print("\nVERIFICATION PASSED: Modular refactoring produces identical results!")
        sys.exit(0)
    else:
        print("\nVERIFICATION FAILED: Found differences between runs")
        sys.exit(1)
