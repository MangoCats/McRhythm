#!/usr/bin/env python3
import re
import sys

# Get filename from command line arg, default to run25
filename = sys.argv[1] if len(sys.argv) > 1 else 'album_matcher_output_run25.txt'

# Parse heartbeat messages to extract query rates
heartbeats = []

with open(filename, 'r', encoding='utf-8') as f:
    for line in f:
        # Match heartbeat lines
        match = re.search(r'\[A(\d+)\].*\[HEARTBEAT\] (\d+)s elapsed \| Queries: (\d+)', line)
        if match:
            album_num = int(match.group(1))
            elapsed_seconds = int(match.group(2))
            total_queries = int(match.group(3))
            heartbeats.append({
                'album': album_num,
                'elapsed_s': elapsed_seconds,
                'queries': total_queries
            })

# Calculate queries per minute for first heartbeat of each album
first_heartbeats = {}
for hb in heartbeats:
    album = hb['album']
    if album not in first_heartbeats or hb['elapsed_s'] < first_heartbeats[album]['elapsed_s']:
        first_heartbeats[album] = hb

print(f"MusicBrainz Query Rates per Album - {filename}")
print("(first 120s heartbeat for each album)")
print("=" * 70)

rates = []
for album in sorted(first_heartbeats.keys()):
    hb = first_heartbeats[album]
    queries_per_minute = (hb['queries'] / hb['elapsed_s']) * 60
    rates.append(queries_per_minute)
    print(f"Album {album:2d}: {hb['queries']:3d} queries in {hb['elapsed_s']:3d}s = {queries_per_minute:5.2f} queries/minute")

if rates:
    avg_rate = sum(rates) / len(rates)
    min_rate = min(rates)
    max_rate = max(rates)

    print("=" * 70)
    print(f"Average: {avg_rate:.2f} queries/minute")
    print(f"Range:   {min_rate:.2f} to {max_rate:.2f} queries/minute")
    print(f"Albums processed: {len(rates)}")

    # Calculate overall rate based on total time
    print("\n" + "=" * 70)
    print("Note: The rate varies per album because:")
    print("  - Different albums require different search strategies")
    print("  - Some albums have MusicBrainz IDs in tags (fewer queries)")
    print("  - Some need comprehensive searches (more queries)")
    print("  - Rate limiting enforces ~1 query per second maximum")
