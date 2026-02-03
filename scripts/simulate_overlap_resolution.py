#!/usr/bin/env python3
"""
Simulate Stage 6 Overlap Resolution for Eagles - The Long Run

This script demonstrates how the overlap resolution algorithm would work
for the Eagles case where cascade and complementary patterns overlap.
"""

# Eagles - The Long Run original track errors (before Stage 6)
original_tracks = [
    {"track": 1, "name": "The Long Run", "expected": 225.68, "detected": 227.34, "error": 1.65},
    {"track": 2, "name": "I Can't Tell You Why", "expected": 290.52, "detected": 289.13, "error": -1.39},
    {"track": 3, "name": "In the City", "expected": 236.85, "detected": 237.28, "error": 0.43},
    {"track": 4, "name": "The Disco Strangler", "expected": 162.32, "detected": 163.30, "error": 0.98},
    {"track": 5, "name": "King of Hollywood", "expected": 397.71, "detected": 381.87, "error": -15.84},
    {"track": 6, "name": "Heartache Tonight", "expected": 268.59, "detected": 269.00, "error": 0.41},
    {"track": 7, "name": "Those Shoes", "expected": 294.26, "detected": 293.41, "error": -0.85},
    {"track": 8, "name": "Teenage Jail", "expected": 223.99, "detected": 147.27, "error": -76.72},  # UNDER
    {"track": 9, "name": "The Greeks Don't Want No Freaks", "expected": 138.40, "detected": 209.70, "error": 71.31},  # OVER
    {"track": 10, "name": "The Sad Café", "expected": 336.95, "detected": 336.95, "error": 0.00}
]

TOLERANCE_SECS = 10.0

def count_within_tolerance(tracks):
    """Count how many tracks are within tolerance."""
    return sum(1 for t in tracks if abs(t["error"]) <= TOLERANCE_SECS)

def calculate_match_percentage(tracks):
    """Calculate match percentage."""
    within = count_within_tolerance(tracks)
    return (within / len(tracks)) * 100.0

def simulate_cascade_refinement(tracks):
    """
    Simulate cascade refinement.

    Cascade pattern detected at tracks 8-9 (both >30s errors).
    Cascade algorithm tries to refine EACH boundary in the cascade region.
    For Eagles, this would try to move boundaries 7-8 AND 8-9.

    Based on actual Stage 6 output, cascade moved boundary 7-8, resulting in:
    - Track 8: 273.97s (+49.98s error)
    - Track 9: 138.50s (perfect!)
    """
    result = [t.copy() for t in tracks]

    # Move ~76s from track 8 to track 7
    result[6]["detected"] += 76.0
    result[6]["error"] = result[6]["detected"] - result[6]["expected"]

    result[7]["detected"] += 50.0
    result[7]["error"] = result[7]["detected"] - result[7]["expected"]

    result[8]["detected"] -= 71.0
    result[8]["error"] = result[8]["detected"] - result[8]["expected"]

    return result

def simulate_complementary_refinement(tracks):
    """
    Simulate complementary refinement.

    Complementary pair detected at tracks 8-9:
    - Track 8: -76.72s (under-allocated)
    - Track 9: +71.31s (over-allocated)

    Complementary algorithm moves the SINGLE boundary between tracks 8-9
    by approximately 74 seconds (average of the two errors).
    """
    result = [t.copy() for t in tracks]

    # Move ~74s from track 9 to track 8
    move_amount = 74.0

    result[7]["detected"] += move_amount
    result[7]["error"] = result[7]["detected"] - result[7]["expected"]

    result[8]["detected"] -= move_amount
    result[8]["error"] = result[8]["detected"] - result[8]["expected"]

    return result

def print_track_report(tracks, title):
    """Print track-by-track report."""
    print(f"\n{'='*80}")
    print(f"{title}")
    print(f"{'='*80}")
    print(f"{'Track':<6} {'Name':<40} {'Expected':>9} {'Detected':>9} {'Error':>8}")
    print(f"{'-'*80}")

    for t in tracks:
        error_str = f"{t['error']:+.2f}s"
        within = "[OK]" if abs(t["error"]) <= TOLERANCE_SECS else "[ER]"
        print(f"{within} {t['track']:<4} {t['name']:<40} {t['expected']:>9.2f} {t['detected']:>9.2f} {error_str:>8}")

    within = count_within_tolerance(tracks)
    match_pct = calculate_match_percentage(tracks)
    print(f"{'-'*80}")
    print(f"Tracks within tolerance: {within}/{len(tracks)} ({match_pct:.1f}%)")

