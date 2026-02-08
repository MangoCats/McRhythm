"""
Test Phase 1 MusicBrainz Extensions Against 9 Failed Albums

Runs album matching on the 9 albums that previously failed to test
whether Phase 1 extensions (artist normalization, similarity bonuses,
additional search strategies) improve match rates.
"""

import subprocess
import json
import sys
import os
from pathlib import Path

# Failed albums to test (from failed_albums_output.txt)
FAILED_ALBUMS = [
    "Brubeck, Dave/TheBestOfTheDaveBrubeckQuartet.mp3",
    "Go Gos, The/BeautyAndTheBeat.mp3",
    "Hooverphonic/LiveAtTheAncienneBelgique.mp3",
    "Mayall, John/AHardRoad.mp3",
    "Phildel/Ritual.mp3",  # Actually Delerium - Ritual
    "Police/RegattaDeBlanc.mp3",
    "Santana/InvitationToIllumination.mp3",
    "Score, The/Atlas.mp3",
    "Various/TheGreatestShowman.mp3",
]

EXPECTED_MATCHES = {
    "Brubeck, Dave/TheBestOfTheDaveBrubeckQuartet.mp3": {
        "artist": "The Dave Brubeck Quartet",
        "album": "The Best Of The Dave Brubeck Quartet",
        "issue": "Artist suffix: 'Dave Brubeck' vs 'Dave Brubeck Quartet'",
    },
    "Go Gos, The/BeautyAndTheBeat.mp3": {
        "artist": "The Go-Go's",
        "album": "Beauty and the Beat",
        "issue": "Prefix + punctuation: 'The Go Gos' vs 'Go-Go's'",
    },
    "Hooverphonic/LiveAtTheAncienneBelgique.mp3": {
        "artist": "Hooverphonic",
        "album": "Live at the Ancienne Belgique",
        "issue": "Unknown - may not be artist name related",
    },
    "Mayall, John/AHardRoad.mp3": {
        "artist": "John Mayall & the Bluesbreakers",
        "album": "A Hard Road",
        "issue": "Artist suffix: 'John Mayall' vs 'John Mayall & the Bluesbreakers'",
    },
    "Phildel/Ritual.mp3": {
        "artist": "Delerium",
        "album": "Ritual",
        "issue": "Wrong artist folder - file mislabeled (not fixable by Phase 1)",
    },
    "Police/RegattaDeBlanc.mp3": {
        "artist": "The Police",
        "album": "Reggatta de Blanc",
        "issue": "Prefix: 'Police' vs 'The Police'",
    },
    "Santana/InvitationToIllumination.mp3": {
        "artist": "Carlos Santana",
        "album": "Invitation to Illumination",
        "issue": "Artist prefix: 'Santana' vs 'Carlos Santana'",
    },
    "Score, The/Atlas.mp3": {
        "artist": "The Score",
        "album": "Atlas",
        "issue": "Prefix: 'Score The' vs 'The Score'",
    },
    "Various/TheGreatestShowman.mp3": {
        "artist": "Various Artists",
        "album": "The Greatest Showman",
        "issue": "Various artists handling",
    },
}

def find_music_root():
    """Find the Music folder containing the test files"""
    # Try common locations
    candidates = [
        Path.home() / "Music",
        Path("C:/Users/Mango Cat/Music"),
        Path("D:/Music"),
    ]

    for candidate in candidates:
        if candidate.exists():
            # Check if any of our test files exist
            test_file = candidate / FAILED_ALBUMS[0]
            if test_file.exists():
                return candidate

    return None

def build_album_matcher():
    """Build the album matcher example"""
    print("Building album matcher...")
    result = subprocess.run(
        ["cargo", "build", "--example", "am30", "--release"],
        cwd="wkmp-ai",
        capture_output=True,
        text=True
    )

    if result.returncode != 0:
        print("Build failed:")
        print(result.stderr)
        return False

    print("Build successful\n")
    return True

