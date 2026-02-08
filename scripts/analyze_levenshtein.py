#!/usr/bin/env python3
"""Analyze Levenshtein ratios between source and winning album/artist names in run20."""

import re
import sys
from difflib import SequenceMatcher

# Force UTF-8 output
sys.stdout.reconfigure(encoding='utf-8')

def levenshtein_ratio(s1, s2):
    """Calculate similarity ratio using SequenceMatcher (similar to Levenshtein ratio)."""
    if not s1 or not s2:
        return 0.0
    return SequenceMatcher(None, s1.lower(), s2.lower()).ratio()

def strip_ansi(text):
    """Remove ANSI escape codes from text."""
    ansi_pattern = re.compile(r'\x1b\[[0-9;]*m')
    return ansi_pattern.sub('', text)

def parse_run20(filepath):
    """Parse the run20 output file to extract album matching data."""
    with open(filepath, 'r', encoding='utf-8') as f:
        lines = [strip_ansi(line.rstrip()) for line in f]

    # Track data for ALL albums simultaneously (they process in parallel)
    album_data = {}  # keyed by album ID number like "4"
    album_phase0_state = {}  # track Phase 0 parsing state per album

    for line in lines:
        # Extract album ID from any log line
        album_marker = re.search(r'\[A(\d+)\]', line)
        if not album_marker:
            continue

        album_id = album_marker.group(1)
        line_content = re.sub(r'^.*?\[A\d+\]\s*', '', line)

        # Initialize album data if needed
        if album_id not in album_data:
            album_data[album_id] = {'album_id': album_id}
            album_phase0_state[album_id] = {'in_phase0': False, 'awaiting_alt_artist': False, 'awaiting_alt_album': False}

        data = album_data[album_id]
        state = album_phase0_state[album_id]

        # Match album header: === Album 4/200 ===
        album_header = re.search(r'===\s*Album\s+(\d+)/200\s*===', line_content)
        if album_header:
            data['album_num'] = album_header.group(1)
            continue

        # Match file path
        file_match = re.search(r'File:\s*(.+\.mp3)', line_content)
        if file_match:
            data['file'] = file_match.group(1)
            continue

        # Detect Phase 0 section
        if 'Phase 0 Reconciliation:' in line_content:
            state['in_phase0'] = True
            state['awaiting_alt_artist'] = False
            state['awaiting_alt_album'] = False
            continue

        # Parse Phase 0 content
        if state['in_phase0']:
            # Match artist in Phase 0
            artist_match = re.search(r'Artist:\s*(.+?)\s*\(source:\s*(\w+)\)', line_content)
            if artist_match:
                data['source_artist'] = artist_match.group(1).strip()
                data['artist_source'] = artist_match.group(2)
                state['awaiting_alt_artist'] = True
                state['awaiting_alt_album'] = False
                continue

            # Match album in Phase 0
            album_match = re.search(r'Album:\s*(.+?)\s*\(source:\s*(\w+)\)', line_content)
            if album_match:
                data['source_album'] = album_match.group(1).strip()
                data['album_source'] = album_match.group(2)
                state['awaiting_alt_artist'] = False
                state['awaiting_alt_album'] = True
                continue

            # Match alternate
            alt_match = re.search(r'Alternate:\s*(.+)', line_content)
            if alt_match:
                if state['awaiting_alt_artist']:
                    data['alt_artist'] = alt_match.group(1).strip()
                    state['awaiting_alt_artist'] = False
                elif state['awaiting_alt_album']:
                    data['alt_album'] = alt_match.group(1).strip()
                    state['awaiting_alt_album'] = False
                continue

            # End of Phase 0 when we see other content
            if 'Estimated track count' in line_content or 'Starting parallel' in line_content:
                state['in_phase0'] = False
                state['awaiting_alt_artist'] = False
                state['awaiting_alt_album'] = False

        # Match FINAL RESULT section
        if 'FINAL RESULT:' in line_content:
            data['has_final_result'] = True
            continue

        # Match winning artist (after FINAL RESULT)
        if data.get('has_final_result'):
            winning_artist = re.match(r'Artist:\s*(.+)', line_content)
            if winning_artist and 'winning_artist' not in data:
                data['winning_artist'] = winning_artist.group(1).strip()
                continue

            winning_album = re.match(r'Album:\s*(.+)', line_content)
            if winning_album and 'winning_album' not in data:
                data['winning_album'] = winning_album.group(1).strip()
                continue

            matched = re.search(r'Matched tracks:\s*(\d+)/(\d+)\s*\(([\d.]+)%\)', line_content)
            if matched:
                data['matched_tracks'] = int(matched.group(1))
                data['total_tracks'] = int(matched.group(2))
                data['match_pct'] = float(matched.group(3))
                continue

            confidence = re.search(r'Confidence:\s*(\w+)', line_content)
            if confidence:
                data['confidence'] = confidence.group(1)

    # Convert to list and calculate ratios
    results = []
    for aid, data in album_data.items():
        if 'winning_artist' not in data or 'winning_album' not in data:
            continue

        source_artist = data.get('source_artist', '')
        source_album = data.get('source_album', '')
        alt_artist = data.get('alt_artist', '')
        alt_album = data.get('alt_album', '')
        winning_artist = data.get('winning_artist', '')
        winning_album = data.get('winning_album', '')

        # Calculate Levenshtein ratios
        artist_ratio = levenshtein_ratio(source_artist, winning_artist)
        album_ratio = levenshtein_ratio(source_album, winning_album)
        alt_artist_ratio = levenshtein_ratio(alt_artist, winning_artist) if alt_artist else None
        alt_album_ratio = levenshtein_ratio(alt_album, winning_album) if alt_album else None

        results.append({
            'album_id': data.get('album_id', ''),
            'album_num': data.get('album_num', ''),
            'file': data.get('file', ''),
            'source_artist': source_artist,
            'alt_artist': alt_artist,
            'source_album': source_album,
            'alt_album': alt_album,
            'winning_artist': winning_artist,
            'winning_album': winning_album,
            'artist_ratio': artist_ratio,
            'album_ratio': album_ratio,
            'alt_artist_ratio': alt_artist_ratio,
            'alt_album_ratio': alt_album_ratio,
            'match_pct': data.get('match_pct', 0),
            'confidence': data.get('confidence', ''),
            'matched_tracks': data.get('matched_tracks', 0),
            'total_tracks': data.get('total_tracks', 0),
        })

    return results

