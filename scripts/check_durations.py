#!/usr/bin/env python3
"""Check duration anomalies in crossval results."""

import json
import os

with open(r'C:\Users\Mango Cat\Music\single_song_crossval_results.json', 'r', encoding='utf-8') as f:
    results = json.load(f)

# Look at files with very long durations (> 600 seconds = 10 minutes)
anomalies = []
for r in results:
    dur = r.get('duration_secs', 0)
    if dur > 600:  # > 10 minutes
        anomalies.append({
            'file': os.path.basename(r.get('file_path', '')),
            'full_path': r.get('file_path', ''),
            'duration': dur,
            'id3_title': r.get('id3_title'),
            'id3_artist': r.get('id3_artist'),
        })

print(f'Files with duration > 10 minutes: {len(anomalies)}')
print()
for a in anomalies[:20]:
    print(f"{a['duration']:.1f}s = {a['duration']/60:.1f}m: {a['id3_artist']} - {a['id3_title']}")
    print(f"  File: {a['file']}")

# Also check the bitrate distribution
print("\n\nBitrate analysis:")
durations = [r.get('duration_secs', 0) for r in results]
print(f"Total files: {len(results)}")
print(f"Min duration: {min(durations):.1f}s")
print(f"Max duration: {max(durations):.1f}s")
print(f"Files > 10min: {sum(1 for d in durations if d > 600)}")
print(f"Files > 20min: {sum(1 for d in durations if d > 1200)}")
print(f"Files > 30min: {sum(1 for d in durations if d > 1800)}")

# Calculate implied bitrate for anomalies
print("\n\nImplied bitrate analysis for anomalies:")
for a in anomalies[:5]:
    path = a['full_path']
    if os.path.exists(path):
        file_size = os.path.getsize(path)
        duration = a['duration']
        implied_bitrate = (file_size * 8) / (duration * 1000)  # kbps
        print(f"{a['file']}:")
        print(f"  File size: {file_size/1024/1024:.2f} MB")
        print(f"  Duration (our data): {duration:.1f}s = {duration/60:.1f}m")
        print(f"  Implied bitrate: {implied_bitrate:.0f} kbps")