def test_single_album(music_root, relative_path):
    """Test a single album file"""
    full_path = music_root / relative_path

    if not full_path.exists():
        return {
            "file": relative_path,
            "status": "FILE_NOT_FOUND",
            "error": f"File not found: {full_path}"
        }

    print(f"Testing: {relative_path}")

    # Run album matcher on this file
    result = subprocess.run(
        [
            "cargo", "run", "--example", "am30", "--release", "--",
            "--file", str(full_path),
            "--db-path", "wkmp.db"
        ],
        cwd="wkmp-ai",
        capture_output=True,
        text=True,
        timeout=120
    )

    # Parse output for match result
    output = result.stdout + result.stderr

    # Look for match indicators
    matched = False
    mbid = None
    confidence = None
    match_percentage = None

    for line in output.split('\n'):
        if "Matched to release" in line or "Selected edition:" in line:
            matched = True
        if "MBID:" in line or "mbid:" in line:
            parts = line.split(":")
            if len(parts) > 1:
                mbid = parts[1].strip()
        if "confidence" in line.lower():
            # Try to extract confidence percentage
            try:
                for word in line.split():
                    if '%' in word:
                        confidence = float(word.replace('%', ''))
            except:
                pass
        if "match percentage" in line.lower() or "track match" in line.lower():
            try:
                for word in line.split():
                    if '%' in word:
                        match_percentage = float(word.replace('%', ''))
            except:
                pass

    return {
        "file": relative_path,
        "status": "MATCHED" if matched else "FAILED",
        "mbid": mbid,
        "confidence": confidence,
        "match_percentage": match_percentage,
        "output_sample": output[:500] if len(output) > 500 else output
    }

def main():
    print("=" * 80)
    print("Phase 1 Extensions Test - 9 Failed Albums")
    print("=" * 80)
    print()

    # Find music root
    music_root = find_music_root()
    if not music_root:
        print("ERROR: Could not find Music folder with test files")
        print("Please specify the root music folder path")
        return 1

    print(f"Music root: {music_root}")
    print()

    # Build album matcher
    if not build_album_matcher():
        return 1

    # Test each failed album
    results = []
    for relative_path in FAILED_ALBUMS:
        try:
            result = test_single_album(music_root, relative_path)
            results.append(result)

            status_symbol = "✓" if result["status"] == "MATCHED" else "✗"
            print(f"  {status_symbol} {result['status']}")

            if result.get("match_percentage"):
                print(f"    Match: {result['match_percentage']:.1f}%")

            print()

        except Exception as e:
            print(f"  ERROR: {e}")
            print()
            results.append({
                "file": relative_path,
                "status": "ERROR",
                "error": str(e)
            })

    # Summary
    print("=" * 80)
    print("SUMMARY")
    print("=" * 80)
    print()

    matched_count = sum(1 for r in results if r["status"] == "MATCHED")
    failed_count = sum(1 for r in results if r["status"] == "FAILED")
    error_count = sum(1 for r in results if r["status"] in ["ERROR", "FILE_NOT_FOUND"])

    print(f"Total albums tested: {len(results)}")
    print(f"Matched: {matched_count}")
    print(f"Failed: {failed_count}")
    print(f"Errors: {error_count}")
    print()

    if matched_count > 0:
        print("Newly Matched Albums:")
        for result in results:
            if result["status"] == "MATCHED":
                file_path = result["file"]
                expected = EXPECTED_MATCHES.get(file_path, {})
                print(f"  ✓ {file_path}")
                print(f"    Issue addressed: {expected.get('issue', 'Unknown')}")
                if result.get("match_percentage"):
                    print(f"    Match quality: {result['match_percentage']:.1f}%")
        print()

    if failed_count > 0:
        print("Still Failing:")
        for result in results:
            if result["status"] == "FAILED":
                file_path = result["file"]
                expected = EXPECTED_MATCHES.get(file_path, {})
                print(f"  ✗ {file_path}")
                print(f"    Known issue: {expected.get('issue', 'Unknown')}")
        print()

    # Calculate improvement rate
    baseline_failures = 9
    remaining_failures = failed_count
    improvement = baseline_failures - remaining_failures

    print(f"Phase 1 Impact: {improvement}/{baseline_failures} albums fixed ({improvement * 100 / baseline_failures:.1f}% improvement)")
    print()

    # Save detailed results
    output_file = "phase1_test_results.json"
    with open(output_file, 'w') as f:
        json.dump({
            "summary": {
                "total": len(results),
                "matched": matched_count,
                "failed": failed_count,
                "errors": error_count,
                "improvement_rate": f"{improvement}/{baseline_failures}"
            },
            "results": results
        }, f, indent=2)

    print(f"Detailed results saved to: {output_file}")

    return 0 if error_count == 0 else 1

if __name__ == "__main__":
    sys.exit(main())
