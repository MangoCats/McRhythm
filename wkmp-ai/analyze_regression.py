import json, os

with open('run29f_comparison_results.json', encoding='utf-8') as f:
    data = json.load(f)

with open('run29f_comparison_results_20260131_baseline.json', encoding='utf-8') as f:
    baseline = json.load(f)

bl = {}
for item in baseline:
    bl[item['path']] = item

exact = 0
diff_pressing = 0
regressions = []
improvements = []
new_failures = []
fixed = []

for item in data:
    path = item['path']
    b = bl.get(path)
    if not b:
        continue

    cur_mbid = item.get('current_mbid')
    bl_mbid = b.get('current_mbid') or b.get('baseline_mbid')
    cur_album = item.get('album', '')
    bl_album = b.get('album', '')
    cur_matched = item.get('matched', False)
    bl_matched = b.get('matched', False)
    cur_pct = item.get('match_percentage', 0) or 0
    bl_pct = b.get('match_percentage', 0) or 0

    if cur_mbid == bl_mbid:
        exact += 1
    elif cur_matched and bl_matched:
        fname = os.path.basename(path)
        entry = {
            'file': fname,
            'old_album': bl_album,
            'new_album': cur_album,
            'old_pct': bl_pct,
            'new_pct': cur_pct,
        }

        if cur_pct > bl_pct + 5:
            improvements.append(entry)
        elif cur_pct < bl_pct - 5:
            regressions.append(entry)
        else:
            diff_pressing += 1
    elif cur_matched and not bl_matched:
        fname = os.path.basename(path)
        fixed.append({'file': fname, 'album': cur_album, 'pct': cur_pct})
    elif not cur_matched and bl_matched:
        fname = os.path.basename(path)
        new_failures.append({'file': fname, 'album': bl_album, 'pct': bl_pct})

print('=== REGRESSION TEST RESULTS ===')
print(f'Total albums: {len(data)}')
print(f'Exact MBID match: {exact}')
print(f'Different pressings (neutral): {diff_pressing}')
print(f'Improvements: {len(improvements)}')
print(f'Regressions: {len(regressions)}')
print(f'Newly fixed: {len(fixed)}')
print(f'New failures: {len(new_failures)}')

if improvements:
    print(f'\n=== IMPROVEMENTS ({len(improvements)}) ===')
    for e in improvements:
        print(f'  {e["file"]}:')
        print(f'    OLD: {e["old_album"]} ({e["old_pct"]}%)')
        print(f'    NEW: {e["new_album"]} ({e["new_pct"]}%)')

if regressions:
    print(f'\n=== REGRESSIONS ({len(regressions)}) ===')
    for e in regressions:
        print(f'  {e["file"]}:')
        print(f'    OLD: {e["old_album"]} ({e["old_pct"]}%)')
        print(f'    NEW: {e["new_album"]} ({e["new_pct"]}%)')

if fixed:
    print(f'\n=== NEWLY FIXED ({len(fixed)}) ===')
    for e in fixed:
        print(f'  {e["file"]}: {e["album"]} ({e["pct"]}%)')

if new_failures:
    print(f'\n=== NEW FAILURES ({len(new_failures)}) ===')
    for e in new_failures:
        print(f'  {e["file"]}: was {e["album"]} ({e["pct"]}%)')
