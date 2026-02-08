#!/usr/bin/env python3
"""
Verify symphonia duration accuracy by counting MP3 frames.
This replicates what symphonia does: count actual frames * 1152 / sample_rate.
"""

import json
import os
import sys
import struct
from collections import Counter

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

    # MPEG1 Layer 3 only for now
    if version != 0x03 or layer != 0x01:
        return None
    if bitrate_idx == 0 or bitrate_idx == 15:
        return None
    if sample_rate_idx >= len(SAMPLE_RATES_MPEG1):
        return None

    bitrate = BITRATES_MPEG1_L3[bitrate_idx] * 1000
    sample_rate = SAMPLE_RATES_MPEG1[sample_rate_idx]
    frame_size = int(144 * bitrate / sample_rate) + padding
    samples_per_frame = 1152  # Always 1152 for MPEG1 Layer 3

    return frame_size, sample_rate, samples_per_frame

def count_mp3_frames(file_path):
    """Count MP3 frames and calculate accurate duration."""
    with open(file_path, 'rb') as f:
        data = f.read()

    file_size = len(data)
    pos = 0

    # Skip ID3v2 tag
    if data[:3] == b'ID3':
        size_bytes = data[6:10]
        tag_size = (size_bytes[0] << 21) | (size_bytes[1] << 14) | (size_bytes[2] << 7) | size_bytes[3]
        pos = 10 + tag_size

    frame_count = 0
    total_samples = 0
    sample_rate = 44100  # Default

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
    return frame_count, total_samples, sample_rate, duration

def main():
    # Load crossval results
    with open(r'C:\Users\Mango Cat\Music\single_song_crossval_results.json', 'r', encoding='utf-8') as f:
        results = json.load(f)

    # Find anomalous files (lofty duration > 600s)
    anomalies = []
    for r in results:
        dur = r.get('duration_secs', 0)
        if dur > 600:
            anomalies.append(r)

    print(f"Found {len(anomalies)} anomalous files (lofty duration > 10 min)")
    print()

    # Test first 10 anomalies
    test_files = []
    for r in anomalies[:20]:
        path = r.get('file_path', '')
        if os.path.exists(path):
            test_files.append({
                'path': path,
                'lofty_duration': r.get('duration_secs', 0),
                'artist': r.get('id3_artist', ''),
                'title': r.get('id3_title', ''),
            })

    print("Verifying frame counts (symphonia method):")
    print("=" * 80)

    verified_correct = 0
    for tf in test_files[:10]:
        frame_count, total_samples, sample_rate, accurate_duration = count_mp3_frames(tf['path'])

        lofty_dur = tf['lofty_duration']
        error_ratio = lofty_dur / accurate_duration if accurate_duration > 0 else 0

        # Check if accurate duration is reasonable (< 10 minutes for pop songs)
        is_reasonable = 60 < accurate_duration < 600

        status = "✓" if is_reasonable else "?"
        if is_reasonable:
            verified_correct += 1

        print(f"{status} {tf['artist'][:20]} - {tf['title'][:25]}")
        print(f"   Frames: {frame_count}, Samples: {total_samples}, Rate: {sample_rate}")
        print(f"   Accurate duration: {accurate_duration:.1f}s ({accurate_duration/60:.1f} min)")
        print(f"   Lofty duration:    {lofty_dur:.1f}s ({lofty_dur/60:.1f} min)")
        print(f"   Error ratio:       {error_ratio:.2f}x")
        print()

    print("=" * 80)
    print(f"Verified reasonable: {verified_correct}/10")

    # Also test some non-anomalous files to confirm they're already correct
    print("\n\nVerifying non-anomalous files (should match lofty):")
    print("=" * 80)

    normal_files = [r for r in results if r.get('duration_secs', 0) < 600 and r.get('duration_secs', 0) > 60][:5]
    for r in normal_files:
        path = r.get('file_path', '')
        if not os.path.exists(path):
            continue

        frame_count, total_samples, sample_rate, accurate_duration = count_mp3_frames(path)
        lofty_dur = r.get('duration_secs', 0)

        diff = abs(accurate_duration - lofty_dur)
        status = "✓" if diff < 1.0 else "✗"

        print(f"{status} {r.get('id3_artist', '')[:20]} - {r.get('id3_title', '')[:25]}")
        print(f"   Accurate: {accurate_duration:.1f}s, Lofty: {lofty_dur:.1f}s, Diff: {diff:.2f}s")

if __name__ == '__main__':
    main()
