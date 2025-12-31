# Album Matcher 28 (Modular) - Usage

## Command Line Parameters

### Cache Mode

- `--use-cache` - Read-only cache mode (cache hits only, errors on misses)
- `--no-cache` - Disable cache entirely
- (default) - ReadWrite mode (check cache, query API on miss, store results)

### Output JSON File

- `--output <filename>` or `-o <filename>` - Specify output JSON filename
- (default) - `album_matcher_results.json`

## Examples

### Run with default settings
```bash
cargo run --example am28 --release -- test_quick.txt
```
Output: `album_matcher_results.json`

### Run with versioned output (Run 29)
```bash
cargo run --example am28 --release -- test_quick.txt --output album_matcher_results_run29.json
```
Output: `album_matcher_results_run29.json`

### Run with cache-only mode and versioned output
```bash
cargo run --example am28 --release -- test_quick.txt --use-cache -o album_matcher_results_run30.json
```
Output: `album_matcher_results_run30.json`

### Capture both log and JSON with matching run numbers
```bash
cargo run --example am28 --release -- test_quick.txt --output album_matcher_results_run31.json > album_matcher_output_run31.txt 2>&1
```
Output:
- `album_matcher_output_run31.txt` (log file)
- `album_matcher_results_run31.json` (JSON results)

## Recommended Workflow

For each test run, use matching run numbers for both files:

```bash
RUN_NUM=32
cargo run --example am28 --release -- test_quick.txt \
  --output album_matcher_results_run${RUN_NUM}.json \
  > album_matcher_output_run${RUN_NUM}.txt 2>&1
```

This creates:
- `album_matcher_output_run32.txt`
- `album_matcher_results_run32.json`

## Comparing Runs

With versioned JSON files, you can now easily compare results:

```python
import json

# Load multiple runs
with open('album_matcher_results_run27.json') as f:
    run27 = json.load(f)

with open('album_matcher_results_run28.json') as f:
    run28 = json.load(f)

# Compare results
for r27, r28 in zip(run27, run28):
    if abs(r27['match_percentage'] - r28['match_percentage']) > 0.1:
        print(f"{r27['artist']} - {r27['album']}: "
              f"{r27['match_percentage']:.1f}% → {r28['match_percentage']:.1f}%")
```

## File Preservation

**Current Run 28 results preserved as:**
- `album_matcher_results_run28.json`

Future runs should use explicit `--output` parameter to avoid overwriting previous results.
