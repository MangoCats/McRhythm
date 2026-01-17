import json
import sys

# Load both result files
with open('am/run29f_comparison_results_before_negative_floor.json') as f:
    before = json.load(f)

with open('am/run29f_comparison_results_with_negative_floor.json') as f:
    after = json.load(f)

# Create lookup by path
before_map = {r['path']: r for r in before}
after_map = {r['path']: r for r in after}

# Find differences
changes = []
for path in sorted(before_map.keys()):
    if path not in after_map:
        continue
    
    b = before_map[path]
    a = after_map[path]
    
    # Check if MBID changed
    b_mbid = b.get('current_mbid', '')
    a_mbid = a.get('current_mbid', '')
    
    if b_mbid != a_mbid:
        changes.append({
            'path': path,
            'artist': b['artist'],
            'album': b['album'],
            'before_mbid': b_mbid,
            'after_mbid': a_mbid,
            'before_tracks': b.get('current_tracks'),
            'after_tracks': a.get('current_tracks'),
        })

print(f"\n=== MBID Changes Due to -1.0 Negative Floor ===\n")
print(f"Total albums with MBID changes: {len(changes)}\n")

if changes:
    for i, change in enumerate(changes, 1):
        print(f"{i}. {change['artist']} - {change['album']}")
        print(f"   Path: {change['path']}")
        print(f"   Before: {change['before_mbid']} ({change['before_tracks']} tracks)")
        print(f"   After:  {change['after_mbid']} ({change['after_tracks']} tracks)")
        print()
else:
    print("No MBID changes detected between runs.")
