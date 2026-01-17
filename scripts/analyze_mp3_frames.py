#!/usr/bin/env python3
"""Analyze MP3 frame structure to see where symphonia might be stopping."""

import struct

file_path = r"C:\Users\Mango Cat\Music\AC-DC\Back in Black\Disc 1 - 1 - Hells Bells.mp3"

# MP3 frame header sync word: 11111111 111xxxxx (0xFF 0xFx or 0xEx)
def is_frame_sync(b1, b2):
    return b1 == 0xFF and (b2 & 0xE0) == 0xE0

# Calculate frame size from header
# Bitrate table for MPEG1 Layer 3
BITRATES_MPEG1_L3 = [0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0]

def get_frame_size(header):
    """Calculate MP3 frame size from 4-byte header."""
    version = (header[1] >> 3) & 0x03  # 00=MPEG2.5, 10=MPEG2, 11=MPEG1
    layer = (header[1] >> 1) & 0x03    # 01=Layer3, 10=Layer2, 11=Layer1
    bitrate_idx = (header[2] >> 4) & 0x0F
    sample_rate_idx = (header[2] >> 2) & 0x03
    padding = (header[2] >> 1) & 0x01

    if version != 0x03 or layer != 0x01:  # Not MPEG1 Layer 3
        return None

    if bitrate_idx == 0 or bitrate_idx == 15:
        return None

    bitrate = BITRATES_MPEG1_L3[bitrate_idx] * 1000
    sample_rates = [44100, 48000, 32000]
    if sample_rate_idx >= len(sample_rates):
        return None
    sample_rate = sample_rates[sample_rate_idx]

    # Frame size for Layer 3: 144 * bitrate / sample_rate + padding
    frame_size = int(144 * bitrate / sample_rate) + padding
    return frame_size, bitrate

with open(file_path, 'rb') as f:
    data = f.read()

file_size = len(data)
print(f"File size: {file_size} bytes")

# Skip ID3v2 tag
pos = 0
if data[:3] == b'ID3':
    size_bytes = data[6:10]
    tag_size = (size_bytes[0] << 21) | (size_bytes[1] << 14) | (size_bytes[2] << 7) | size_bytes[3]
    pos = 10 + tag_size
    print(f"ID3v2 tag size: {pos} bytes, starting audio scan at offset {pos}")

# Scan for frame sync words
frame_count = 0
frame_positions = []
bitrates = []
last_valid_pos = pos

while pos < file_size - 4:
    if is_frame_sync(data[pos], data[pos+1]):
        header = data[pos:pos+4]
        result = get_frame_size(header)
        if result:
            frame_size, bitrate = result
            frame_count += 1
            bitrates.append(bitrate)
            if frame_count <= 5 or frame_count % 5000 == 0:
                frame_positions.append((pos, frame_size, bitrate))
            last_valid_pos = pos
            pos += frame_size
            continue

    # No valid frame at this position, try next byte
    pos += 1

# Check what's at the position where symphonia stopped (around frame 11961)
symphonia_stop_frame = 11961
print(f"\nTotal frames found: {frame_count}")
print(f"Symphonia decoded: {symphonia_stop_frame} frames")
print(f"Last valid frame position: {last_valid_pos}")

# Calculate approximate position of frame 11961
approx_stop_pos = 8901  # ID3v2 tag size
for i in range(min(symphonia_stop_frame, frame_count)):
    # Average frame size at 88kbps: 144 * 88000 / 44100 = ~288 bytes
    pass

# Show first few frames
print("\nFirst 5 frames:")
for pos, size, br in frame_positions[:5]:
    print(f"  Offset {pos}: size={size}, bitrate={br/1000}kbps")

# Show bitrate distribution
from collections import Counter
br_counts = Counter(bitrates)
print(f"\nBitrate distribution:")
for br, count in sorted(br_counts.items()):
    print(f"  {br/1000}kbps: {count} frames ({100*count/frame_count:.1f}%)")

# Calculate actual duration
total_samples = frame_count * 1152  # samples per frame for MPEG1 Layer 3
duration = total_samples / 44100
print(f"\nCalculated duration: {duration:.1f} seconds ({duration/60:.1f} min)")

# What's at position after frame 11961?
avg_frame_size = sum(144 * br / 44100 for br in bitrates) / len(bitrates)
approx_pos_11961 = 8901 + int(11961 * avg_frame_size)
print(f"\nApproximate position of frame 11961: {approx_pos_11961}")
print(f"Bytes at that position: {data[approx_pos_11961:approx_pos_11961+20].hex()}")
