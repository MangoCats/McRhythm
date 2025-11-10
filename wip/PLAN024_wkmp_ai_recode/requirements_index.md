# Requirements Index: WKMP-AI Audio Import System Recode

**Source:** wip/SPEC_wkmp_ai_recode.md (1375 lines)
**Extracted:** 2025-11-09
**Total Requirements:** 72

---

## Summary Statistics

**By Priority:**
- High: 66 (91.7%)
- Medium: 4 (5.6%)
- Low: 2 (2.8%)

**By Type:**
- Functional: 61 (84.7%)
- Non-Functional: 11 (15.3%)

**Top-Level Requirements:** 14
**Nested Requirements:** 58

---

## Requirements Table

| Req ID | Type | Brief Description | Line # | Priority | Parent |
|--------|------|-------------------|--------|----------|--------|
| REQ-AI-010 | Functional | Per-Song Import Workflow | 74 | High | - |
| REQ-AI-011 | Functional | Phase 0: Passage Boundary Detection | 78 | High | REQ-AI-010 |
| REQ-AI-012 | Functional | Phase 1-6: Per-Song Processing | 84 | High | REQ-AI-010 |
| REQ-AI-013 | Functional | Per-Song Error Isolation | 94 | High | REQ-AI-010 |
| REQ-AI-020 | Functional | Identity Resolution (Hybrid Fusion Tier 2) | 100 | High | - |
| REQ-AI-021 | Functional | Multi-Source MBID Resolution | 104 | High | REQ-AI-020 |
| REQ-AI-022 | Functional | Conflict Detection | 111 | High | REQ-AI-020 |
| REQ-AI-023 | Functional | Bayesian Update Algorithm | 117 | High | REQ-AI-020 |
| REQ-AI-024 | Functional | Low-Confidence Flagging | 124 | High | REQ-AI-020 |
| REQ-AI-030 | Functional | Metadata Fusion (Hybrid Fusion Tier 2) | 128 | High | - |
| REQ-AI-031 | Functional | Multi-Source Metadata Extraction | 131 | High | REQ-AI-030 |
| REQ-AI-032 | Functional | Quality Scoring | 136 | High | REQ-AI-030 |
| REQ-AI-033 | Functional | Field-Wise Selection Strategy | 141 | High | REQ-AI-030 |
| REQ-AI-034 | Functional | Consistency Validation | 147 | High | REQ-AI-030 |
| REQ-AI-040 | Functional | Musical Flavor Synthesis (Hybrid Fusion Tier 2) | 152 | High | - |
| REQ-AI-041 | Functional | Multi-Source Flavor Extraction (Parallel) | 156 | High | REQ-AI-040 |
| REQ-AI-042 | Functional | Source Priority and Confidence | 163 | High | REQ-AI-040 |
| REQ-AI-043 | Functional | Characteristic-Wise Weighted Averaging | 169 | High | REQ-AI-040 |
| REQ-AI-044 | Functional | Normalization | 175 | High | REQ-AI-040 |
| REQ-AI-045 | Functional | Completeness Scoring | 180 | High | REQ-AI-040 |
| REQ-AI-050 | Functional | Passage Boundary Detection (Hybrid Fusion Tier 1) | 184 | High | - |
| REQ-AI-051 | Functional | Silence Detection (Baseline) | 188 | High | REQ-AI-050 |
| REQ-AI-052 | Functional | Multi-Strategy Fusion (Future Extension) | 193 | Medium | REQ-AI-050 |
| REQ-AI-053 | Functional | Boundary Validation | 198 | High | REQ-AI-050 |
| REQ-AI-060 | Functional | Quality Validation (Hybrid Fusion Tier 3) | 205 | High | - |
| REQ-AI-061 | Functional | Title Consistency Check | 209 | High | REQ-AI-060 |
| REQ-AI-062 | Functional | Duration Consistency Check | 215 | High | REQ-AI-060 |
| REQ-AI-063 | Functional | Genre-Flavor Alignment Check | 220 | High | REQ-AI-060 |
| REQ-AI-064 | Functional | Overall Quality Score | 227 | High | REQ-AI-060 |
| REQ-AI-070 | Functional | Real-Time SSE Event Streaming | 232 | High | - |
| REQ-AI-071 | Functional | Event Types | 236 | High | REQ-AI-070 |
| REQ-AI-072 | Functional | Event Format | 248 | High | REQ-AI-070 |
| REQ-AI-073 | Functional | Event Throttling | 254 | High | REQ-AI-070 |
| REQ-AI-075 | Functional | UI Progress Reporting and User Feedback | 259 | High | - |
| REQ-AI-075-01 | Functional | Real-Time Progress Updates | 263 | High | REQ-AI-075 |
| REQ-AI-075-02 | Functional | Estimated Time Remaining (ETA) | 270 | High | REQ-AI-075 |
| REQ-AI-075-03 | Functional | File-by-File Processing Workflow Clarity | 277 | High | REQ-AI-075 |
| REQ-AI-075-04 | Functional | Multi-Phase Progress Visualization | 284 | High | REQ-AI-075 |
| REQ-AI-075-05 | Functional | Accurate Progress Counter Behavior | 298 | High | REQ-AI-075 |
| REQ-AI-075-06 | Functional | Current Operation Clarity | 308 | High | REQ-AI-075 |
| REQ-AI-075-07 | Functional | Per-Song Granularity Feedback | 319 | High | REQ-AI-075 |
| REQ-AI-075-08 | Functional | Error and Warning Visibility | 326 | High | REQ-AI-075 |
| REQ-AI-078 | Functional | Database Initialization and Self-Repair | 333 | High | - |
| REQ-AI-078-01 | Functional | Zero-Configuration Startup | 337 | High | REQ-AI-078 |
| REQ-AI-078-02 | Functional | Self-Repair for Schema Changes | 343 | High | REQ-AI-078 |
| REQ-AI-078-03 | Functional | Migration Framework Integration | 349 | High | REQ-AI-078 |
| REQ-AI-078-04 | Functional | Breaking Changes Handling | 355 | High | REQ-AI-078 |
| REQ-AI-080 | Functional | Database Schema Extensions | 366 | High | - |
| REQ-AI-081 | Functional | Flavor Source Provenance | 370 | High | REQ-AI-080 |
| REQ-AI-082 | Functional | Metadata Source Provenance | 374 | High | REQ-AI-080 |
| REQ-AI-083 | Functional | Identity Resolution Tracking | 380 | High | REQ-AI-080 |
| REQ-AI-084 | Functional | Quality Scores | 386 | High | REQ-AI-080 |
| REQ-AI-085 | Functional | Validation Flags | 390 | High | REQ-AI-080 |
| REQ-AI-086 | Functional | Import Metadata | 394 | High | REQ-AI-080 |
| REQ-AI-087 | Functional | Import Provenance Log Table | 399 | High | REQ-AI-080 |
| REQ-AI-088 | Functional | SPEC017 Time Representation Compliance | 412 | High | - |
| REQ-AI-088-01 | Functional | Tick Definition (per SPEC017 [SRC-TICK-030]) | 416 | High | REQ-AI-088 |
| REQ-AI-088-02 | Functional | Database Storage (per SPEC017 [SRC-DB-010] through [SRC-DB-016]) | 421 | High | REQ-AI-088 |
| REQ-AI-088-03 | Functional | Internal Representation (per SPEC017 [SRC-LAYER-011]) | 429 | High | REQ-AI-088 |
| REQ-AI-088-04 | Functional | Conversion Rules (per SPEC017 [SRC-LAYER-030]) | 435 | High | REQ-AI-088 |
| REQ-AI-088-05 | Functional | Boundary Detection | 441 | High | REQ-AI-088 |
| REQ-AI-NF-010 | Non-Functional | Performance | 460 | High | - |
| REQ-AI-NF-011 | Non-Functional | Sequential Processing Performance | 462 | Medium | REQ-AI-NF-010 |
| REQ-AI-NF-012 | Non-Functional | Parallel Extraction | 467 | High | REQ-AI-NF-010 |
| REQ-AI-NF-020 | Non-Functional | Reliability | 472 | High | - |
| REQ-AI-NF-021 | Non-Functional | Error Isolation | 474 | High | REQ-AI-NF-020 |
| REQ-AI-NF-022 | Non-Functional | Graceful Degradation | 479 | High | REQ-AI-NF-020 |
| REQ-AI-NF-030 | Non-Functional | Maintainability | 484 | High | - |
| REQ-AI-NF-031 | Non-Functional | Modular Architecture | 486 | High | REQ-AI-NF-030 |
| REQ-AI-NF-032 | Non-Functional | Testability | 492 | High | REQ-AI-NF-030 |
| REQ-AI-NF-040 | Non-Functional | Extensibility | 498 | Medium | - |
| REQ-AI-NF-041 | Non-Functional | New Source Integration | 500 | Medium | REQ-AI-NF-040 |
| REQ-AI-NF-042 | Non-Functional | Future Optimizations | 505 | Low | REQ-AI-NF-040 |

