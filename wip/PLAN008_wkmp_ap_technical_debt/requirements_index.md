# Requirements Index - wkmp-ap Technical Debt Remediation

**Source Document:** wip/SPEC024-wkmp_ap_technical_debt_remediation.md
**Total Requirements:** 37
**Plan Number:** PLAN008

---

## Requirements by Category

### Critical Security (1 requirement)

| Req ID | Brief Description | Line # | Priority | Sprint |
|--------|-------------------|--------|----------|--------|
| REQ-DEBT-SEC-001-010 | Validate authentication for ALL POST/PUT requests | 39 | CRITICAL | 1 |
| REQ-DEBT-SEC-001-020 | POST/PUT auth uses shared_secret mechanism | 41 | CRITICAL | 1 |
| REQ-DEBT-SEC-001-030 | Extract credentials from JSON body field | 43 | CRITICAL | 1 |
| REQ-DEBT-SEC-001-040 | Return HTTP 401 on auth failure | 45 | CRITICAL | 1 |

### High-Priority Functionality (17 requirements)

| Req ID | Brief Description | Line # | Priority | Sprint |
|--------|-------------------|--------|----------|--------|
| REQ-DEBT-FUNC-001-010 | Decoder errors include absolute file path | 108 | HIGH | 1 |
| REQ-DEBT-FUNC-001-020 | ChunkedDecoder stores file_path field | 110 | HIGH | 1 |
| REQ-DEBT-FUNC-001-030 | Use stored file_path in error construction | 112 | HIGH | 1 |
| REQ-DEBT-FUNC-002-010 | Read buffer_capacity_samples from database | 177 | HIGH | 1 |
| REQ-DEBT-FUNC-002-020 | Read buffer_headroom_samples from database | 179 | HIGH | 1 |
| REQ-DEBT-FUNC-002-030 | Use compiled defaults if settings NULL | 181 | HIGH | 1 |
| REQ-DEBT-FUNC-002-040 | Load settings once at BufferManager creation | 183 | HIGH | 1 |
| REQ-DEBT-FUNC-003-010 | Include decoder_state in BufferChainInfo | 268 | HIGH | 2 |
| REQ-DEBT-FUNC-003-020 | Include source_sample_rate in BufferChainInfo | 270 | HIGH | 2 |
| REQ-DEBT-FUNC-003-030 | Include fade_stage in BufferChainInfo | 272 | HIGH | 2 |
| REQ-DEBT-FUNC-003-040 | Include started_at timestamp in BufferChainInfo | 274 | HIGH | 2 |
| REQ-DEBT-FUNC-004-010 | PassageStarted includes song_albums field | 374 | HIGH | 2 |
| REQ-DEBT-FUNC-004-020 | PassageComplete includes song_albums field | 376 | HIGH | 2 |
| REQ-DEBT-FUNC-004-030 | Query albums joining passages→passage_albums→albums | 378 | HIGH | 2 |
| REQ-DEBT-FUNC-005-010 | Track passage playback start time | 464 | HIGH | 2 |
| REQ-DEBT-FUNC-005-020 | Calculate duration_played as elapsed time | 466 | HIGH | 2 |
| REQ-DEBT-FUNC-005-030 | Duration in seconds with millisecond precision | 468 | HIGH | 2 |
| REQ-DEBT-FUNC-005-040 | duration_played = 0.0 if start time unavailable | 470 | HIGH | 2 |

### Code Quality (10 requirements)

| Req ID | Brief Description | Line # | Priority | Sprint |
|--------|-------------------|--------|----------|--------|
| REQ-DEBT-QUALITY-001-010 | No .unwrap() in critical audio thread code | 572 | MEDIUM | 3 |
| REQ-DEBT-QUALITY-001-020 | Proper error propagation for mutex locks | 574 | MEDIUM | 3 |
| REQ-DEBT-QUALITY-001-030 | match/if-let for event channels (no unwrap) | 576 | MEDIUM | 3 |
| REQ-DEBT-QUALITY-002-010 | Split engine.rs into 3 modules | 619 | MEDIUM | 3 |
| REQ-DEBT-QUALITY-002-020 | Each refactored module <1500 lines | 621 | MEDIUM | 3 |
| REQ-DEBT-QUALITY-002-030 | Public API unchanged (internal refactor only) | 623 | MEDIUM | 3 |
| REQ-DEBT-QUALITY-003-010 | Remove all unused imports | 673 | MEDIUM | 3 |
| REQ-DEBT-QUALITY-003-020 | Remove dead code never called | 675 | MEDIUM | 3 |
| REQ-DEBT-QUALITY-003-030 | Mark Axum-routed functions with #[allow(dead_code)] | 677 | MEDIUM | 3 |
| REQ-DEBT-QUALITY-004-010 | Only one config module in wkmp-ap | 705 | MEDIUM | 2 |

### Low-Priority Cleanup (3 requirements)

| Req ID | Brief Description | Line # | Priority | Sprint |
|--------|-------------------|--------|----------|--------|
| REQ-DEBT-QUALITY-004-020 | Identify active config by main.rs imports | 707 | MEDIUM | 2 |
| REQ-DEBT-QUALITY-004-030 | Delete obsolete config file | 709 | MEDIUM | 2 |
| REQ-DEBT-QUALITY-005-010 | No backup files in repository | 729 | LOW | 2 |
| REQ-DEBT-QUALITY-005-020 | Version control as backup mechanism | 731 | LOW | 2 |

### Future Enhancements (2 requirements)

| Req ID | Brief Description | Line # | Priority | Sprint |
|--------|-------------------|--------|----------|--------|
| REQ-DEBT-FUTURE-003-010 | Log warning when audio clipping detected | 779 | LOW | 3 |

---

## Requirements Summary by Sprint

**Sprint 1 (Security & Critical):** 11 requirements
- 4 CRITICAL (authentication)
- 7 HIGH (file paths, buffer config)

**Sprint 2 (Functionality & Diagnostics):** 14 requirements
- 12 HIGH (telemetry, metadata, duration)
- 2 MEDIUM (config cleanup)

**Sprint 3 (Code Health):** 12 requirements
- 10 MEDIUM (unwrap audit, refactoring, warnings)
- 1 LOW (clipping log)
- 1 deferred (outdated TODO removal - trivial)

**Total:** 37 requirements across 13 debt items

---

## Requirement Dependencies

**No blocking dependencies between sprints** - can proceed linearly:
- Sprint 1 → Sprint 2: Auth must be secure before adding features
- Sprint 2 → Sprint 3: Functionality complete before refactoring

**Internal dependencies:**
- REQ-DEBT-FUNC-003-* depends on decoder telemetry infrastructure
- REQ-DEBT-FUNC-004-* depends on database query function
- REQ-DEBT-FUNC-005-* depends on mixer timestamp tracking
