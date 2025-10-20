#!/usr/bin/env python3
"""
WKMP Documentation Link Validator
Validates all cross-references and links in markdown documentation
"""

import re
import json
from pathlib import Path
from typing import Dict, List, Set, Tuple
from collections import defaultdict

class LinkValidator:
    def __init__(self, docs_dir: Path):
        self.docs_dir = docs_dir
        self.all_files = list(docs_dir.glob("*.md"))
        self.file_names = {f.name for f in self.all_files}

        # Results tracking
        self.total_links = 0
        self.valid_links = 0
        self.broken_links = []
        self.invalid_requirement_ids = []
        self.missing_anchors = []
        self.orphaned_references = []
        self.documents_with_issues = defaultdict(list)
        self.critical_issues = []
        self.warnings = []

        # Cache for requirement IDs and anchors
        self.requirement_ids = {}  # file -> set of req IDs
        self.anchors = {}  # file -> set of anchors

    def extract_anchors_from_file(self, file_path: Path) -> Set[str]:
        """Extract all valid anchor targets from a file"""
        anchors = set()
        content = file_path.read_text(encoding='utf-8')

        # Extract headers (# Header -> #header)
        for match in re.finditer(r'^#+\s+(.+)$', content, re.MULTILINE):
            header = match.group(1)
            # Convert to anchor format (lowercase, spaces to hyphens, remove special chars)
            anchor = re.sub(r'[^\w\s-]', '', header.lower())
            anchor = re.sub(r'\s+', '-', anchor.strip())
            anchors.add(anchor)

        # Extract requirement IDs (these become anchors)
        for match in re.finditer(r'\[([A-Z]+-[A-Z]+-\d+)\]', content):
            req_id = match.group(1)
            anchors.add(req_id.lower())
            anchors.add(req_id)  # Try both cases

        return anchors

    def extract_requirement_ids_from_file(self, file_path: Path) -> Set[str]:
        """Extract all requirement IDs defined in a file"""
        req_ids = set()
        content = file_path.read_text(encoding='utf-8')

        # Find requirement ID definitions (typically in headers or as standalone)
        for match in re.finditer(r'\[([A-Z]+-[A-Z]+-\d+)\]', content):
            req_id = match.group(1)
            req_ids.add(req_id)

        return req_ids

    def cache_all_anchors_and_ids(self):
        """Pre-cache all anchors and requirement IDs from all files"""
        for file_path in self.all_files:
            self.anchors[file_path.name] = self.extract_anchors_from_file(file_path)
            self.requirement_ids[file_path.name] = self.extract_requirement_ids_from_file(file_path)

    def validate_link(self, source_file: Path, link_text: str, link_target: str) -> Tuple[bool, str]:
        """
        Validate a single link
        Returns (is_valid, error_message)
        """
        # Parse the target
        if '#' in link_target:
            file_part, anchor_part = link_target.split('#', 1)
        else:
            file_part = link_target
            anchor_part = None

        # If no file part, it's an anchor in the current file
        if not file_part:
            target_file = source_file.name
        else:
            target_file = file_part

        # Check if target file exists
        if target_file not in self.file_names:
            return False, f"Target file not found: {target_file}"

        # Check anchor if specified
        if anchor_part:
            if target_file not in self.anchors:
                return False, f"No anchors cached for {target_file}"

            if anchor_part not in self.anchors[target_file]:
                return False, f"Anchor #{anchor_part} not found in {target_file}"

        return True, ""

    def extract_links_from_file(self, file_path: Path) -> List[Tuple[str, str]]:
        """
        Extract all markdown links from a file
        Returns list of (link_text, link_target) tuples
        """
        links = []
        content = file_path.read_text(encoding='utf-8')

        # Match [text](target) format
        for match in re.finditer(r'\[([^\]]+)\]\(([^)]+)\)', content):
            link_text = match.group(1)
            link_target = match.group(2)

            # Skip external URLs
            if link_target.startswith('http://') or link_target.startswith('https://'):
                continue

            links.append((link_text, link_target))

        return links

    def find_requirement_references(self, file_path: Path) -> List[str]:
        """Find all requirement ID references [REQ-XXX-NNN] in a file"""
        refs = []
        content = file_path.read_text(encoding='utf-8')

        for match in re.finditer(r'\[([A-Z]+-[A-Z]+-\d+)\]', content):
            req_id = match.group(1)
            refs.append(req_id)

        return refs

    def validate_requirement_reference(self, req_id: str) -> Tuple[bool, str]:
        """
        Validate that a requirement ID reference exists in the appropriate file
        """
        # Determine which file should contain this requirement
        prefix = req_id.split('-')[0]

        # Map prefixes to expected files
        file_map = {
            'REQ': 'REQ001-requirements.md',
            'GOV': ['GOV001-document_hierarchy.md', 'GOV002-requirements_enumeration.md', 'GOV003-filename_convention.md'],
            'ARCH': 'SPEC001-architecture.md',
            'XFD': 'SPEC002-crossfade.md',
            'FLV': 'SPEC003-musical_flavor.md',
            'API': 'SPEC007-api_design.md',
            'DBD': 'SPEC016-decoder_buffer_design.md',
            'SRC': 'SPEC017-sample_rate_conversion.md',
            'VOL': 'SPEC013-single_stream_playback.md',
            'SSP': 'SPEC014-single_stream_design.md',
            'DB': 'IMPL001-database_schema.md',
        }

        expected_files = file_map.get(prefix, [])
        if isinstance(expected_files, str):
            expected_files = [expected_files]

        if not expected_files:
            return True, ""  # Unknown prefix, can't validate

        # Check if requirement exists in any of the expected files
        for expected_file in expected_files:
            if expected_file in self.requirement_ids:
                if req_id in self.requirement_ids[expected_file]:
                    return True, ""

        return False, f"Requirement ID {req_id} not found in expected file(s): {', '.join(expected_files)}"

    def find_orphaned_references(self, file_path: Path) -> List[str]:
        """Find text mentions of SPEC016/SPEC017 without proper links"""
        orphaned = []
        content = file_path.read_text(encoding='utf-8')

        # Look for mentions of DBD-* or SRC-* that aren't in markdown links
        # Remove all markdown links first
        content_no_links = re.sub(r'\[([^\]]+)\]\(([^)]+)\)', r'\1', content)

        # Now find orphaned references
        for match in re.finditer(r'(DBD-[A-Z]+-\d+|SRC-[A-Z]+-\d+)', content_no_links):
            ref = match.group(1)
            orphaned.append(ref)

        return orphaned

    def validate_all(self) -> Dict:
        """Run all validation checks"""
        print("Caching all anchors and requirement IDs...")
        self.cache_all_anchors_and_ids()

        print(f"Validating {len(self.all_files)} documentation files...")

        for file_path in self.all_files:
            print(f"  Checking {file_path.name}...")

            # Validate markdown links
            links = self.extract_links_from_file(file_path)
            for link_text, link_target in links:
                self.total_links += 1
                is_valid, error = self.validate_link(file_path, link_text, link_target)

                if is_valid:
                    self.valid_links += 1
                else:
                    self.broken_links.append({
                        'file': file_path.name,
                        'link_text': link_text,
                        'link_target': link_target,
                        'error': error
                    })
                    self.documents_with_issues[file_path.name].append(error)

                    if 'SPEC016' in link_target or 'SPEC017' in link_target:
                        self.critical_issues.append(f"{file_path.name}: {error}")

            # Validate requirement references
            req_refs = self.find_requirement_references(file_path)
            for req_id in req_refs:
                is_valid, error = self.validate_requirement_reference(req_id)

                if not is_valid:
                    self.invalid_requirement_ids.append({
                        'file': file_path.name,
                        'requirement_id': req_id,
                        'error': error
                    })
                    self.documents_with_issues[file_path.name].append(error)

            # Find orphaned references
            orphaned = self.find_orphaned_references(file_path)
            if orphaned:
                for ref in orphaned:
                    self.orphaned_references.append({
                        'file': file_path.name,
                        'reference': ref
                    })
                    self.warnings.append(f"{file_path.name}: Orphaned reference {ref} (not in markdown link)")

        return self.generate_report()

    def generate_report(self) -> Dict:
        """Generate the final validation report"""
        total_issues = len(self.broken_links) + len(self.invalid_requirement_ids)

        # Calculate health score
        if self.total_links > 0:
            health_score = int((self.valid_links / self.total_links) * 100)
        else:
            health_score = 100

        # Adjust for requirement ID issues
        if self.invalid_requirement_ids:
            health_score = max(0, health_score - len(self.invalid_requirement_ids) * 2)

        # Find documents with most issues
        docs_sorted = sorted(
            self.documents_with_issues.items(),
            key=lambda x: len(x[1]),
            reverse=True
        )[:10]

        report = {
            'total_links_checked': self.total_links,
            'valid_links': self.valid_links,
            'broken_links': self.broken_links,
            'invalid_requirement_ids': self.invalid_requirement_ids,
            'missing_anchors': self.missing_anchors,
            'orphaned_references': self.orphaned_references,
            'documents_with_issues': dict(docs_sorted),
            'critical_issues': self.critical_issues,
            'warnings': self.warnings,
            'health_score': health_score,
            'summary': {
                'total_files_checked': len(self.all_files),
                'total_links': self.total_links,
                'valid_links': self.valid_links,
                'broken_links_count': len(self.broken_links),
                'invalid_req_ids_count': len(self.invalid_requirement_ids),
                'orphaned_refs_count': len(self.orphaned_references),
                'critical_issues_count': len(self.critical_issues),
                'warnings_count': len(self.warnings),
                'health_score': health_score
            }
        }

        return report


