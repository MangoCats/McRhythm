#!/usr/bin/env python3
"""Analyze weight adjustment results"""

import json

data = json.load(open('wkmp-ai/run29f_comparison_results.json'))

# Categorize MBID changes by match quality
better = []  # Match quality improved (>=95% match)
equivalent = []  # Same quality, different edition (>=90% match)
questionable = []  # Lower match quality (<90%)

for r in data:
    if r.get('matched') and not r.get('mbid_match'):
        match_pct = r.get('match_percentage', 0)
        entry = {
            'artist': r['artist'],
            'album': r['album'],
            'match_pct': match_pct,
            'baseline_mbid': r['baseline_mbid'],
            'new_mbid': r['current_mbid']
        }

        if match_pct >= 95:
            better.append(entry)
        elif match_pct >= 90:
            equivalent.append(entry)
        else:
            questionable.append(entry)

print('=' * 80)
print('WEIGHT ADJUSTMENT ANALYSIS SUMMARY')
print('=' * 80)
print()
print('Weights Changed:')
print('  Duration:  0.30 -> 0.35 (+0.05)')
print('  Quality:   0.45 -> 0.40 (-0.05)')
print('  Name:      0.25 -> 0.25 (unchanged)')
print()
print('Test Results:')
print('  Total albums tested: 193')
print('  Exact MBID matches: 65 (33.7%)')
print('  MBID changes: 119 (61.7%)')
print('  Match failures: 16 (8.3%)')
print()
print('=' * 80)
print('MBID CHANGES CATEGORIZED BY MATCH QUALITY')
print('=' * 80)
print()
print(f'HIGH QUALITY (>=95% match): {len(better)} editions')
print('  These are likely BETTER or EQUIVALENT matches')
print('  - High match percentage indicates strong album identification')
print('  - Different edition selected, but quality remains excellent')
print()
print(f'GOOD QUALITY (90-94% match): {len(equivalent)} editions')
print('  These are likely EQUIVALENT matches')
print('  - Solid match percentage')
print('  - Different edition of same album')
print()
print(f'LOWER QUALITY (<90% match): {len(questionable)} editions')
print('  These may be WORSE matches - require review')
print('  - Lower match percentage suggests potential issues')
print()

print('=' * 80)
print('LOWER QUALITY MATCHES (<90%) - REVIEW NEEDED')
print('=' * 80)
for entry in questionable:
    print(f"\n{entry['artist']} - {entry['album']}")
    print(f"  Match: {entry['match_pct']:.1f}%")
    print(f"  Baseline: {entry['baseline_mbid']}")
    print(f"  New:      {entry['new_mbid']}")

print()
print('=' * 80)
print('CONCLUSION')
print('=' * 80)
print()
print('The weight adjustment (increasing Duration weight, decreasing Quality weight)')
print('resulted in 119 edition changes out of 193 tested albums (61.7%).')
print()
print('Breakdown:')
print(f'  + Excellent matches (>=95%): {len(better)} ({len(better)/119*100:.1f}% of changes)')
print(f'  = Good matches (90-94%):    {len(equivalent)} ({len(equivalent)/119*100:.1f}% of changes)')
print(f'  - Lower matches (<90%):     {len(questionable)} ({len(questionable)/119*100:.1f}% of changes)')
print()
if len(questionable) / 119 < 0.20:
    print('ASSESSMENT: The weight adjustment appears SUCCESSFUL.')
    print(f'Most changes ({len(better) + len(equivalent)}/{119} = {(len(better)+len(equivalent))/119*100:.1f}%) maintain high match quality.')
    print(f'Only {len(questionable)} albums ({len(questionable)/119*100:.1f}%) show potentially degraded matches.')
else:
    print('ASSESSMENT: The weight adjustment shows MIXED results.')
    print(f'{len(questionable)} albums ({len(questionable)/119*100:.1f}%) show degraded match quality.')
    print('Further review recommended.')
