#!/usr/bin/env python3
"""
Comprehensive album-by-album comparison of Run 27 (monolithic) vs Run 28 (modular)

Extracts and compares:
- Match percentage
- Mean error
- Matched artist/album names
- Artist/album similarity scores
- Matching stage
- Confidence level
"""

import re
import sys
from collections import defaultdict

def parse_album_result(lines, album_idx):
    """Parse album result from output lines"""
    result = {
        'album_idx': album_idx,
        'file_path': None,
        'source_artist': None,
        'source_album': None,
        'matched_artist': None,
        'matched_album': None,
        'artist_similarity': None,
        'album_similarity': None,
        'match_percentage': None,
        'mean_error': None,
        'confidence': None,
        'status': None,
        'matching_stage': None,
        'detected_tracks': None,
        'expected_tracks': None,
    }

    # Search through lines for this album
    for i, line in enumerate(lines):
        # Check if this line is for our album
        if f'[A{album_idx}]' not in line:
            continue

        # Extract file path from "Processing: " line
        if 'Processing:' in line:
            match = re.search(r'Processing: (.+)$', line)
            if match:
                result['file_path'] = match.group(1).strip()

        # Extract source artist/album from "Reconciled metadata" line
        if 'Reconciled metadata:' in line:
            match = re.search(r"artist='([^']+)'.*album='([^']+)'", line)
            if match:
                result['source_artist'] = match.group(1)
                result['source_album'] = match.group(2)

        # Extract match result
        if 'SUCCESS:' in line or 'FAILED:' in line:
            if 'SUCCESS:' in line:
                result['status'] = 'SUCCESS'
                # Extract match percentage
                match = re.search(r'(\d+\.\d+)% match', line)
                if match:
                    result['match_percentage'] = float(match.group(1))
                # Extract mean error
                match = re.search(r'mean error: ([\d.]+)s', line)
                if match:
                    result['mean_error'] = float(match.group(1))
                # Extract confidence
                match = re.search(r'\((Excellent|Good|Fair|Poor)\)', line)
                if match:
                    result['confidence'] = match.group(1)
            else:
                result['status'] = 'FAILED'
                result['match_percentage'] = 0.0
                # Extract failure reason
                match = re.search(r'FAILED: (.+)$', line)
                if match:
                    result['confidence'] = match.group(1).strip()

        # Extract matched artist/album and similarities
        if 'Matched artist:' in line:
            match = re.search(r"Matched artist: '([^']+)' \(similarity: ([\d.]+), Levenshtein: ([\d.]+)\)", line)
            if match:
                result['matched_artist'] = match.group(1)
                result['artist_similarity'] = float(match.group(2))

        if 'Matched album:' in line:
            match = re.search(r"Matched album: '([^']+)' \(similarity: ([\d.]+), Levenshtein: ([\d.]+)\)", line)
            if match:
                result['matched_album'] = match.group(1)
                result['album_similarity'] = float(match.group(2))

        # Extract matching stage
        if 'via' in line and 'Stage' in line:
            match = re.search(r'via (Stage \d+|album_extractor_\d+_\w+)', line)
            if match:
                result['matching_stage'] = match.group(1)

        # Extract track counts
        if 'detected tracks' in line or 'expected tracks' in line:
            match = re.search(r'(\d+) detected tracks.*(\d+) expected', line)
            if match:
                result['detected_tracks'] = int(match.group(1))
                result['expected_tracks'] = int(match.group(2))

    return result