def main():
    docs_dir = Path('/home/sw/Dev/McRhythm/docs')
    output_file = Path('/home/sw/Dev/McRhythm/docs/validation/phase5-link-validation.json')

    validator = LinkValidator(docs_dir)
    report = validator.validate_all()

    # Write JSON report
    with open(output_file, 'w', encoding='utf-8') as f:
        json.dump(report, f, indent=2)

    # Print summary
    print("\n" + "="*80)
    print("LINK VALIDATION SUMMARY")
    print("="*80)
    print(f"Total files checked: {report['summary']['total_files_checked']}")
    print(f"Total links checked: {report['summary']['total_links']}")
    print(f"Valid links: {report['summary']['valid_links']}")
    print(f"Broken links: {report['summary']['broken_links_count']}")
    print(f"Invalid requirement IDs: {report['summary']['invalid_req_ids_count']}")
    print(f"Orphaned references: {report['summary']['orphaned_refs_count']}")
    print(f"Critical issues: {report['summary']['critical_issues_count']}")
    print(f"Warnings: {report['summary']['warnings_count']}")
    print(f"\nOverall health score: {report['summary']['health_score']}%")
    print("="*80)

    if report['broken_links']:
        print(f"\nBROKEN LINKS ({len(report['broken_links'])}):")
        for link in report['broken_links'][:10]:
            print(f"  - {link['file']}: [{link['link_text']}]({link['link_target']})")
            print(f"    Error: {link['error']}")

    if report['invalid_requirement_ids']:
        print(f"\nINVALID REQUIREMENT IDs ({len(report['invalid_requirement_ids'])}):")
        for item in report['invalid_requirement_ids'][:10]:
            print(f"  - {item['file']}: {item['requirement_id']}")
            print(f"    Error: {item['error']}")

    if report['critical_issues']:
        print(f"\nCRITICAL ISSUES ({len(report['critical_issues'])}):")
        for issue in report['critical_issues'][:10]:
            print(f"  - {issue}")

    print(f"\nFull report written to: {output_file}")


if __name__ == '__main__':
    main()
