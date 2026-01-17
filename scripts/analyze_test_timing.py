#!/usr/bin/env python3
"""
Analyze timing bottlenecks from full_library_import_test.log files.

Usage:
    python analyze_test_timing.py full_library_import_20251217_*.log
"""

import re
import sys
from collections import defaultdict
from dataclasses import dataclass
from typing import Dict, List, Optional


@dataclass
class TimingEvent:
    timestamp: float
    event_type: str
    passage_index: Optional[int]
    extractor: Optional[str]
    status: Optional[str]
    file_path: Optional[str]


def parse_log_file(log_path: str) -> List[TimingEvent]:
    """Parse log file and extract timing events."""
    events = []

    with open(log_path, 'r', encoding='utf-8') as f:
        for line in f:
            # Match timestamp: [  123.456s]
            ts_match = re.match(r'\[\s*(\d+\.\d+)s\] (.+)', line)
            if not ts_match:
                continue

            timestamp = float(ts_match.group(1))
            content = ts_match.group(2)

            # Parse event type
            if content.startswith('FILE_START'):
                match = re.match(r'FILE_START \[(\d+)/(\d+)\] (.+)', content)
                if match:
                    events.append(TimingEvent(
                        timestamp=timestamp,
                        event_type='FILE_START',
                        passage_index=None,
                        extractor=None,
                        status=None,
                        file_path=match.group(3)
                    ))

            elif content.startswith('FILE_COMPLETE'):
                events.append(TimingEvent(
                    timestamp=timestamp,
                    event_type='FILE_COMPLETE',
                    passage_index=None,
                    extractor=None,
                    status=None,
                    file_path=None
                ))

            elif content.startswith('EVENT'):
                # Parse EVENT lines
                if 'PassageStarted' in content:
                    match = re.search(r'passage #(\d+)', content)
                    events.append(TimingEvent(
                        timestamp=timestamp,
                        event_type='PassageStarted',
                        passage_index=int(match.group(1)) if match else None,
                        extractor=None,
                        status=None,
                        file_path=None
                    ))

                elif 'PassageCompleted' in content:
                    match = re.search(r'passage #(\d+)', content)
                    events.append(TimingEvent(
                        timestamp=timestamp,
                        event_type='PassageCompleted',
                        passage_index=int(match.group(1)) if match else None,
                        extractor=None,
                        status=None,
                        file_path=None
                    ))

                elif 'ExtractionProgress' in content:
                    # Extract passage_index, extractor, status
                    match = re.search(r'passage_index: (\d+), extractor: "([^"]+)", status: "([^"]+)"', content)
                    if match:
                        events.append(TimingEvent(
                            timestamp=timestamp,
                            event_type='ExtractionProgress',
                            passage_index=int(match.group(1)),
                            extractor=match.group(2),
                            status=match.group(3),
                            file_path=None
                        ))

                elif 'FusionStarted' in content:
                    match = re.search(r'passage_index: (\d+)', content)
                    events.append(TimingEvent(
                        timestamp=timestamp,
                        event_type='FusionStarted',
                        passage_index=int(match.group(1)) if match else None,
                        extractor=None,
                        status=None,
                        file_path=None
                    ))

                elif 'ValidationStarted' in content:
                    match = re.search(r'passage_index: (\d+)', content)
                    events.append(TimingEvent(
                        timestamp=timestamp,
                        event_type='ValidationStarted',
                        passage_index=int(match.group(1)) if match else None,
                        extractor=None,
                        status=None,
                        file_path=None
                    ))

                elif 'AlbumMatchingStarted' in content:
                    events.append(TimingEvent(
                        timestamp=timestamp,
                        event_type='AlbumMatchingStarted',
                        passage_index=None,
                        extractor=None,
                        status=None,
                        file_path=None
                    ))

                elif 'AlbumMatchingCompleted' in content or 'AlbumMatchingFailed' in content:
                    events.append(TimingEvent(
                        timestamp=timestamp,
                        event_type='AlbumMatchingComplete',
                        passage_index=None,
                        extractor=None,
                        status=None,
                        file_path=None
                    ))

    return events


