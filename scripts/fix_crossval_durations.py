#!/usr/bin/env python3
"""
Fix crossval results with accurate durations for VBR MP3 files.

This script:
1. Identifies files with suspicious durations (implied bitrate < 40 kbps)
2. Computes accurate durations by counting MP3 frames
3. Saves corrected crossval results
"""

import json
import os
import sys
from concurrent.futures import ProcessPoolExecutor, as_completed

sys.stdout.reconfigure(encoding='utf-8')

# MP3 bitrate table for MPEG1 Layer 3
BITRATES_MPEG1_L3 = [0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0]
SAMPLE_RATES_MPEG1 = [44100, 48000, 32000]

def is_frame_sync(b1, b2):
    """Check if bytes are MP3 frame sync."""
    return b1 == 0xFF and (b2 & 0xE0) == 0xE0

def get_frame_info(header):
    """Parse MP3 frame header, return (frame_size, sample_rate, samples_per_frame) or None."""
    version = (header[1] >> 3) & 0x03
    layer = (header[1] >> 1) & 0x03
    bitrate_idx = (header[2] >> 4) & 0x0F
    sample_rate_idx = (header[2] >> 2) & 0x03
    padding = (header[2] >> 1) & 0x01

    # MPEG1 Layer 3 only
    if version != 0x03 or layer != 0x01:
        return None
    if bitrate_idx == 0 or bitrate_idx == 15:
        return None
    if sample_rate_idx >= len(SAMPLE_RATES_MPEG1):
        return None

    bitrate = BITRATES_MPEG1_L3[bitrate_idx] * 1000
    sample_rate = SAMPLE_RATES_MPEG1[sample_rate_idx]
    frame_size = int(144 * bitrate / sample_rate) + padding
    samples_per_frame = 1152

    return frame_size, sample_rate, samples_per_frame

def count_mp3_frames(file_path):
    """Count MP3 frames and calculate accurate duration."""
    try:
        with open(file_path, 'rb') as f:
            data = f.read()
    except Exception as e:
        return None, str(e)

    file_size = len(data)
    pos = 0

    # Skip ID3v2 tag
    if data[:3] == b'ID3':
        size_bytes = data[6:10]
        tag_size = (size_bytes[0] << 21) | (size_bytes[1] << 14) | (size_bytes[2] << 7) | size_bytes[3]
        pos = 10 + tag_size

    frame_count = 0
    total_samples = 0
    sample_rate = 44100

    while pos < file_size - 4:
        if is_frame_sync(data[pos], data[pos+1]):
            header = data[pos:pos+4]
            result = get_frame_info(header)
            if result:
                frame_size, sample_rate, samples = result
                frame_count += 1
                total_samples += samples
                pos += frame_size
                continue
        pos += 1

    duration = total_samples / sample_rate if sample_rate > 0 else 0
    return duration, None

def is_duration_suspicious(duration, file_size):
    """Check if duration is suspicious (implies < 40 kbps)."""
    if duration <= 0:
        return True
    implied_bitrate_kbps = (file_size * 8) / (duration * 1000)
    return implied_bitrate_kbps < 40

def process_file(args):
    """Process a single file - for parallel execution."""
    file_path, lofty_duration = args
    if not os.path.exists(file_path):
        return file_path, None, "File not found"

    file_size = os.path.getsize(file_path)

    # Check if duration is suspicious
    if not is_duration_suspicious(lofty_duration, file_size):
        return file_path, lofty_duration, None  # Keep original

    # Compute accurate duration
    accurate_duration, error = count_mp3_frames(file_path)
    if error:
        return file_path, lofty_duration, error

    return file_path, accurate_duration, None

def main():
    input_path = r'C:\Users\Mango Cat\Music\single_song_crossval_results.json'
    output_path = r'C:\Users\Mango Cat\Music\single_song_crossval_results_fixed.json'

    print("Loading crossval results...")
    with open(input_path, 'r', encoding='utf-8') as f:
        results = json.load(f)

    print(f"Loaded {len(results)} files")

    # Identify suspicious files
    suspicious = []
    normal = []
    for r in results:
        file_path = r.get('file_path', '')
        duration = r.get('duration_secs', 0)
        if not os.path.exists(file_path):
            normal.append(r)
            continue

        file_size = os.path.getsize(file_path)
        if is_duration_suspicious(duration, file_size):
            suspicious.append(r)
        else:
            normal.append(r)

    print(f"Found {len(suspicious)} files with suspicious durations")
    print(f"Found {len(normal)} files with normal durations")
    print()

    if not suspicious:
        print("No suspicious files to fix!")
        return

    # Process suspicious files in parallel
    print(f"Computing accurate durations for {len(suspicious)} files...")
    print()

    # Prepare arguments for parallel processing
    process_args = [(r['file_path'], r.get('duration_secs', 0)) for r in suspicious]

    fixed_count = 0
    error_count = 0
    duration_map = {}

    # Process in parallel
    with ProcessPoolExecutor(max_workers=4) as executor:
        futures = {executor.submit(process_file, args): args[0] for args in process_args}

        for i, future in enumerate(as_completed(futures)):
            if (i + 1) % 100 == 0 or i + 1 == len(futures):
                print(f"\r  Progress: {i+1}/{len(futures)} ({100*(i+1)/len(futures):.1f}%)", end='', flush=True)

            file_path, accurate_duration, error = future.result()
            if error:
                error_count += 1
            elif accurate_duration is not None:
                duration_map[file_path] = accurate_duration
                fixed_count += 1

    print()
    print()
    print(f"Fixed {fixed_count} durations, {error_count} errors")

    # Update results with fixed durations
    fixed_results = []
    for r in results:
        file_path = r.get('file_path', '')
        if file_path in duration_map:
            old_duration = r.get('duration_secs', 0)
            new_duration = duration_map[file_path]
            r['duration_secs'] = new_duration
            r['duration_secs_original'] = old_duration
            r['duration_fixed'] = True
        fixed_results.append(r)

    # Save fixed results
    print(f"\nSaving to {output_path}...")
    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(fixed_results, f, indent=2, ensure_ascii=False)

    # Summary
    print()
    print("=" * 70)
    print("SUMMARY")
    print("=" * 70)
    print(f"Total files:          {len(results)}")
    print(f"Suspicious durations: {len(suspicious)}")
    print(f"Fixed:                {fixed_count}")
    print(f"Errors:               {error_count}")
    print()

    # Show some examples
    print("Sample fixes:")
    samples = [r for r in fixed_results if r.get('duration_fixed')][:10]
    for r in samples:
        old = r.get('duration_secs_original', 0)
        new = r.get('duration_secs', 0)
        ratio = old / new if new > 0 else 0
        artist = r.get('id3_artist', '')[:20]
        title = r.get('id3_title', '')[:25]
        print(f"  {artist} - {title}")
        print(f"    Old: {old:.1f}s ({old/60:.1f}m) -> New: {new:.1f}s ({new/60:.1f}m) [{ratio:.1f}x fix]")

if __name__ == '__main__':
    main()