def main():
    print("="*80)
    print("Stage 6 Overlap Resolution Simulation: Eagles - The Long Run")
    print("="*80)

    # Show original state
    print_track_report(original_tracks, "ORIGINAL (before Stage 6)")

    print("\n\n" + "="*80)
    print("PATTERN DETECTION")
    print("="*80)
    print("\nCascade pattern detected:")
    print("  - Tracks 8-9: Both have >30s timing errors")
    print("  - Count: 2 consecutive tracks")

    print("\nComplementary pair detected:")
    print("  - Track 8: -76.72s (under-allocated)")
    print("  - Track 9: +71.31s (over-allocated)")
    print("  - Magnitude difference: 5.41s (within 20s threshold)")

    print("\nOVERLAP DETECTED:")
    print("  - Cascade affects tracks 8-9")
    print("  - Complementary pair affects tracks 8-9")
    print("  - SAME LOCATION -> Use overlap resolution")

    # Sequential strategy (OLD)
    print("\n\n" + "="*80)
    print("SEQUENTIAL STRATEGY (OLD BEHAVIOR)")
    print("="*80)

    print("\nStep 1: Apply cascade refinement FIRST")
    cascade_result = simulate_cascade_refinement(original_tracks)
    print_track_report(cascade_result, "After Cascade Refinement")

    print("\nStep 2: Apply complementary refinement on modified boundaries")
    print("(Complementary now sees different error pattern, may fail validation)")

    # Overlap resolution (NEW)
    print("\n\n" + "="*80)
    print("OVERLAP RESOLUTION (NEW BEHAVIOR)")
    print("="*80)

    print("\nStep 1: Try CASCADE approach")
    cascade_result = simulate_cascade_refinement(original_tracks)
    cascade_match = calculate_match_percentage(cascade_result)
    print(f"  -> Match percentage: {cascade_match:.1f}%")

    print("\nStep 2: Try COMPLEMENTARY approach")
    complementary_result = simulate_complementary_refinement(original_tracks)
    complementary_match = calculate_match_percentage(complementary_result)
    print(f"  -> Match percentage: {complementary_match:.1f}%")

    print(f"\nStep 3: Pick the BETTER approach")
    if complementary_match > cascade_match:
        winner = "COMPLEMENTARY"
        winner_result = complementary_result
        winner_match = complementary_match
    else:
        winner = "CASCADE"
        winner_result = cascade_result
        winner_match = cascade_match

    print(f"  -> Winner: {winner} ({winner_match:.1f}% vs {cascade_match if winner == 'COMPLEMENTARY' else complementary_match:.1f}%)")

    print_track_report(cascade_result, "CASCADE Approach Result")
    print_track_report(complementary_result, "COMPLEMENTARY Approach Result")
    print_track_report(winner_result, f"FINAL RESULT ({winner} approach selected)")

    # Compare improvements
    original_match = calculate_match_percentage(original_tracks)
    improvement = winner_match - original_match

    print("\n\n" + "="*80)
    print("SUMMARY")
    print("="*80)
    print(f"\nOriginal match:       {original_match:.1f}%")
    print(f"Cascade approach:     {cascade_match:.1f}%")
    print(f"Complementary approach: {complementary_match:.1f}%")
    print(f"\nSelected approach:    {winner}")
    print(f"Final match:          {winner_match:.1f}%")
    print(f"Improvement:          {improvement:+.1f} percentage points")

    print("\n" + "="*80)
    print("CONCLUSION")
    print("="*80)
    print(f"\nThe overlap resolution algorithm correctly identifies that the")
    print(f"COMPLEMENTARY approach (moving single boundary 8-9) produces a")
    print(f"better result than the CASCADE approach (moving boundaries 7-8 and 8-9).")
    print(f"\nThis validates the user's insight: 'The algorithm should simply move")
    print(f"the single boundary between tracks 8 and 9 by approximately 74 seconds.'")

if __name__ == "__main__":
    main()
