import json

# Load run 28
with open('album_matcher_results.json', 'r', encoding='utf-8') as f:
    run28 = json.load(f)

# Extract run 28 worst albums
run28_sorted = sorted(run28, key=lambda x: x.get('match_percentage', 0))

print('=== RUN 27 vs RUN 28 COMPARISON ===\n')
print('WORST 5 ALBUMS - RUN 28 (Modular):')
for i, album in enumerate(run28_sorted[:5], 1):
    print(f'  {i}. {album["artist"]} - {album["album"]}: {album["match_percentage"]:.1f}% ({album["confidence"]})')

print('\nWORST 5 ALBUMS - RUN 27 (Monolithic):')
print('  1. Various - The Greatest Showman: 66.7% (Good)')
print('  2. Steve Howe - Anthology: 75.0% (Good)')
print('  3. Blackmore Night - Beyond Sunset: 76.5% (Good)')
print('  4. Jethro Tull - Aqualung: 78.6% (Good)')
print('  5. King Crimson - Power To Believe: 81.8% (Excellent)')

# Find regressions
print('\n=== MAJOR REGRESSIONS ===')
print('Album                                  Run27   Run28    Delta')
print('-' * 70)
for album in run28:
    name = f'{album["artist"]} - {album["album"]}'
    pct = album['match_percentage']
    delta = None
    old = None
    if 'Jethro Tull' in name and 'Aqualung' in name:
        old, delta = 78.6, pct - 78.6
    elif 'M83' in name and 'Junk' in name:
        old, delta = 100.0, pct - 100.0
    elif 'Steppenwolf' in name and 'ABC' in name:
        old, delta = 100.0, pct - 100.0
    elif 'Michael McDonald' in name and 'Takes' in name:
        old, delta = 100.0, pct - 100.0
    
    if delta is not None and delta < -10:
        print(f'{name[:38]:38} {old:6.1f}% {pct:6.1f}%  {delta:+6.1f}%')

# Aggregate stats
excellent = sum(1 for a in run28 if a["confidence"] == "Excellent")
good = sum(1 for a in run28 if a["confidence"] == "Good")
fair = sum(1 for a in run28 if a["confidence"] == "Fair")
poor = sum(1 for a in run28 if a["confidence"] == "Poor")

print('\n=== AGGREGATE STATISTICS ===')
print('                Run 27    Run 28')
print(f'Excellent:      133       {excellent}')
print(f'Good:           45        {good}')
print(f'Fair:           14        {fair}')
print(f'Poor:           2         {poor}')