---

## Requirements by Functional Area

### Per-Song Processing (REQ-AI-010)
- REQ-AI-011: Phase 0 Passage Boundary Detection
- REQ-AI-012: Phase 1-6 Per-Song Processing
- REQ-AI-013: Per-Song Error Isolation

### Identity Resolution (REQ-AI-020)
- REQ-AI-021: Multi-Source MBID Resolution
- REQ-AI-022: Conflict Detection
- REQ-AI-023: Bayesian Update Algorithm
- REQ-AI-024: Low-Confidence Flagging

### Metadata Fusion (REQ-AI-030)
- REQ-AI-031: Multi-Source Metadata Extraction
- REQ-AI-032: Quality Scoring
- REQ-AI-033: Field-Wise Selection Strategy
- REQ-AI-034: Consistency Validation

### Musical Flavor Synthesis (REQ-AI-040)
- REQ-AI-041: Multi-Source Flavor Extraction
- REQ-AI-042: Source Priority and Confidence
- REQ-AI-043: Characteristic-Wise Weighted Averaging
- REQ-AI-044: Normalization
- REQ-AI-045: Completeness Scoring

### Passage Boundary Detection (REQ-AI-050)
- REQ-AI-051: Silence Detection (Baseline)
- REQ-AI-052: Multi-Strategy Fusion (Future)
- REQ-AI-053: Boundary Validation

