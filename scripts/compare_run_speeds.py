#!/usr/bin/env python3
import re
from datetime import datetime

def parse_timestamp(ts_str):
    """Parse timestamp like '2025-11-24T18:39:34.636286-05:00'"""
    # Remove the ANSI color codes
    ts_str = re.sub(r'\x1b\[.*?m', '', ts_str)
    # Extract just the timestamp part
    match = re.search(r'(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})\.\d+-\d{2}:\d{2}', ts_str)
    if match:
        return datetime.strptime(match.group(1), '%Y-%m-%dT%H:%M:%S')
    return None

def parse_album_events(filename):
    """Parse album start and completion events from log file"""
    starts = {}
    completions = {}

    with open(filename, 'r', encoding='utf-8') as f:
        for line in f:
            # Look for album starts
            start_match = re.search(r'(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+-\d{2}:\d{2}).*\[A(\d+)\].*Starting parallel: decode', line)
            if start_match:
                ts = parse_timestamp(start_match.group(1))
                album_num = int(start_match.group(2))
                if album_num not in starts:  # Only keep first start
                    starts[album_num] = ts

            # Look for album completions (MusicBrainz match found)
            complete_match = re.search(r'(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+-\d{2}:\d{2}).*\[A(\d+)\].*MusicBrainz: https://musicbrainz.org/release', line)
            if complete_match:
                ts = parse_timestamp(complete_match.group(1))
                album_num = int(complete_match.group(2))
                completions[album_num] = ts

    return starts, completions

# Parse both runs
print("Parsing run 25 (MAX_CONCURRENT_ALBUMS=8)...")
starts_25, completions_25 = parse_album_events('album_matcher_output_run25.txt')

print("Parsing run 25b (MAX_CONCURRENT_ALBUMS=16)...")
starts_25b, completions_25b = parse_album_events('album_matcher_output_run25b.txt')

# Find common albums that completed in both runs
common_albums = sorted(set(completions_25.keys()) & set(completions_25b.keys()))

print("\n" + "="*80)
print("ALBUM PROCESSING SPEED COMPARISON")
print("="*80)
print(f"Common albums completed in both runs: {len(common_albums)}")
print()

if common_albums:
    # Calculate elapsed time from start to completion for each album
    start_25 = min(starts_25.values())
    start_25b = min(starts_25b.values())

    print(f"Run 25  started at: {start_25}")
    print(f"Run 25b started at: {start_25b}")
    print()

    print("Album | Run 25 (8 concurrent) | Run 25b (16 concurrent) | Difference")
    print("------|-----------------------|-------------------------|------------")

    for album in common_albums:
        if album in completions_25 and album in completions_25b:
            elapsed_25 = (completions_25[album] - start_25).total_seconds()
            elapsed_25b = (completions_25b[album] - start_25b).total_seconds()
            diff = elapsed_25b - elapsed_25
            diff_pct = (diff / elapsed_25) * 100 if elapsed_25 > 0 else 0

            print(f"A{album:2d}   | {elapsed_25:8.1f}s ({elapsed_25/60:5.1f}m) | {elapsed_25b:8.1f}s ({elapsed_25b/60:5.1f}m) | {diff:+7.1f}s ({diff_pct:+5.1f}%)")

    # Calculate average processing time
    avg_25 = sum((completions_25[a] - start_25).total_seconds() for a in common_albums) / len(common_albums)
    avg_25b = sum((completions_25b[a] - start_25b).total_seconds() for a in common_albums) / len(common_albums)

    print("------|-----------------------|-------------------------|------------")
    print(f"Avg   | {avg_25:8.1f}s ({avg_25/60:5.1f}m) | {avg_25b:8.1f}s ({avg_25b/60:5.1f}m) | {avg_25b-avg_25:+7.1f}s ({((avg_25b-avg_25)/avg_25)*100:+5.1f}%)")

    print("\n" + "="*80)
    print("INTERPRETATION:")
    print("="*80)
    if avg_25b < avg_25:
        speedup = (avg_25 / avg_25b - 1) * 100
        print(f"[+] Run 25b is FASTER by {speedup:.1f}% on average")
        print(f"    Doubling concurrency (8->16) improved throughput")
    elif avg_25b > avg_25:
        slowdown = (avg_25b / avg_25 - 1) * 100
        print(f"[!] Run 25b is SLOWER by {slowdown:.1f}% on average")
        print(f"    Doubling concurrency (8->16) did not improve throughput")
        print(f"    Likely bottlenecked by rate limiting or I/O contention")
    else:
        print("= Runs are approximately equal in speed")
        print("  Concurrency change had minimal impact")

    # Check if run 25b is still running
    max_album_25 = max(completions_25.keys())
    max_album_25b = max(completions_25b.keys())

    print()
    print(f"Albums completed: Run 25 = {len(completions_25)}, Run 25b = {len(completions_25b)}")
    if max_album_25b < max_album_25:
        print(f"Note: Run 25b is still processing (latest = A{max_album_25b})")
else:
    print("No common completed albums found yet")
