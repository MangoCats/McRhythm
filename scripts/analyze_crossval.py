#!/usr/bin/env python3
"""Analyze cross-validation results and categorize disagreements."""

import json
import sys
from collections import defaultdict
from pathlib import Path

def analyze_results(results_path: str):
    """Analyze cross-validation results."""
    with open(results_path, 'r', encoding='utf-8') as f:
        results = json.load(f)

    # Basic stats
    total = len(results)
    errors = sum(1 for r in results if r.get('error'))
    acoustid_found = sum(1 for r in results if r.get('acoustid_mbid') and not r.get('error'))
    no_acoustid = sum(1 for r in results if not r.get('acoustid_mbid') and not r.get('error'))

    # Agreement stats
    agree = sum(1 for r in results if r.get('sources_agree') and r.get('acoustid_mbid'))
    disagree = [r for r in results if not r.get('sources_agree') and r.get('acoustid_mbid') and not r.get('error')]

    # Verification stats
    verified = sum(1 for r in results if r.get('verified_mbid'))

    # Categorize disagreements
    double_mismatch = []  # Both artist AND title mismatch (<0.70)
    title_only = []       # Artist matches, title doesn't
    artist_only = []      # Title matches, artist doesn't

    for r in disagree:
        artist_sim = r.get('artist_similarity', 0)
        title_sim = r.get('title_similarity', 0)

        if artist_sim < 0.70 and title_sim < 0.70:
            double_mismatch.append(r)
        elif artist_sim >= 0.70 and title_sim < 0.70:
            title_only.append(r)
        elif artist_sim < 0.70 and title_sim >= 0.70:
            artist_only.append(r)

    print("=" * 70)
    print("CROSS-VALIDATION ANALYSIS")
    print("=" * 70)
    print(f"\nTotal files: {total}")
    print(f"Errors: {errors}")
    print(f"AcoustID found: {acoustid_found}")
    print(f"No AcoustID match: {no_acoustid}")
    print(f"\nAcoustID Coverage: {acoustid_found / (total - errors) * 100:.1f}%")

    print(f"\n--- AGREEMENT ANALYSIS ---")
    print(f"Sources agree: {agree}")
    print(f"Sources disagree: {len(disagree)}")
    if acoustid_found > 0:
        print(f"Agreement rate: {agree / acoustid_found * 100:.1f}%")

    print(f"\n--- DISAGREEMENT CATEGORIES ---")
    print(f"Double mismatch (BOTH <0.70): {len(double_mismatch)} ({len(double_mismatch)/len(disagree)*100:.1f}% of disagreements)")
    print(f"Title mismatch only: {len(title_only)} ({len(title_only)/len(disagree)*100:.1f}% of disagreements)")
    print(f"Artist mismatch only: {len(artist_only)} ({len(artist_only)/len(disagree)*100:.1f}% of disagreements)")

    print(f"\n--- VERIFIED GROUND TRUTH ---")
    print(f"Verified MBIDs: {verified}")
    print(f"Verification rate: {verified / total * 100:.1f}%")

    # Show detail for each category
    print("\n" + "=" * 70)
    print("DOUBLE MISMATCH (likely AcoustID errors)")
    print("=" * 70)
    for r in double_mismatch[:10]:
        print(f"\nFile: {Path(r['file_path']).name}")
        print(f"  ID3: {r.get('id3_artist')} - {r.get('id3_title')}")
        print(f"  AID: {r.get('acoustid_artist')} - {r.get('acoustid_title')}")
        print(f"  Sim: artist={r.get('artist_similarity', 0):.2f}, title={r.get('title_similarity', 0):.2f}")
        print(f"  Conf: {r.get('acoustid_confidence', 0):.2f}")

    print("\n" + "=" * 70)
    print("TITLE MISMATCH ONLY (same artist, wrong track)")
    print("=" * 70)
    for r in title_only[:10]:
        print(f"\nFile: {Path(r['file_path']).name}")
        print(f"  ID3: {r.get('id3_artist')} - {r.get('id3_title')}")
        print(f"  AID: {r.get('acoustid_artist')} - {r.get('acoustid_title')}")
        print(f"  Sim: artist={r.get('artist_similarity', 0):.2f}, title={r.get('title_similarity', 0):.2f}")
        print(f"  Conf: {r.get('acoustid_confidence', 0):.2f}")

    print("\n" + "=" * 70)
    print("ARTIST MISMATCH ONLY (possible cover version)")
    print("=" * 70)
    for r in artist_only[:10]:
        print(f"\nFile: {Path(r['file_path']).name}")
        print(f"  ID3: {r.get('id3_artist')} - {r.get('id3_title')}")
        print(f"  AID: {r.get('acoustid_artist')} - {r.get('acoustid_title')}")
        print(f"  Sim: artist={r.get('artist_similarity', 0):.2f}, title={r.get('title_similarity', 0):.2f}")
        print(f"  Conf: {r.get('acoustid_confidence', 0):.2f}")

    # Confidence distribution for disagreements
    print("\n" + "=" * 70)
    print("CONFIDENCE DISTRIBUTION FOR DISAGREEMENTS")
    print("=" * 70)
    conf_buckets = defaultdict(list)
    for r in disagree:
        conf = r.get('acoustid_confidence', 0)
        if conf >= 0.95:
            conf_buckets['95-100%'].append(r)
        elif conf >= 0.90:
            conf_buckets['90-95%'].append(r)
        elif conf >= 0.85:
            conf_buckets['85-90%'].append(r)
        elif conf >= 0.80:
            conf_buckets['80-85%'].append(r)
        else:
            conf_buckets['<80%'].append(r)

    for bucket in ['95-100%', '90-95%', '85-90%', '80-85%', '<80%']:
        count = len(conf_buckets[bucket])
        print(f"  {bucket}: {count}")

    return {
        'total': total,
        'errors': errors,
        'acoustid_found': acoustid_found,
        'agree': agree,
        'disagree': len(disagree),
        'double_mismatch': len(double_mismatch),
        'title_only': len(title_only),
        'artist_only': len(artist_only),
        'verified': verified,
    }

if __name__ == '__main__':
    results_path = sys.argv[1] if len(sys.argv) > 1 else r"C:\Users\Mango Cat\Music\single_song_crossval_results.json"
    analyze_results(results_path)