def analyze_timing(events: List[TimingEvent]):
    """Analyze timing bottlenecks from events."""

    # Track time spent in each phase
    extractor_times = defaultdict(list)
    fusion_times = []
    validation_times = []
    album_matching_times = []
    file_times = []

    # Track current state
    passage_start_times = {}
    extractor_start_times = {}
    fusion_start_time = None
    validation_start_time = None
    album_matching_start_time = None
    file_start_time = None

    for event in events:
        if event.event_type == 'FILE_START':
            file_start_time = event.timestamp

        elif event.event_type == 'FILE_COMPLETE' and file_start_time is not None:
            file_times.append(event.timestamp - file_start_time)
            file_start_time = None

        elif event.event_type == 'PassageStarted':
            passage_start_times[event.passage_index] = event.timestamp

        elif event.event_type == 'ExtractionProgress':
            key = (event.passage_index, event.extractor)

            if event.status == 'running':
                extractor_start_times[key] = event.timestamp

            elif event.status in ('completed', 'failed') and key in extractor_start_times:
                duration = event.timestamp - extractor_start_times[key]
                extractor_times[event.extractor].append(duration)
                del extractor_start_times[key]

        elif event.event_type == 'FusionStarted':
            fusion_start_time = event.timestamp

        elif event.event_type == 'ValidationStarted':
            if fusion_start_time is not None:
                fusion_times.append(event.timestamp - fusion_start_time)
                fusion_start_time = None
            validation_start_time = event.timestamp

        elif event.event_type == 'PassageCompleted':
            if validation_start_time is not None:
                validation_times.append(event.timestamp - validation_start_time)
                validation_start_time = None

        elif event.event_type == 'AlbumMatchingStarted':
            album_matching_start_time = event.timestamp

        elif event.event_type == 'AlbumMatchingComplete' and album_matching_start_time is not None:
            album_matching_times.append(event.timestamp - album_matching_start_time)
            album_matching_start_time = None

    # Print analysis
    print("\n" + "=" * 80)
    print("TIMING BOTTLENECK ANALYSIS")
    print("=" * 80)

    if file_times:
        print(f"\nPer-File Processing:")
        print(f"  Average:  {sum(file_times) / len(file_times):.2f}s")
        print(f"  Min:      {min(file_times):.2f}s")
        print(f"  Max:      {max(file_times):.2f}s")
        print(f"  Total:    {sum(file_times):.2f}s")

    if extractor_times:
        print(f"\nExtractor Times (per passage):")
        extractor_totals = []
        for extractor, times in sorted(extractor_times.items()):
            if times:
                avg = sum(times) / len(times)
                total = sum(times)
                count = len(times)
                extractor_totals.append((extractor, total, avg, count))
                print(f"  {extractor:20s}: avg={avg:6.3f}s  total={total:7.2f}s  count={count:4d}")

        print(f"\nExtractor Time Breakdown (sorted by total time):")
        extractor_totals.sort(key=lambda x: x[1], reverse=True)
        total_extractor_time = sum(t[1] for t in extractor_totals)
        for extractor, total, avg, count in extractor_totals:
            pct = 100.0 * total / total_extractor_time if total_extractor_time > 0 else 0
            print(f"  {extractor:20s}: {total:7.2f}s ({pct:5.1f}%)")

    if fusion_times:
        print(f"\nFusion Phase:")
        print(f"  Average:  {sum(fusion_times) / len(fusion_times):.3f}s")
        print(f"  Total:    {sum(fusion_times):.2f}s")

    if validation_times:
        print(f"\nValidation Phase:")
        print(f"  Average:  {sum(validation_times) / len(validation_times):.3f}s")
        print(f"  Total:    {sum(validation_times):.2f}s")

    if album_matching_times:
        print(f"\nAlbum Matching (per file):")
        print(f"  Average:  {sum(album_matching_times) / len(album_matching_times):.2f}s")
        print(f"  Min:      {min(album_matching_times):.2f}s")
        print(f"  Max:      {max(album_matching_times):.2f}s")
        print(f"  Total:    {sum(album_matching_times):.2f}s")

    print("\n" + "=" * 80)


def main():
    if len(sys.argv) < 2:
        print("Usage: python analyze_test_timing.py <log_file>")
        sys.exit(1)

    log_path = sys.argv[1]
    print(f"Analyzing: {log_path}")

    events = parse_log_file(log_path)
    print(f"Parsed {len(events)} events")

    analyze_timing(events)


if __name__ == '__main__':
    main()