def classify_match(result):
    """Classify whether this is a proper match or mismatch based on various criteria."""
    # Get best ratios considering alternates
    best_artist_ratio = result['artist_ratio']
    best_album_ratio = result['album_ratio']

    if result['alt_artist_ratio'] and result['alt_artist_ratio'] > best_artist_ratio:
        best_artist_ratio = result['alt_artist_ratio']
    if result['alt_album_ratio'] and result['alt_album_ratio'] > best_album_ratio:
        best_album_ratio = result['alt_album_ratio']

    result['best_artist_ratio'] = best_artist_ratio
    result['best_album_ratio'] = best_album_ratio

    # Consider it a proper match if both artist AND album have >0.7 similarity
    if best_artist_ratio > 0.7 and best_album_ratio > 0.7:
        return 'proper_match'

    # If one is very high and other is medium, still proper
    if (best_artist_ratio > 0.9 and best_album_ratio > 0.4) or (best_album_ratio > 0.9 and best_artist_ratio > 0.4):
        return 'proper_match'

    # Mismatch if both are very low
    if best_artist_ratio < 0.3 and best_album_ratio < 0.3:
        return 'mismatch'

    return 'uncertain'

def main():
    results = parse_run20(r"c:\Users\Mango Cat\Dev\McRhythm\album_matcher_output_run20.txt")

    print(f"Total albums analyzed: {len(results)}\n")

    # Classify all results
    for r in results:
        r['classification'] = classify_match(r)

    proper_matches = [r for r in results if r['classification'] == 'proper_match']
    mismatches = [r for r in results if r['classification'] == 'mismatch']
    uncertain = [r for r in results if r['classification'] == 'uncertain']

    print(f"Proper matches: {len(proper_matches)}")
    print(f"Potential mismatches: {len(mismatches)}")
    print(f"Uncertain: {len(uncertain)}\n")

    print("=" * 100)
    print("PROPER MATCHES - Levenshtein Ratios")
    print("=" * 100)

    if proper_matches:
        artist_ratios = [r['best_artist_ratio'] for r in proper_matches]
        album_ratios = [r['best_album_ratio'] for r in proper_matches]
        print(f"Artist ratio - min: {min(artist_ratios):.3f}, max: {max(artist_ratios):.3f}, avg: {sum(artist_ratios)/len(artist_ratios):.3f}")
        print(f"Album ratio  - min: {min(album_ratios):.3f}, max: {max(album_ratios):.3f}, avg: {sum(album_ratios)/len(album_ratios):.3f}")
        print(f"\nSample proper matches (first 10):")
        for r in sorted(proper_matches, key=lambda x: int(x['album_id']))[:10]:
            print(f"  [{r['album_id']:>3}] Source: {r['source_artist']} - {r['source_album']}")
            print(f"         Winner: {r['winning_artist']} - {r['winning_album']}")
            print(f"         Artist ratio: {r['best_artist_ratio']:.3f}, Album ratio: {r['best_album_ratio']:.3f}, Match: {r['match_pct']}%")
            if r['alt_artist']:
                print(f"         (Alt artist: {r['alt_artist']})")
            if r['alt_album']:
                print(f"         (Alt album: {r['alt_album']})")

    print("\n" + "=" * 100)
    print("POTENTIAL MISMATCHES - Levenshtein Ratios (low artist AND album similarity)")
    print("=" * 100)

    if mismatches:
        artist_ratios = [r['best_artist_ratio'] for r in mismatches]
        album_ratios = [r['best_album_ratio'] for r in mismatches]
        print(f"Artist ratio - min: {min(artist_ratios):.3f}, max: {max(artist_ratios):.3f}, avg: {sum(artist_ratios)/len(artist_ratios):.3f}")
        print(f"Album ratio  - min: {min(album_ratios):.3f}, max: {max(album_ratios):.3f}, avg: {sum(album_ratios)/len(album_ratios):.3f}")
        print(f"\nAll potential mismatches:")
        for r in sorted(mismatches, key=lambda x: int(x['album_id'])):
            print(f"  [{r['album_id']:>3}] Source: {r['source_artist']} - {r['source_album']}")
            print(f"         Winner: {r['winning_artist']} - {r['winning_album']}")
            print(f"         Artist: {r['best_artist_ratio']:.3f}, Album: {r['best_album_ratio']:.3f}, Match: {r['match_pct']}%, Conf: {r['confidence']}")
            if r['alt_artist']:
                print(f"         (Alt artist: {r['alt_artist']} -> ratio: {r['alt_artist_ratio']:.3f})")
            if r['alt_album']:
                print(f"         (Alt album: {r['alt_album']} -> ratio: {r['alt_album_ratio']:.3f})")
    else:
        print("No clear mismatches found (all have at least moderate similarity)")

    print("\n" + "=" * 100)
    print("UNCERTAIN CASES - Mixed similarity scores")
    print("=" * 100)

    if uncertain:
        artist_ratios = [r['best_artist_ratio'] for r in uncertain]
        album_ratios = [r['best_album_ratio'] for r in uncertain]
        print(f"Artist ratio - min: {min(artist_ratios):.3f}, max: {max(artist_ratios):.3f}, avg: {sum(artist_ratios)/len(artist_ratios):.3f}")
        print(f"Album ratio  - min: {min(album_ratios):.3f}, max: {max(album_ratios):.3f}, avg: {sum(album_ratios)/len(album_ratios):.3f}")
        print(f"\nAll uncertain cases:")
        for r in sorted(uncertain, key=lambda x: int(x['album_id'])):
            print(f"  [{r['album_id']:>3}] Source: {r['source_artist']} - {r['source_album']}")
            print(f"         Winner: {r['winning_artist']} - {r['winning_album']}")
            print(f"         Artist: {r['best_artist_ratio']:.3f}, Album: {r['best_album_ratio']:.3f}, Match: {r['match_pct']}%, Conf: {r['confidence']}")
            if r['alt_artist']:
                print(f"         (Alt artist: {r['alt_artist']} -> ratio: {r['alt_artist_ratio']:.3f})")
            if r['alt_album']:
                print(f"         (Alt album: {r['alt_album']} -> ratio: {r['alt_album_ratio']:.3f})")

    # Statistical comparison
    print("\n" + "=" * 100)
    print("STATISTICAL COMPARISON")
    print("=" * 100)

    if proper_matches and (mismatches or uncertain):
        proper_artist_avg = sum(r['best_artist_ratio'] for r in proper_matches) / len(proper_matches)
        proper_album_avg = sum(r['best_album_ratio'] for r in proper_matches) / len(proper_matches)

        non_proper = mismatches + uncertain
        non_proper_artist_avg = sum(r['best_artist_ratio'] for r in non_proper) / len(non_proper)
        non_proper_album_avg = sum(r['best_album_ratio'] for r in non_proper) / len(non_proper)

        print(f"\nProper matches ({len(proper_matches)} albums):")
        print(f"  Avg artist ratio: {proper_artist_avg:.3f}")
        print(f"  Avg album ratio:  {proper_album_avg:.3f}")

        print(f"\nNon-proper (mismatches + uncertain, {len(non_proper)} albums):")
        print(f"  Avg artist ratio: {non_proper_artist_avg:.3f}")
        print(f"  Avg album ratio:  {non_proper_album_avg:.3f}")

        print(f"\nDifference (proper - non-proper):")
        print(f"  Artist ratio diff: {proper_artist_avg - non_proper_artist_avg:+.3f}")
        print(f"  Album ratio diff:  {proper_album_avg - non_proper_album_avg:+.3f}")

    # Detailed output for manual review - sorted by combined ratio (lowest first to spot mismatches)
    print("\n" + "=" * 100)
    print("ALL RESULTS SORTED BY COMBINED RATIO (low to high) - FOR MANUAL REVIEW")
    print("=" * 100)

    sorted_results = sorted(results, key=lambda x: x['best_artist_ratio'] + x['best_album_ratio'])
    for r in sorted_results[:50]:  # Show first 50 (lowest ratios)
        combined = r['best_artist_ratio'] + r['best_album_ratio']
        marker = "!!!" if r['classification'] == 'mismatch' else ("?" if r['classification'] == 'uncertain' else "")
        print(f"[{r['album_id']:>3}] {marker:3} Combined: {combined:.3f} | Artist: {r['best_artist_ratio']:.3f} | Album: {r['best_album_ratio']:.3f} | Match: {r['match_pct']:>5.1f}%")
        src_artist = (r['source_artist'][:45] + '..') if len(r['source_artist']) > 45 else r['source_artist']
        src_album = (r['source_album'][:45] + '..') if len(r['source_album']) > 45 else r['source_album']
        win_artist = (r['winning_artist'][:45] + '..') if len(r['winning_artist']) > 45 else r['winning_artist']
        win_album = (r['winning_album'][:45] + '..') if len(r['winning_album']) > 45 else r['winning_album']
        print(f"       Source: {src_artist} - {src_album}")
        print(f"       Winner: {win_artist} - {win_album}")

if __name__ == '__main__':
    main()
