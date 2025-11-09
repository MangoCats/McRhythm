#!/usr/bin/env python3
"""
Generate minimal test audio fixtures for wkmp-ai tests.

Creates:
- sine_440hz_5s.wav: 5-second 440 Hz sine wave (A4 note)
- silence_3s.wav: 3 seconds of silence
- chirp_2s.wav: 2-second frequency sweep (100-1000 Hz)

Requirements: numpy, scipy
"""

import numpy as np
from scipy.io import wavfile
import os

# Test audio parameters
SAMPLE_RATE = 44100  # CD quality
BIT_DEPTH = np.int16
MAX_AMPLITUDE = 32767  # 16-bit max

def generate_sine_wave(frequency_hz, duration_seconds, sample_rate=SAMPLE_RATE):
    """Generate a sine wave at the specified frequency."""
    t = np.linspace(0, duration_seconds, int(sample_rate * duration_seconds), False)
    signal = np.sin(2 * np.pi * frequency_hz * t)
    # Apply fade in/out to avoid clicks
    fade_samples = int(sample_rate * 0.01)  # 10ms fade
    fade_in = np.linspace(0, 1, fade_samples)
    fade_out = np.linspace(1, 0, fade_samples)
    signal[:fade_samples] *= fade_in
    signal[-fade_samples:] *= fade_out
    return (signal * MAX_AMPLITUDE * 0.8).astype(BIT_DEPTH)

def generate_silence(duration_seconds, sample_rate=SAMPLE_RATE):
    """Generate silence."""
    samples = int(sample_rate * duration_seconds)
    return np.zeros(samples, dtype=BIT_DEPTH)

def generate_chirp(duration_seconds, f0=100, f1=1000, sample_rate=SAMPLE_RATE):
    """Generate a frequency sweep (chirp)."""
    t = np.linspace(0, duration_seconds, int(sample_rate * duration_seconds), False)
    # Logarithmic chirp
    k = (f1 / f0) ** (1.0 / duration_seconds)
    signal = np.sin(2 * np.pi * f0 * (k ** t - 1) / np.log(k))
    # Apply fade in/out
    fade_samples = int(sample_rate * 0.01)
    fade_in = np.linspace(0, 1, fade_samples)
    fade_out = np.linspace(1, 0, fade_samples)
    signal[:fade_samples] *= fade_in
    signal[-fade_samples:] *= fade_out
    return (signal * MAX_AMPLITUDE * 0.8).astype(BIT_DEPTH)

def main():
    """Generate all test fixtures."""
    script_dir = os.path.dirname(os.path.abspath(__file__))

    print("Generating test audio fixtures...")

    # 1. 440 Hz sine wave (A4 note) - 5 seconds
    print("  Creating sine_440hz_5s.wav (440 Hz, 5 seconds)...")
    sine_wave = generate_sine_wave(440, 5.0)
    wavfile.write(os.path.join(script_dir, "sine_440hz_5s.wav"), SAMPLE_RATE, sine_wave)

    # 2. Silence - 3 seconds
    print("  Creating silence_3s.wav (silence, 3 seconds)...")
    silence = generate_silence(3.0)
    wavfile.write(os.path.join(script_dir, "silence_3s.wav"), SAMPLE_RATE, silence)

    # 3. Frequency sweep (chirp) - 2 seconds
    print("  Creating chirp_2s.wav (100-1000 Hz sweep, 2 seconds)...")
    chirp = generate_chirp(2.0, 100, 1000)
    wavfile.write(os.path.join(script_dir, "chirp_2s.wav"), SAMPLE_RATE, chirp)

    print("✓ Test fixtures generated successfully!")
    print(f"  Location: {script_dir}")
    print("  Files: sine_440hz_5s.wav, silence_3s.wav, chirp_2s.wav")

if __name__ == "__main__":
    main()
