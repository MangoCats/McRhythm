#!/usr/bin/env python3
"""
Apply edits from phase3-edit-plan.json to SPEC014-single_stream_design.md
"""

import json
import sys

# Load the edit plan
with open('docs/validation/spec014-edits-working.json', 'r') as f:
    edits = json.load(f)

# Track which edits to apply
print("=" * 80)
print("SPEC014 EDIT APPLICATION PLAN")
print("=" * 80)
print(f"\nTotal edits: {len(edits)}")

# Group by priority
high_edits = [e for e in edits if e['priority'] == 'HIGH']
medium_edits = [e for e in edits if e['priority'] == 'MEDIUM']
low_edits = [e for e in edits if e['priority'] == 'LOW']

print(f"\nHIGH priority: {len(high_edits)}")
print(f"MEDIUM priority: {len(medium_edits)}")
print(f"LOW priority: {len(low_edits)}")

# Print details for each edit
for priority_group, name in [(high_edits, 'HIGH'), (medium_edits, 'MEDIUM'), (low_edits, 'LOW')]:
    print(f"\n\n{name} PRIORITY EDITS")
    print("=" * 80)

    for i, edit in enumerate(priority_group, 1):
        print(f"\n{i}. {edit['edit_id']}")
        print(f"   Type: {edit['type']}")
        print(f"   Section: {edit['location']['section']}")
        print(f"   Lines: {edit['location'].get('lines', 'N/A')}")

        # Print old/new content summaries
        old_len = len(edit.get('current_content', ''))
        new_len = len(edit.get('new_content', ''))
        reduction = old_len - new_len

        print(f"   Old length: {old_len} chars")
        print(f"   New length: {new_len} chars")
        print(f"   Change: {reduction:+d} chars")

        # Show first line of old/new content
        if 'current_content' in edit:
            first_line = edit['current_content'].split('\n')[0][:80]
            print(f"   Old starts: {first_line}...")

        if 'new_content' in edit:
            first_line = edit['new_content'].split('\n')[0][:80]
            print(f"   New starts: {first_line}...")

print("\n\nEDIT APPLICATION READY")
print("=" * 80)
