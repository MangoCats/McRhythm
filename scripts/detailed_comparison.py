import json

# Load both result files
with open('am/run29f_comparison_results_before_negative_floor.json') as f:
    before = json.load(f)

with open('am/run29f_comparison_results_with_negative_floor.json') as f:
    after = json.load(f)

# Create lookup by path
before_map = {r['path']: r for r in before}
after_map = {r['path']: r for r in after}

# Track status changes
became_exact = []
lost_exact = []

for path in sorted(before_map.keys()):
    if path not in after_map:
        continue
    
    b = before_map[path]
    a = after_map[path]
    
    b_exact = b.get('mbid_match', False) and b.get('tracks_match', False)
    a_exact = a.get('mbid_match', False) and a.get('tracks_match', False)
    
    if not b_exact and a_exact:
        became_exact.append({
            'path': path,
            'artist': b['artist'],
            'album': b['album'],
        })
    elif b_exact and not a_exact:
        lost_exact.append({
            'path': path,
            'artist': b['artist'],
            'album': b['album'],
        })

print("\n=== Status Changes ===\n")
print(f"Albums that became EXACT MATCH: {len(became_exact)}")
for album in became_exact:
    print(f"  + {album['artist']} - {album['album']}")

print(f"\nAlbums that lost EXACT MATCH: {len(lost_exact)}")
for album in lost_exact:
    print(f"  - {album['artist']} - {album['album']}")
