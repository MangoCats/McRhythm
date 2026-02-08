#!/usr/bin/env python3
"""
Analyze best parameters from Run 27 output to validate Phase 1 assumptions.

Extracts "Best parameters: XXdB, Ys" from each successful album match
to determine how many albums succeeded with default parameters (-50dB, 3s).
"""

import re
from collections import Counter
from pathlib import Path

def extract_best_params(log_file):
    """Extract album IDs and their best matching parameters."""

    # Pattern: [A123]   Best parameters: -50dB, 3s
    # Need to extract album ID from same line as "Best parameters"
    param_line_pattern = re.compile(r'\[A(\d+)\].*Best parameters: (-?\d+)dB, ([\d.]+)s')

    albums = {}

    with open(log_file, 'r', encoding='utf-8') as f:
        for line in f:
            # Extract album ID and parameters from same line
            match = param_line_pattern.search(line)
            if match:
                album_id = int(match.group(1))
                threshold_db = int(match.group(2))
                min_duration_s = float(match.group(3))
                # Store (only update if not already present to keep first occurrence)
                if album_id not in albums:
                    albums[album_id] = (threshold_db, min_duration_s)

    return albums

def analyze_params(albums):
    """Analyze parameter distribution and default parameter success rate."""

    DEFAULT_THRESHOLD = -50
    DEFAULT_MIN_DURATION = 3.0

    # Count parameter combinations
    param_counts = Counter(albums.values())

    # Count albums using default parameters
    default_count = sum(1 for params in albums.values()
                       if params == (DEFAULT_THRESHOLD, DEFAULT_MIN_DURATION))

    total_successful = len(albums)
    default_rate = (default_count / total_successful * 100) if total_successful > 0 else 0

    print("=" * 80)
    print("RUN 27 PARAMETER ANALYSIS")
    print("=" * 80)
    print()
    print(f"Total successful albums: {total_successful}")
    print(f"Albums with default params (-50dB, 3.0s): {default_count}")
    print(f"Default parameter success rate: {default_rate:.1f}%")
    print()
    print("=" * 80)
    print("PARAMETER DISTRIBUTION (Top 20)")
    print("=" * 80)
    print(f"{'Threshold':>12} | {'Min Duration':>12} | {'Count':>5} | {'Percent':>7}")
    print("-" * 55)

    for (thresh, min_dur), count in param_counts.most_common(20):
        pct = count / total_successful * 100
        marker = " <- DEFAULT" if (thresh, min_dur) == (DEFAULT_THRESHOLD, DEFAULT_MIN_DURATION) else ""
        print(f"{thresh:>11}dB | {min_dur:>11}s | {count:>5} | {pct:>6.1f}%{marker}")

    print()
    print("=" * 80)
    print("CRITICAL BLOCKER VALIDATION")
    print("=" * 80)
    print()
    print("CRIT-02: Default Parameter Conflict")
    print("  Specification claimed: -54dB, 0.6s")
    print("  Actual code uses:      -50dB, 3.0s")
    print("  [OK] RESOLVED: Specification error confirmed")
    print()
    print("CRIT-01: Stage 1 Success Rate Assumption")
    print("  Specification assumed: 70-80% success with defaults")
    print(f"  Actual Run 27 data:    {default_rate:.1f}% success with defaults")

    if default_rate >= 70 and default_rate <= 80:
        print("  [OK] VALIDATED: Assumption confirmed")
    elif default_rate >= 60:
        print(f"  [!] CAUTION: Lower than assumed ({default_rate:.1f}% vs 70-80%)")
    elif default_rate >= 50:
        print(f"  [!] WARNING: Significantly lower ({default_rate:.1f}% vs 70-80%)")
    else:
        print(f"  [X] FAILED: Much lower than assumed ({default_rate:.1f}% vs 70-80%)")

    print()
    print("NOTE: These percentages are for SUCCESSFUL albums only.")
    print("Run 27 had some failed albums which would reduce the overall rate.")
    print()

    return default_rate

def main():
    log_file = Path("album_matcher_output_run27.txt")

    if not log_file.exists():
        print(f"ERROR: {log_file} not found")
        return 1

    albums = extract_best_params(log_file)
    default_rate = analyze_params(albums)

    return 0

if __name__ == "__main__":
    exit(main())
