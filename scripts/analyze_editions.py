import re
from collections import Counter

def extract_edition_winners(log_file):
    """Extract winning edition numbers from run 23 log file."""

    with open(log_file, 'r', encoding='utf-8') as f:
        content = f.read()

    # Pattern to find winning edition lines
    # Example: "Parallel processing complete. Best: 100.0% from edition 1 (mean error: 2.88s)"
    pattern = r'\[A(\d+)\].*?Parallel processing complete\. Best: ([\d.]+)% from edition (\d+)'

    matches = re.findall(pattern, content)

    results = []
    for album_idx, match_pct, edition_num in matches:
        results.append({
            'album_idx': int(album_idx),
            'match_pct': float(match_pct),
            'edition': int(edition_num)
        })

    return results

def main():
    log_file = r'c:\Users\Mango Cat\Dev\McRhythm\album_matcher_output_run23.txt'

    results = extract_edition_winners(log_file)

    print(f"Found {len(results)} successful album matches in run 23\n")

    # Count edition distribution
    edition_counts = Counter([r['edition'] for r in results])

    print("Edition Distribution:")
    print("=" * 50)
    for edition in sorted(edition_counts.keys()):
        count = edition_counts[edition]
        pct = (count / len(results)) * 100
        bar = '#' * int(pct / 2)
        print(f"Edition {edition:2d}: {count:3d} albums ({pct:5.1f}%) {bar}")

    print(f"\n{'-' * 50}")
    print(f"Total:      {len(results):3d} albums")

    # Statistics
    editions = [r['edition'] for r in results]
    print(f"\nStatistics:")
    print(f"  First edition wins:  {edition_counts[1]} / {len(results)} ({edition_counts[1]/len(results)*100:.1f}%)")
    print(f"  Mean edition rank:   {sum(editions) / len(editions):.2f}")
    print(f"  Median edition rank: {sorted(editions)[len(editions)//2]}")
    print(f"  Max edition rank:    {max(editions)}")

    # Show albums that needed higher-ranked editions
    print(f"\n{'-' * 50}")
    print("Albums that required edition 2 or higher:")
    print("=" * 50)

    for r in sorted(results, key=lambda x: x['edition'], reverse=True):
        if r['edition'] > 1:
            print(f"  Album A{r['album_idx']:3d}: Edition {r['edition']:2d} ({r['match_pct']:5.1f}%)")

if __name__ == '__main__':
    main()
