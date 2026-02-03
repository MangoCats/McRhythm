#!/usr/bin/env python3
"""Extract album list from run29f results for comparison testing"""

import json

with open('album_matcher_results_run29f.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

print(f"// run29f baseline: {len(data)} albums")
print("vec![")

for item in data:
    path = item['album_path']
    # Remove the music library prefix
    if 'Music\\' in path:
        path = path.split('Music\\', 1)[1]
    elif 'Music/' in path:
        path = path.split('Music/', 1)[1]

    # Escape backslashes and quotes
    path = path.replace('\\', '/')
    artist = item['artist'].replace('"', '\\"')
    album = item['album'].replace('"', '\\"')
    expected = item['expected_track_count']
    mbid = item['mbid']

    print(f'    ("{path}", "{artist}", "{album}", {expected}, "{mbid}"),')

print("]")
