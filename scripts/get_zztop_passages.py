#!/usr/bin/env python3
"""
Extract passage boundaries for ZZTopsFirstAlbum.mp3
"""
import subprocess
import json
import sys

# Run a Rust test that will output boundary detection details
test_code = """
use std::path::PathBuf;
use wkmp_ai::workflow::boundary_detector::detect_boundaries;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_path = PathBuf::from(r"C:\\Users\\Mango Cat\\Music\\Z.Z. Top\\ZZTopsFirstAlbum.mp3");

    let result = detect_boundaries(&file_path)?;

    println!("Total passages: {}", result.boundaries.len());
    println!("Sample rate: {}", result.sample_rate);
    println!("Channels: {}", result.num_channels);
    println!("");

    for (i, boundary) in result.boundaries.iter().enumerate() {
        let start_sec = boundary.start_frame as f64 / result.sample_rate as f64;
        let end_sec = boundary.end_frame as f64 / result.sample_rate as f64;
        let duration_sec = end_sec - start_sec;

        println!("Passage {}: {:.2}s - {:.2}s (duration: {:.2}s, {} frames)",
            i + 1, start_sec, end_sec, duration_sec,
            boundary.end_frame - boundary.start_frame);
    }

    Ok(())
}
"""

# Write temporary Rust file
with open("wkmp-ai/examples/show_zztop_boundaries.rs", "w") as f:
    f.write(test_code)

print("Running boundary detection on ZZTopsFirstAlbum.mp3...")
print("This may take 20-30 seconds...")
print()

# Run the example
result = subprocess.run(
    ["cargo", "run", "--release", "--example", "show_zztop_boundaries"],
    cwd="wkmp-ai",
    capture_output=True,
    text=True
)

print(result.stdout)
if result.stderr:
    print("Errors/Warnings:", file=sys.stderr)
    print(result.stderr, file=sys.stderr)

sys.exit(result.returncode)
