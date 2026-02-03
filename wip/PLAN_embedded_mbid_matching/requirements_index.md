# Requirements Index

**Specification:** SPEC-EMBID-001 (Embedded MusicBrainz ID Matching)
**Total Requirements:** 12 (8 Functional + 4 Non-Functional)

## Functional Requirements

| Req ID | Type | Brief Description | Spec Line | Priority | Complexity | Status |
|--------|------|-------------------|-----------|----------|------------|--------|
| FR-01 | Functional | Extract embedded MB Recording ID from ID3 tags | 226 | P0 | Low | Exists (partial) |
| FR-02 | Functional | Extract ISRC from ID3 tags | 227 | P0 | Low | New |
| FR-03 | Functional | Validate MB Recording ID format (UUID) | 228 | P0 | Low | Exists |
| FR-04 | Functional | Assign confidence tier based on metadata | 229 | P0 | Medium | New |
| FR-05 | Functional | Skip AcoustID when embedded MB ID present | 230 | P1 | Medium | New |
| FR-06 | Functional | Fall back to AcoustID when no embedded ID | 231 | P1 | Low | Exists |
| FR-07 | Functional | Optionally validate MB ID against database | 232 | P2 | Medium | New |
| FR-08 | Functional | Log confidence tier with match results | 233 | P1 | Low | New |

## Non-Functional Requirements

| Req ID | Type | Brief Description | Spec Line | Target | Complexity | Status |
|--------|------|-------------------|-----------|--------|------------|--------|
| NFR-01 | Performance | Stage 0 matching (ID3 only) | 239 | < 50ms/file | Low | Verify |
| NFR-02 | Coverage | Total coverage rate | 240 | >= 98% | N/A | Measure |
| NFR-03 | Accuracy | Tier 1 false positive rate | 241 | < 0.1% | N/A | Measure |
| NFR-04 | Availability | Stage 0 works without network | 242 | 100% offline | Low | Design |

## Complexity Summary

- **Low:** 7 requirements (FR-01, FR-02, FR-03, FR-06, FR-08, NFR-01, NFR-04)
- **Medium:** 3 requirements (FR-04, FR-05, FR-07)
- **N/A (measurement only):** 2 requirements (NFR-02, NFR-03)

## Existing Code Leverage

| Requirement | Existing Code | Modification Needed |
|-------------|---------------|---------------------|
| FR-01 | `id3_extractor.rs:172-195` | Minor - already extracts MB ID |
| FR-03 | `id3_extractor.rs:257-279` | None - UUID validation exists |
| FR-06 | `acoustid_client.rs` | None - already works as fallback |

## Dependencies

- **lofty crate:** ID3 tag reading (already in use)
- **uuid crate:** UUID validation (already in use)
- **Existing pipeline:** ID3Extractor, IdentityResolver