def load_run_results(filename, max_albums=200):
    """Load all album results from output file"""
    print(f"Loading {filename}...", file=sys.stderr)

    with open(filename, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    results = {}
    for album_idx in range(1, max_albums + 1):
        result = parse_album_result(lines, album_idx)
        if result['file_path'] or result['status']:  # Valid result found
            results[album_idx] = result

    print(f"  Loaded {len(results)} album results", file=sys.stderr)
    return results

def compare_albums(run27, run28):
    """Compare album results between two runs"""

    print("=" * 100)
    print("ALBUM-BY-ALBUM COMPARISON: RUN 27 (Monolithic) vs RUN 28 (Modular)")
    print("=" * 100)
    print()

    # Collect differences
    differences = []
    match_pct_diffs = []
    mean_error_diffs = []
    status_changes = []
    name_changes = []

    all_indices = sorted(set(run27.keys()) | set(run28.keys()))

    for idx in all_indices:
        r27 = run27.get(idx, {})
        r28 = run28.get(idx, {})

        if not r27 or not r28:
            continue

        has_diff = False
        diff_details = []

        # Compare match percentage
        pct27 = r27.get('match_percentage', 0)
        pct28 = r28.get('match_percentage', 0)
        if pct27 is not None and pct28 is not None:
            pct_diff = pct28 - pct27
            if abs(pct_diff) > 0.1:  # >0.1% difference
                has_diff = True
                diff_details.append(f"Match%: {pct27:.1f}% → {pct28:.1f}% ({pct_diff:+.1f}%)")
                match_pct_diffs.append((idx, pct27, pct28, pct_diff))

        # Compare mean error
        err27 = r27.get('mean_error')
        err28 = r28.get('mean_error')
        if err27 is not None and err28 is not None:
            err_diff = err28 - err27
            if abs(err_diff) > 0.1:  # >0.1s difference
                has_diff = True
                diff_details.append(f"MeanErr: {err27:.2f}s → {err28:.2f}s ({err_diff:+.2f}s)")
                mean_error_diffs.append((idx, err27, err28, err_diff))

        # Compare status
        status27 = r27.get('status')
        status28 = r28.get('status')
        if status27 != status28:
            has_diff = True
            diff_details.append(f"Status: {status27} → {status28}")
            status_changes.append((idx, status27, status28))

        # Compare matched artist
        artist27 = r27.get('matched_artist')
        artist28 = r28.get('matched_artist')
        if artist27 and artist28 and artist27 != artist28:
            has_diff = True
            diff_details.append(f"Artist: '{artist27}' → '{artist28}'")
            name_changes.append((idx, 'artist', artist27, artist28))

        # Compare matched album
        album27 = r27.get('matched_album')
        album28 = r28.get('matched_album')
        if album27 and album28 and album27 != album28:
            has_diff = True
            diff_details.append(f"Album: '{album27}' → '{album28}'")
            name_changes.append((idx, 'album', album27, album28))

        # Compare confidence
        conf27 = r27.get('confidence')
        conf28 = r28.get('confidence')
        if conf27 != conf28 and conf27 and conf28:
            has_diff = True
            diff_details.append(f"Confidence: {conf27} → {conf28}")

        if has_diff:
            differences.append({
                'idx': idx,
                'file': r27.get('file_path') or r28.get('file_path'),
                'source_artist': r27.get('source_artist') or r28.get('source_artist'),
                'source_album': r27.get('source_album') or r28.get('source_album'),
                'details': diff_details,
                'r27': r27,
                'r28': r28,
            })

    # Print differences
    if differences:
        print(f"FOUND {len(differences)} ALBUMS WITH DIFFERENCES:\n")
        for diff in differences:
            print(f"[A{diff['idx']}] {diff['source_artist']} - {diff['source_album']}")
            for detail in diff['details']:
                print(f"  • {detail}")
            print()
    else:
        print("NO DIFFERENCES FOUND - Runs are identical!\n")

    # Summary statistics
    print("=" * 100)
    print("SUMMARY STATISTICS")
    print("=" * 100)
    print()

    # Match percentage changes
    if match_pct_diffs:
        print(f"MATCH PERCENTAGE CHANGES: {len(match_pct_diffs)} albums")
        print()

        # Sort by absolute difference
        match_pct_diffs.sort(key=lambda x: abs(x[3]), reverse=True)

        print("Top 10 largest changes:")
        for idx, pct27, pct28, diff in match_pct_diffs[:10]:
            r28_data = run28[idx]
            print(f"  [A{idx}] {r28_data.get('source_artist')} - {r28_data.get('source_album')}")
            print(f"         {pct27:.1f}% → {pct28:.1f}% ({diff:+.1f}%)")
        print()

        # Regressions (negative changes >5%)
        regressions = [(idx, pct27, pct28, diff) for idx, pct27, pct28, diff in match_pct_diffs if diff < -5]
        if regressions:
            print(f"REGRESSIONS (>5% decrease): {len(regressions)} albums")
            for idx, pct27, pct28, diff in regressions:
                r28_data = run28[idx]
                print(f"  [A{idx}] {r28_data.get('source_artist')} - {r28_data.get('source_album')}")
                print(f"         {pct27:.1f}% → {pct28:.1f}% ({diff:+.1f}%)")
            print()

        # Improvements (positive changes >5%)
        improvements = [(idx, pct27, pct28, diff) for idx, pct27, pct28, diff in match_pct_diffs if diff > 5]
        if improvements:
            print(f"IMPROVEMENTS (>5% increase): {len(improvements)} albums")
            for idx, pct27, pct28, diff in improvements:
                r28_data = run28[idx]
                print(f"  [A{idx}] {r28_data.get('source_artist')} - {r28_data.get('source_album')}")
                print(f"         {pct27:.1f}% → {pct28:.1f}% ({diff:+.1f}%)")
            print()

    # Mean error changes
    if mean_error_diffs:
        print(f"MEAN ERROR CHANGES: {len(mean_error_diffs)} albums")
        print()

        # Large error increases (>1s worse)
        error_increases = [(idx, err27, err28, diff) for idx, err27, err28, diff in mean_error_diffs if diff > 1.0]
        if error_increases:
            print(f"LARGE ERROR INCREASES (>1s worse): {len(error_increases)} albums")
            for idx, err27, err28, diff in error_increases:
                r28_data = run28[idx]
                print(f"  [A{idx}] {r28_data.get('source_artist')} - {r28_data.get('source_album')}")
                print(f"         {err27:.2f}s → {err28:.2f}s ({diff:+.2f}s)")
            print()

    # Status changes
    if status_changes:
        print(f"STATUS CHANGES: {len(status_changes)} albums")
        for idx, status27, status28 in status_changes:
            r28_data = run28[idx]
            print(f"  [A{idx}] {r28_data.get('source_artist')} - {r28_data.get('source_album')}")
            print(f"         {status27} → {status28}")
        print()

    # Name changes
    if name_changes:
        print(f"ARTIST/ALBUM NAME CHANGES: {len(name_changes)} albums")
        for idx, field, old, new in name_changes:
            r28_data = run28[idx]
            print(f"  [A{idx}] {r28_data.get('source_artist')} - {r28_data.get('source_album')}")
            print(f"         Matched {field}: '{old}' → '{new}'")
        print()

    # Overall statistics
    print("=" * 100)
    print("OVERALL STATISTICS")
    print("=" * 100)
    print()

    # Count by confidence level
    conf_counts_27 = defaultdict(int)
    conf_counts_28 = defaultdict(int)

    for idx in all_indices:
        if idx in run27:
            conf = run27[idx].get('confidence')
            if conf in ['Excellent', 'Good', 'Fair', 'Poor']:
                conf_counts_27[conf] += 1
        if idx in run28:
            conf = run28[idx].get('confidence')
            if conf in ['Excellent', 'Good', 'Fair', 'Poor']:
                conf_counts_28[conf] += 1

    print("Confidence Level Distribution:")
    print(f"                 Run 27    Run 28    Delta")
    print(f"  Excellent:     {conf_counts_27['Excellent']:3d}       {conf_counts_28['Excellent']:3d}       {conf_counts_28['Excellent'] - conf_counts_27['Excellent']:+3d}")
    print(f"  Good:          {conf_counts_27['Good']:3d}       {conf_counts_28['Good']:3d}       {conf_counts_28['Good'] - conf_counts_27['Good']:+3d}")
    print(f"  Fair:          {conf_counts_27['Fair']:3d}       {conf_counts_28['Fair']:3d}       {conf_counts_28['Fair'] - conf_counts_27['Fair']:+3d}")
    print(f"  Poor:          {conf_counts_27['Poor']:3d}       {conf_counts_28['Poor']:3d}       {conf_counts_28['Poor'] - conf_counts_27['Poor']:+3d}")
    print()

    # Success/failure counts
    success_27 = sum(1 for r in run27.values() if r.get('status') == 'SUCCESS')
    success_28 = sum(1 for r in run28.values() if r.get('status') == 'SUCCESS')

    print(f"Success Rate:")
    print(f"  Run 27: {success_27}/{len(run27)} ({100*success_27/len(run27):.1f}%)")
    print(f"  Run 28: {success_28}/{len(run28)} ({100*success_28/len(run28):.1f}%)")
    print()

    return differences

if __name__ == '__main__':
    run27 = load_run_results('album_matcher_output_run27.txt', max_albums=200)
    run28 = load_run_results('album_matcher_output_run28.txt', max_albums=200)

    differences = compare_albums(run27, run28)

    if not differences:
        print("\n✓ VERIFICATION PASSED: Modular refactoring produces identical results!")
        sys.exit(0)
    else:
        print(f"\n✗ VERIFICATION FAILED: Found {len(differences)} albums with differences")
        sys.exit(1)
