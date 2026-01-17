#!/usr/bin/env python3
"""Test script to verify duration fixes are working."""

import json
import subprocess
import os

# Load a few anomalous files from crossval results
with open(r'C:\Users\Mango Cat\Music\single_song_crossval_results.json', 'r', encoding='utf-8') as f:
    results = json.load(f)

# Find files with duration > 10 minutes (anomalies)
anomalies = []
for r in results:
    dur = r.get('duration_secs', 0)
    if dur > 600:  # > 10 minutes
        anomalies.append({
            'path': r.get('file_path', ''),
            'lofty_duration': dur,
            'id3_title': r.get('id3_title'),
            'id3_artist': r.get('id3_artist'),
        })

print(f"Found {len(anomalies)} files with duration > 10 minutes")
print()

# Test FFprobe on first 5 anomalies
for a in anomalies[:5]:
    path = a['path']
    if not os.path.exists(path):
        continue

    # Get FFprobe duration
    try:
        result = subprocess.run(
            ['ffprobe', '-v', 'error', '-show_entries', 'format=duration',
             '-of', 'default=noprint_wrappers=1:nokey=1', path],
            capture_output=True, text=True, timeout=10
        )
        ffprobe_duration = float(result.stdout.strip())
    except:
        ffprobe_duration = 0

    file_size = os.path.getsize(path) / (1024 * 1024)

    print(f"{a['id3_artist']} - {a['id3_title']}")
    print(f"  File: {os.path.basename(path)}")
    print(f"  Lofty duration: {a['lofty_duration']:.1f}s ({a['lofty_duration']/60:.1f} min)")
    print(f"  FFprobe duration: {ffprobe_duration:.1f}s ({ffprobe_duration/60:.1f} min)")
    print(f"  Error ratio: {a['lofty_duration']/ffprobe_duration:.2f}x")
    print(f"  File size: {file_size:.2f} MB")
    print()
