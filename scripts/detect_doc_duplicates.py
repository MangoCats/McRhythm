#!/usr/bin/env python3
"""
Detect duplicate content blocks in WKMP documentation.

Usage:
    python scripts/detect_doc_duplicates.py

Exit codes:
    0 - No duplicates found
    1 - Duplicates found
"""

import hashlib
import sys
import io
from pathlib import Path
from collections import defaultdict

# Fix Windows console encoding issues
if sys.platform == 'win32':
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')


def hash_paragraph(text):
    """Hash normalized paragraph text."""
    normalized = ' '.join(text.split())
    return hashlib.md5(normalized.encode()).hexdigest()


def find_duplicates(docs_dir):
    """Find duplicate paragraphs across docs."""
    paragraph_locations = defaultdict(list)
    docs_path = Path(docs_dir).resolve()

    for doc in docs_path.rglob("*.md"):
        with open(doc, 'r', encoding='utf-8') as f:
            lines = f.readlines()
            paragraph = []
            start_line = 0

            for i, line in enumerate(lines, 1):
                if line.strip() == '':
                    if paragraph:
                        text = ' '.join(paragraph)
                        if len(text) > 100:  # Only check substantial paragraphs
                            h = hash_paragraph(text)
                            paragraph_locations[h].append({
                                'file': str(doc.relative_to(docs_path)),
                                'lines': f"{start_line}-{i-1}",
                                'text': text[:100] + "..."
                            })
                        paragraph = []
                else:
                    if not paragraph:
                        start_line = i
                    paragraph.append(line.strip())

            # Handle final paragraph if file doesn't end with blank line
            if paragraph:
                text = ' '.join(paragraph)
                if len(text) > 100:
                    h = hash_paragraph(text)
                    paragraph_locations[h].append({
                        'file': str(doc.relative_to(docs_path)),
                        'lines': f"{start_line}-{len(lines)}",
                        'text': text[:100] + "..."
                    })

    # Print duplicates
    duplicates_found = False
    duplicate_count = 0

    for h, locations in paragraph_locations.items():
        if len(locations) > 1:
            duplicates_found = True
            duplicate_count += 1
            print(f"\n{'='*70}")
            print(f"Duplicate #{duplicate_count} found in {len(locations)} locations:")
            print(f"{'='*70}")
            for loc in locations:
                print(f"  {loc['file']}:{loc['lines']}")
                print(f"    Preview: {loc['text']}")
                print()

    if duplicates_found:
        print(f"\n{'='*70}")
        print(f"SUMMARY: {duplicate_count} duplicate content blocks found")
        print(f"{'='*70}")
        return 1
    else:
        print("SUCCESS: No duplicate content blocks found")
        return 0


if __name__ == '__main__':
    exit_code = find_duplicates('docs/')
    sys.exit(exit_code)