### Quality Validation (REQ-AI-060)
- REQ-AI-061: Title Consistency Check
- REQ-AI-062: Duration Consistency Check
- REQ-AI-063: Genre-Flavor Alignment Check
- REQ-AI-064: Overall Quality Score

### SSE Event Streaming (REQ-AI-070)
- REQ-AI-071: Event Types
- REQ-AI-072: Event Format
- REQ-AI-073: Event Throttling

### UI Progress Reporting (REQ-AI-075)
- REQ-AI-075-01: Real-Time Progress Updates
- REQ-AI-075-02: Estimated Time Remaining
- REQ-AI-075-03: File-by-File Workflow Clarity
- REQ-AI-075-04: Multi-Phase Progress Visualization
- REQ-AI-075-05: Accurate Progress Counter Behavior
- REQ-AI-075-06: Current Operation Clarity
- REQ-AI-075-07: Per-Song Granularity Feedback
- REQ-AI-075-08: Error and Warning Visibility

### Database Initialization (REQ-AI-078)
- REQ-AI-078-01: Zero-Configuration Startup
- REQ-AI-078-02: Self-Repair for Schema Changes
- REQ-AI-078-03: Migration Framework Integration
- REQ-AI-078-04: Breaking Changes Handling

### Database Schema (REQ-AI-080)
- REQ-AI-081: Flavor Source Provenance
- REQ-AI-082: Metadata Source Provenance
- REQ-AI-083: Identity Resolution Tracking
- REQ-AI-084: Quality Scores
- REQ-AI-085: Validation Flags
- REQ-AI-086: Import Metadata
- REQ-AI-087: Import Provenance Log Table

### Time Representation (REQ-AI-088)
- REQ-AI-088-01: Tick Definition
- REQ-AI-088-02: Database Storage
- REQ-AI-088-03: Internal Representation
- REQ-AI-088-04: Conversion Rules
- REQ-AI-088-05: Boundary Detection

### Performance (REQ-AI-NF-010)
- REQ-AI-NF-011: Sequential Processing Performance
- REQ-AI-NF-012: Parallel Extraction

### Reliability (REQ-AI-NF-020)
- REQ-AI-NF-021: Error Isolation
- REQ-AI-NF-022: Graceful Degradation

### Maintainability (REQ-AI-NF-030)
- REQ-AI-NF-031: Modular Architecture
- REQ-AI-NF-032: Testability

### Extensibility (REQ-AI-NF-040)
- REQ-AI-NF-041: New Source Integration
- REQ-AI-NF-042: Future Optimizations
