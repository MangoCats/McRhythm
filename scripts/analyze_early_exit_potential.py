#!/usr/bin/env python3
"""
Analyze early-exit potential if parameters are tested in likelihood order.

Strategy: Try most-common parameters first, exit when 100% match found.
"""

import re
from pathlib import Path
from collections import Counter

def extract_match_quality_and_params(log_file):
    """Extract match percentage and best parameters for each successful album."""

    albums = {}

    # Pattern: [A123]     Matched tracks: 10/10 (100.0%)
    match_pct_pattern = re.compile(r'\[A(\d+)\].*Matched tracks: \d+/\d+ \(([\d.]+)%\)')
    # Pattern: [A123]   Best parameters: -50dB, 3s
    param_pattern = re.compile(r'\[A(\d+)\].*Best parameters: (-?\d+)dB, ([\d.]+)s')

    with open(log_file, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    # Two-pass: collect match percentages and parameters
    match_pcts = {}
    param_vals = {}

    for line in lines:
        match_pct_match = match_pct_pattern.search(line)
        if match_pct_match:
            album_id = int(match_pct_match.group(1))
            match_pct = float(match_pct_match.group(2))
            match_pcts[album_id] = match_pct

        param_match = param_pattern.search(line)
        if param_match:
            album_id = int(param_match.group(1))
            threshold = int(param_match.group(2))
            min_dur = float(param_match.group(3))
            param_vals[album_id] = (threshold, min_dur)

    # Combine data
    for album_id in match_pcts.keys():
        if album_id in param_vals:
            albums[album_id] = {
                'match_pct': match_pcts[album_id],
                'params': param_vals[album_id]
            }

    return albums

def analyze_early_exit_potential(albums):
    """Analyze savings from early-exit when 100% match found."""

    # Count parameter frequency
    param_counts = Counter(data['params'] for data in albums.values())

    # Order parameters by frequency (most common first)
    ordered_params = [p for p, _ in param_counts.most_common()]

    # For each album with 100% match, determine when it would early-exit
    perfect_matches = {aid: data for aid, data in albums.items() if data['match_pct'] == 100.0}
    partial_matches = {aid: data for aid, data in albums.items() if data['match_pct'] < 100.0}

    # Calculate rank of each perfect match's parameters
    early_exit_ranks = []
    for album_id, data in perfect_matches.items():
        params = data['params']
        rank = ordered_params.index(params) + 1  # 1-indexed
        early_exit_ranks.append(rank)

    print("=" * 80)
    print("EARLY-EXIT OPTIMIZATION ANALYSIS")
    print("=" * 80)
    print()
    print(f"Total successful albums: {len(albums)}")
    print(f"Albums with 100% match: {len(perfect_matches)} ({len(perfect_matches)/len(albums)*100:.1f}%)")
    print(f"Albums with partial match: {len(partial_matches)} ({len(partial_matches)/len(albums)*100:.1f}%)")
    print()
    print("=" * 80)
    print("PARAMETER ORDERING BY FREQUENCY")
    print("=" * 80)
    print(f"{'Rank':>4} | {'Threshold':>10} | {'Min Dur':>8} | {'Albums':>7} | {'Cumulative':>11}")
    print("-" * 60)

    cumulative_count = 0
    for rank, (params, count) in enumerate(param_counts.most_common(20), 1):
        thresh, min_dur = params
        cumulative_count += count
        cumulative_pct = cumulative_count / len(albums) * 100
        print(f"{rank:>4} | {thresh:>9}dB | {min_dur:>7}s | {count:>7} | {cumulative_pct:>10.1f}%")

    print()
    print("=" * 80)
    print("EARLY-EXIT POTENTIAL (100% matches only)")
    print("=" * 80)
    print()

    if early_exit_ranks:
        from statistics import mean, median

        print(f"Albums that would early-exit: {len(early_exit_ranks)}")
        print()
        print("Exit rank distribution:")
        print(f"  Mean rank:   {mean(early_exit_ranks):.1f}")
        print(f"  Median rank: {median(early_exit_ranks):.1f}")
        print(f"  Min rank:    {min(early_exit_ranks)}")
        print(f"  Max rank:    {max(early_exit_ranks)}")
        print()

        # Count by rank buckets
        rank_1_5 = sum(1 for r in early_exit_ranks if r <= 5)
        rank_6_10 = sum(1 for r in early_exit_ranks if 6 <= r <= 10)
        rank_11_20 = sum(1 for r in early_exit_ranks if 11 <= r <= 20)
        rank_21_plus = sum(1 for r in early_exit_ranks if r > 20)

        print("Exit rank buckets:")
        print(f"  Rank 1-5:    {rank_1_5:>3} albums ({rank_1_5/len(early_exit_ranks)*100:>5.1f}%) - Test <5 params")
        print(f"  Rank 6-10:   {rank_6_10:>3} albums ({rank_6_10/len(early_exit_ranks)*100:>5.1f}%) - Test 6-10 params")
        print(f"  Rank 11-20:  {rank_11_20:>3} albums ({rank_11_20/len(early_exit_ranks)*100:>5.1f}%) - Test 11-20 params")
        print(f"  Rank 21+:    {rank_21_plus:>3} albums ({rank_21_plus/len(early_exit_ranks)*100:>5.1f}%) - Test 21+ params")

    print()
    print("=" * 80)
    print("PERFORMANCE IMPACT ESTIMATE")
    print("=" * 80)
    print()
    print("Current behavior (Run 27):")
    print("  - Tests all 180 parameter combinations for every album")
    print("  - Mean Stage 2 time: 0.43s")
    print("  - Finds best match across all parameters")
    print()
    print("Proposed optimization:")
    print("  - Test parameters in frequency order (most common first)")
    print("  - Early-exit immediately upon finding 100% match")
    print("  - Continue testing all 180 if no 100% match found")
    print()
    print("Expected savings:")

    if early_exit_ranks:
        # Assume testing scales linearly with number of params tested
        # Current: 180 params = 0.43s
        # Proposed: N params = 0.43s * (N/180)

        savings_per_album = []
        for rank in early_exit_ranks:
            current_time = 0.43
            proposed_time = 0.43 * (rank / 180)
            savings = current_time - proposed_time
            savings_per_album.append(savings)

        total_savings = sum(savings_per_album)
        mean_savings = mean(savings_per_album)
        median_savings = median(savings_per_album)

        print(f"  100% match albums ({len(perfect_matches)}):")
        print(f"    Mean savings:   {mean_savings:.3f}s per album")
        print(f"    Median savings: {median_savings:.3f}s per album")
        print(f"    Total savings:  {total_savings:.1f}s across {len(perfect_matches)} albums")
        print()
        print(f"  Partial match albums ({len(partial_matches)}):")
        print(f"    Savings: 0s (still tests all 180 params)")
        print()
        print(f"  Overall impact (all {len(albums)} albums):")
        overall_mean = total_savings / len(albums)
        print(f"    Mean savings: {overall_mean:.3f}s per album")
        print()
        print(f"  Run 27 total time: ~200.5s per album average")
        print(f"  Proposed speedup: {overall_mean:.3f}s ({overall_mean/200.5*100:.2f}% faster)")

    print()
    print("=" * 80)
    print("RECOMMENDATION")
    print("=" * 80)
    print()
    print("IMPLEMENT: Parameter reordering with early-exit")
    print()
    print("Advantages:")
    print("  1. Simple implementation (reorder array + add early-exit condition)")
    print("  2. No additional stages or complexity")
    print("  3. Guaranteed speedup for 100% matches (no slowdown for others)")
    print("  4. Adaptive to dataset (reorder based on empirical success rates)")
    print()
    print("Implementation:")
    print("  1. Reorder STAGE2_THRESHOLD_VALUES and STAGE2_MIN_DURATION_VALUES")
    print("     by empirical frequency from Run 27 dataset")
    print("  2. Add early-exit logic: if (match_percentage == 100.0) break;")
    print("  3. Document ordering rationale in code comments")
    print()

def main():
    log_file = Path("album_matcher_output_run27.txt")

    if not log_file.exists():
        print(f"ERROR: {log_file} not found")
        return 1

    albums = extract_match_quality_and_params(log_file)
    analyze_early_exit_potential(albums)

    return 0

if __name__ == "__main__":
    exit(main())
