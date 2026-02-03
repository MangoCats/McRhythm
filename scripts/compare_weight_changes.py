#!/usr/bin/env python3
"""
Compare weight adjustment results (run29g) vs baseline (run29f)

Categorizes MBID changes as:
- Better: Higher match percentage or better edition selection
- Worse: Lower match percentage or worse edition selection
- Equivalent: Different edition of same album with similar quality
"""

import json
import sys

def load_results(filename):
    """Load comparison results JSON"""
    with open(filename, 'r', encoding='utf-8') as f:
        return json.load(f)

def categorize_change(baseline, current):
    """
    Categorize a MBID change as better/worse/equivalent

    Returns: ("better"|"worse"|"equivalent", reason)
    """
    # Compare match percentages
    baseline_match = baseline.get('match_percentage', 0)
    current_match = current.get('match_percentage', 0)

    # Threshold for significant difference
    SIGNIFICANT_DIFF = 5.0  # 5% points

    diff = current_match - baseline_match

    if abs(diff) < SIGNIFICANT_DIFF:
        # Similar match percentage - check if same album
        baseline_album = baseline.get('album_name', '')
        current_album = current.get('album_name', '')

        # Normalize album names for comparison (remove common suffixes)
        baseline_norm = baseline_album.lower().replace(' (remastered)', '').replace(' (deluxe edition)', '').strip()
        current_norm = current_album.lower().replace(' (remastered)', '').replace(' (deluxe edition)', '').strip()

        if baseline_norm == current_norm:
            return ("equivalent", f"Different edition of same album (match: {baseline_match:.1f}% → {current_match:.1f}%)")
        else:
            # Different album with similar match - need manual review
            return ("equivalent", f"Different album, similar match ({baseline_match:.1f}% → {current_match:.1f}%)")

    elif diff > SIGNIFICANT_DIFF:
        return ("better", f"Higher match percentage: {baseline_match:.1f}% → {current_match:.1f}% (+{diff:.1f}%)")

    else:  # diff < -SIGNIFICANT_DIFF
        return ("worse", f"Lower match percentage: {baseline_match:.1f}% → {current_match:.1f}% ({diff:.1f}%)")

def main():
    if len(sys.argv) < 3:
        print("Usage: python compare_weight_changes.py <baseline.json> <current.json>")
        sys.exit(1)

    baseline_file = sys.argv[1]
    current_file = sys.argv[2]

    print(f"Loading baseline: {baseline_file}")
    baseline = load_results(baseline_file)

    print(f"Loading current: {current_file}")
    current = load_results(current_file)

    # Build path → result maps
    baseline_map = {r['file_path']: r for r in baseline}
    current_map = {r['file_path']: r for r in current}

    # Find MBID changes
    changes = {
        'better': [],
        'worse': [],
        'equivalent': []
    }

    for path in baseline_map.keys():
        if path not in current_map:
            print(f"Warning: {path} not in current results")
            continue

        baseline_result = baseline_map[path]
        current_result = current_map[path]

        baseline_mbid = baseline_result.get('release_mbid')
        current_mbid = current_result.get('release_mbid')

        # Skip if either failed
        if not baseline_result.get('matched') or not current_result.get('matched'):
            continue

        # Check for MBID change
        if baseline_mbid != current_mbid:
            category, reason = categorize_change(baseline_result, current_result)
            changes[category].append({
                'path': path,
                'baseline_album': baseline_result.get('album_name', 'Unknown'),
                'baseline_mbid': baseline_mbid,
                'baseline_match': baseline_result.get('match_percentage', 0),
                'current_album': current_result.get('album_name', 'Unknown'),
                'current_mbid': current_mbid,
                'current_match': current_result.get('match_percentage', 0),
                'reason': reason
            })

    # Print summary
    total_changes = sum(len(v) for v in changes.values())

    print("\n" + "=" * 80)
    print(f"WEIGHT ADJUSTMENT IMPACT ANALYSIS")
    print("=" * 80)
    print(f"\nWeights: Duration 0.30→0.35, Quality 0.45→0.40, Name 0.25 (unchanged)")
    print(f"\nTotal MBID Changes: {total_changes}")
    print(f"  Better:     {len(changes['better']):3d} ({len(changes['better'])/total_changes*100:.1f}%)")
    print(f"  Equivalent: {len(changes['equivalent']):3d} ({len(changes['equivalent'])/total_changes*100:.1f}%)")
    print(f"  Worse:      {len(changes['worse']):3d} ({len(changes['worse'])/total_changes*100:.1f}%)")

    # Print details
    print("\n" + "=" * 80)
    print("BETTER MATCHES")
    print("=" * 80)
    for change in sorted(changes['better'], key=lambda x: x['current_match'] - x['baseline_match'], reverse=True):
        print(f"\n{change['path']}")
        print(f"  Before: {change['baseline_album']} ({change['baseline_match']:.1f}%)")
        print(f"  After:  {change['current_album']} ({change['current_match']:.1f}%)")
        print(f"  Reason: {change['reason']}")

    print("\n" + "=" * 80)
    print("WORSE MATCHES")
    print("=" * 80)
    for change in sorted(changes['worse'], key=lambda x: x['baseline_match'] - x['current_match'], reverse=True):
        print(f"\n{change['path']}")
        print(f"  Before: {change['baseline_album']} ({change['baseline_match']:.1f}%)")
        print(f"  After:  {change['current_album']} ({change['current_match']:.1f}%)")
        print(f"  Reason: {change['reason']}")

    print("\n" + "=" * 80)
    print("EQUIVALENT MATCHES (Different Editions)")
    print("=" * 80)
    count = 0
    for change in changes['equivalent']:
        if count < 20:  # Show first 20
            print(f"\n{change['path']}")
            print(f"  Before: {change['baseline_album']} ({change['baseline_match']:.1f}%)")
            print(f"  After:  {change['current_album']} ({change['current_match']:.1f}%)")
            print(f"  Reason: {change['reason']}")
            count += 1
        else:
            break

    if len(changes['equivalent']) > 20:
        print(f"\n... and {len(changes['equivalent']) - 20} more equivalent changes")

    print("\n" + "=" * 80)
    print("SUMMARY")
    print("=" * 80)
    print(f"Weight adjustment produced {total_changes} MBID changes:")
    print(f"  ✓ Better:     {len(changes['better']):3d} albums ({len(changes['better'])/total_changes*100:.1f}%) - Improved match quality")
    print(f"  = Equivalent: {len(changes['equivalent']):3d} albums ({len(changes['equivalent'])/total_changes*100:.1f}%) - Different editions, similar quality")
    print(f"  ✗ Worse:      {len(changes['worse']):3d} albums ({len(changes['worse'])/total_changes*100:.1f}%) - Degraded match quality")

    # Save detailed results
    output_file = "weight_adjustment_analysis.json"
    with open(output_file, 'w', encoding='utf-8') as f:
        json.dump(changes, f, indent=2, ensure_ascii=False)
    print(f"\nDetailed results saved to: {output_file}")

if __name__ == '__main__':
    main()
