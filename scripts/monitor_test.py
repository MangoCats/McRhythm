#!/usr/bin/env python3
"""
Monitor full library test and report status every 600 seconds.
Focus on improvement validation compared to am29 performance.
"""

import time
import re
import sys
from datetime import datetime

def extract_stats(logfile):
    """Extract key statistics from test log."""
    try:
        with open(logfile, 'r', encoding='utf-8') as f:
            content = f.read()
    except FileNotFoundError:
        return None

    stats = {
        'files_processed': 0,
        'total_files': 0,
        'passages_created': 0,
        'chromaprint_failures': 0,
        'audio_derived_failures': 0,
        'album_match_successes': 0,
        'album_match_failures': 0,
        'album_match_fallbacks': 0,
        'cached_audio_reuse': 0,
        'happy_nation_status': 'Not yet processed',
        'elapsed_time': 0,
    }

    # Count files processed
    file_completed = re.findall(r'FILE_COMPLETE \[(\d+)/(\d+)\]', content)
    if file_completed:
        last = file_completed[-1]
        stats['files_processed'] = int(last[0])
        stats['total_files'] = int(last[1])

    # Count passages
    passages = re.findall(r'FILE_COMPLETE.*?(\d+) passages', content)
    stats['passages_created'] = sum(int(p) for p in passages)

    # Count failures
    stats['chromaprint_failures'] = content.count('Chromaprint extraction failed')
    stats['audio_derived_failures'] = content.count('Audio-derived extraction failed')

    # Album matching stats
    stats['album_match_successes'] = content.count('AlbumMatchingCompleted')
    stats['album_match_failures'] = content.count('AlbumMatchingFailed')
    stats['album_match_fallbacks'] = content.count('AlbumMatchingFallback')

    # Count cached audio reuse
    stats['cached_audio_reuse'] = content.count('Processing file with') + content.count('cached (reused from album matcher)')

    # Check HappyNation.mp3 status
    if 'HappyNation.mp3' in content:
        if 'Processing file with' in content and 'HappyNation' in content:
            stats['happy_nation_status'] = 'IMPROVEMENT: Reused cached audio'
        elif content.count('HappyNation') > 1:
            if 'No audio samples provided' in content:
                stats['happy_nation_status'] = 'FAILED: Same as before (0 samples)'
            else:
                stats['happy_nation_status'] = 'PROCESSED: Check details'

    # Extract elapsed time
    time_match = re.search(r'total_elapsed: ([\d.]+)s', content)
    if time_match:
        stats['elapsed_time'] = float(time_match.group(1))

    return stats

def print_report(stats, iteration):
    """Print status report."""
    print(f"\n{'='*80}")
    print(f"STATUS REPORT #{iteration} - {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"{'='*80}")

    if stats is None:
        print("ERROR: Log file not found yet. Test may not have started.")
        return

    print(f"\nPROGRESS:")
    print(f"  Files: {stats['files_processed']}/{stats['total_files']} ({stats['files_processed']/max(stats['total_files'],1)*100:.1f}%)")
    print(f"  Passages created: {stats['passages_created']}")
    print(f"  Elapsed time: {stats['elapsed_time']:.1f}s ({stats['elapsed_time']/60:.1f} minutes)")

    if stats['files_processed'] > 0:
        print(f"  Processing rate: {stats['files_processed']/(stats['elapsed_time']/60):.2f} files/minute")
        remaining = stats['total_files'] - stats['files_processed']
        eta_minutes = remaining / (stats['files_processed']/(stats['elapsed_time']/60)) if stats['files_processed'] > 0 else 0
        print(f"  ETA: {eta_minutes:.0f} minutes ({eta_minutes/60:.1f} hours)")

    print(f"\nIMPROVEMENT VALIDATION:")
    print(f"  Album Match Successes: {stats['album_match_successes']}")
    print(f"  Album Match Failures: {stats['album_match_failures']}")
    print(f"  Album Match Fallbacks: {stats['album_match_fallbacks']}")
    print(f"  Cached Audio Reuse: {stats['cached_audio_reuse']} (IMPROVEMENT#1)")

    print(f"\nEXTRACTOR FAILURES:")
    print(f"  Chromaprint: {stats['chromaprint_failures']}")
    print(f"  AudioDerived: {stats['audio_derived_failures']}")

    print(f"\nHAPPYNATION.MP3 STATUS (Key test case):")
    print(f"  {stats['happy_nation_status']}")

    print(f"\n{'='*80}\n")
    sys.stdout.flush()

def main():
    logfile = r'full_library_test_improvements.txt'
    interval = 600  # 10 minutes
    iteration = 0

    print(f"Monitoring {logfile} every {interval} seconds...")
    print(f"Focus: Validate improvements vs am29 performance")
    print(f"Key metrics: Album match fallbacks with cached audio reuse, HappyNation.mp3 processing\n")

    while True:
        iteration += 1
        stats = extract_stats(logfile)
        print_report(stats, iteration)

        # Exit if test completed
        if stats and stats['files_processed'] == stats['total_files'] and stats['total_files'] > 0:
            print("TEST COMPLETED!")
            break

        time.sleep(interval)

if __name__ == '__main__':
    main()
