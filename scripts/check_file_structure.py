#!/usr/bin/env python3
"""Check MP3 file structure for tags."""

import sys

file_path = r"C:\Users\Mango Cat\Music\AC-DC\Back in Black\Disc 1 - 1 - Hells Bells.mp3"

with open(file_path, 'rb') as f:
    # Read first 10 bytes
    header = f.read(10)
    print(f"File size: {f.seek(0, 2)} bytes")
    f.seek(0)

    # Check ID3v2 header
    if header[:3] == b'ID3':
        version = header[3:5]
        flags = header[5]
        size_bytes = header[6:10]
        # Size is syncsafe integer
        size = (size_bytes[0] << 21) | (size_bytes[1] << 14) | (size_bytes[2] << 7) | size_bytes[3]
        print(f"ID3v2.{version[0]}.{version[1]} tag found, size: {size + 10} bytes (including header)")

    # Check for ID3v1 at end (last 128 bytes)
    f.seek(-128, 2)
    id3v1 = f.read(3)
    if id3v1 == b'TAG':
        print("ID3v1 tag found at end of file")
    else:
        print(f"No ID3v1 tag (last 128 bytes start with: {id3v1})")

    # Check for APE tag (look for APETAGEX at end)
    f.seek(-32, 2)
    ape_check = f.read(8)
    if ape_check == b'APETAGEX':
        print("APE tag found at end of file")
    else:
        print(f"No APE tag (last 32 bytes start with: {ape_check})")

    # Calculate expected audio data size
    f.seek(0, 2)
    file_size = f.tell()

    # Expected audio based on FFprobe: 527.77s @ 88kbps
    expected_audio = int(527.77 * 88000 / 8)
    print(f"\nExpected audio data (527.77s @ 88kbps): {expected_audio} bytes")
    print(f"File size: {file_size} bytes")

    # Symphonia decode: 312.5s @ ~? kbps
    symphonia_audio = int(312.5 * 88000 / 8)
    print(f"Symphonia audio estimate (312.5s @ 88kbps): {symphonia_audio} bytes")

    # What bitrate would give 312.5s for the file?
    # file_size = duration * bitrate / 8
    # bitrate = file_size * 8 / duration
    symphonia_bitrate = file_size * 8 / 312.5 / 1000
    ffprobe_bitrate = file_size * 8 / 527.77 / 1000
    print(f"\nImplied bitrate if duration=312.5s: {symphonia_bitrate:.0f} kbps")
    print(f"Implied bitrate if duration=527.77s: {ffprobe_bitrate:.0f} kbps")
