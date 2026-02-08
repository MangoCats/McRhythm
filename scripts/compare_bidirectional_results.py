#!/usr/bin/env python3
"""
Compare bidirectional boundary refinement results against baseline

Analyzes changes in match quality and MBIDs to categorize:
- Improvements: Match percentage increased
- Equivalent: Different MBID but similar/same match percentage
- Regressions: Match percentage decreased
"""

import json
import sys
import io

# Fix Windows console encoding for Unicode output
if sys.platform == 'win32':
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

def load_results(filename):
    """Load comparison results JSON"""
    with open(filename, 'r', encoding='utf-8') as f:
        return json.load(f)

def main():
    if len(sys.argv) < 3:
        print("Usage: python compare_bidirectional_results.py <baseline.json> <current.json>")
        sys.exit(1)

    baseline_file = sys.argv[1]
    current_file = sys.argv[2]

    print(f"Loading baseline: {baseline_file}")
    baseline = load_results(baseline_file)

    print(f"Loading current: {current_file}")
    current = load_results(current_file)

    # Build path -> result maps
    baseline_map = {r['path']: r for r in baseline}
    current_map = {r['path']: r for r in current}

    # Track changes
    improvements = []
    regressions = []
    mbid_changes_equivalent = []
    mbid_changes_improved = []
    mbid_changes_regressed = []
    no_change = []

    for path in baseline_map.keys():
        if path not in current_map:
            print(f"Warning: {path} not in current results")
            continue

        baseline_result = baseline_map[path]
        current_result = current_map[path]

        # Skip if either failed to match
        if not baseline_result.get('matched') or not current_result.get('matched'):
            continue

        baseline_pct = baseline_result.get('match_percentage', 0)
        current_pct = current_result.get('match_percentage', 0)
        baseline_mbid = baseline_result.get('current_mbid')
        current_mbid = current_result.get('current_mbid')

        # Calculate percentage point change
        pct_change = current_pct - baseline_pct

        entry = {
            'path': path,
            'artist': baseline_result['artist'],
            'album': baseline_result['album'],
            'baseline_pct': baseline_pct,
            'current_pct': current_pct,
            'pct_change': pct_change,
            'baseline_mbid': baseline_mbid,
            'current_mbid': current_mbid,
            'mbid_changed': baseline_mbid != current_mbid
        }

        # Categorize changes
        SIGNIFICANT_IMPROVEMENT = 5.0  # 5+ percentage points = significant
        SIGNIFICANT_REGRESSION = -5.0

        if pct_change >= SIGNIFICANT_IMPROVEMENT:
            improvements.append(entry)
            if entry['mbid_changed']:
                mbid_changes_improved.append(entry)
        elif pct_change <= SIGNIFICANT_REGRESSION:
            regressions.append(entry)
            if entry['mbid_changed']:
                mbid_changes_regressed.append(entry)
        elif entry['mbid_changed']:
            mbid_changes_equivalent.append(entry)
        else:
            no_change.append(entry)

    # Generate report
    print("\n" + "=" * 80)
    print("BIDIRECTIONAL BOUNDARY REFINEMENT - RESULTS COMPARISON")
    print("=" * 80)
    print()
    print(f"Total albums compared: {len(baseline_map)}")
    print(f"  Improvements (≥+5%): {len(improvements)}")
    print(f"  Regressions (≤-5%):  {len(regressions)}")
    print(f"  MBID changes (equivalent quality): {len(mbid_changes_equivalent)}")
    print(f"  No significant change: {len(no_change)}")
    print()

    # Improvements
    if improvements:
        print("=" * 80)
        print(f"IMPROVEMENTS ({len(improvements)} albums)")
        print("=" * 80)
        improvements.sort(key=lambda x: x['pct_change'], reverse=True)
        for entry in improvements[:20]:  # Show top 20
            print(f"\n{entry['artist']} - {entry['album']}")
            print(f"  Match: {entry['baseline_pct']:.1f}% → {entry['current_pct']:.1f}% (+{entry['pct_change']:.1f}%)")
            if entry['mbid_changed']:
                print(f"  MBID: {entry['baseline_mbid']}")
                print(f"     → {entry['current_mbid']}")
        if len(improvements) > 20:
            print(f"\n... and {len(improvements) - 20} more improvements")

    # Regressions
    if regressions:
        print("\n" + "=" * 80)
        print(f"REGRESSIONS ({len(regressions)} albums)")
        print("=" * 80)
        regressions.sort(key=lambda x: x['pct_change'])
        for entry in regressions:
            print(f"\n{entry['artist']} - {entry['album']}")
            print(f"  Match: {entry['baseline_pct']:.1f}% → {entry['current_pct']:.1f}% ({entry['pct_change']:.1f}%)")
            if entry['mbid_changed']:
                print(f"  MBID: {entry['baseline_mbid']}")
                print(f"     → {entry['current_mbid']}")

    # MBID changes with equivalent quality
    if mbid_changes_equivalent:
        print("\n" + "=" * 80)
        print(f"MBID CHANGES - EQUIVALENT QUALITY ({len(mbid_changes_equivalent)} albums)")
        print("=" * 80)
        for entry in mbid_changes_equivalent[:10]:  # Show first 10
            print(f"\n{entry['artist']} - {entry['album']}")
            print(f"  Match: {entry['baseline_pct']:.1f}% → {entry['current_pct']:.1f}% ({entry['pct_change']:+.1f}%)")
            print(f"  MBID: {entry['baseline_mbid']}")
            print(f"     → {entry['current_mbid']}")
        if len(mbid_changes_equivalent) > 10:
            print(f"\n... and {len(mbid_changes_equivalent) - 10} more equivalent MBID changes")

    # Summary statistics
    print("\n" + "=" * 80)
    print("SUMMARY")
    print("=" * 80)

    total_tested = len([r for r in baseline if r.get('matched')])
    improvement_rate = (len(improvements) / total_tested * 100) if total_tested > 0 else 0
    regression_rate = (len(regressions) / total_tested * 100) if total_tested > 0 else 0

    print(f"\nMatch Quality Changes:")
    print(f"  Improvements: {len(improvements)} ({improvement_rate:.1f}% of tested albums)")
    print(f"  Regressions:  {len(regressions)} ({regression_rate:.1f}% of tested albums)")
    print(f"  Unchanged:    {len(no_change)}")

    # Calculate average change for improved albums
    if improvements:
        avg_improvement = sum(e['pct_change'] for e in improvements) / len(improvements)
        print(f"\nAverage improvement: +{avg_improvement:.1f} percentage points")

    if regressions:
        avg_regression = sum(e['pct_change'] for e in regressions) / len(regressions)
        print(f"Average regression: {avg_regression:.1f} percentage points")

    # Overall assessment
    print("\n" + "=" * 80)
    print("ASSESSMENT")
    print("=" * 80)

    if len(regressions) == 0 and len(improvements) > 0:
        print("\n✓ EXCELLENT: No regressions, with improvements detected")
        print(f"  Bidirectional refinement improved {len(improvements)} albums")
    elif len(regressions) == 0:
        print("\n✓ GOOD: No regressions detected")
        print("  Algorithm maintains quality without degrading any matches")
    elif len(improvements) > len(regressions) * 3:
        print("\n✓ POSITIVE: Improvements significantly outweigh regressions")
        print(f"  Ratio: {len(improvements)} improvements vs {len(regressions)} regressions")
    elif len(improvements) > len(regressions):
        print("\n~ MIXED POSITIVE: More improvements than regressions")
        print(f"  Ratio: {len(improvements)} improvements vs {len(regressions)} regressions")
    else:
        print("\n✗ CONCERNING: Regressions present")
        print(f"  {len(regressions)} albums degraded, review needed")

    # Save detailed results
    output = {
        'improvements': improvements,
        'regressions': regressions,
        'mbid_changes_equivalent': mbid_changes_equivalent,
        'mbid_changes_improved': mbid_changes_improved,
        'mbid_changes_regressed': mbid_changes_regressed
    }

    output_file = "bidirectional_refinement_comparison.json"
    with open(output_file, 'w', encoding='utf-8') as f:
        json.dump(output, f, indent=2, ensure_ascii=False)
    print(f"\nDetailed results saved to: {output_file}")

if __name__ == '__main__':
    main()
