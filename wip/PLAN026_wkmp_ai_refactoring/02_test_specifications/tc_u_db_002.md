# TC-U-DB-002: Passages Table Tests

**Requirement:** REQ-DB-002 (Passages table with tick-based timing)
**Type:** Unit Tests
**Source:** refactor1126.md lines 293-354

---

## TC-U-DB-002-01: Passages Table with Tick-Based Timing

**Scope:** Passages table schema and timing columns

**Given:**
- Clean test database with schema applied
- A test file entry exists in files table

**When:**
- Insert passage with all timing fields:
  - start_time_ticks = 0
  - end_time_ticks = 84_672_000 (3 seconds)
  - fade_in_start_ticks = 28_224_000 (1 second)
  - lead_in_start_ticks = 28_224_000
  - lead_out_start_ticks = 56_448_000 (2 seconds)
  - fade_out_start_ticks = 56_448_000

**Then:**
- Passage inserted successfully
- All timing values stored as INTEGER
- Timing values round-trip correctly on SELECT
- Foreign key to file_id enforced

**Pass Criteria:**
- `INSERT` succeeds without error
- `SELECT` returns exact tick values
- `TYPEOF(start_time_ticks) == 'integer'`
- Attempt to insert with invalid file_id fails with FK constraint

**Estimated Effort:** 30 minutes

---

## TC-U-DB-002-02: Passage Constraints Enforced

**Scope:** Database constraints per SPEC002

**Given:**
- Clean test database with passages table

**When:**
- Attempt invalid inserts:
  - start_time_ticks < 0 (negative start)
  - end_time_ticks <= start_time_ticks (end before start)
  - fade_in_start_ticks outside passage bounds
  - lead_out_start_ticks outside passage bounds

**Then:**
- All invalid inserts rejected by database
- Constraint violation errors returned
- Valid inserts still succeed after rejections

**Pass Criteria:**
- Negative start rejected: `CHECK constraint failed`
- End before start rejected: `CHECK constraint failed`
- Out-of-bounds fade rejected: `CHECK constraint failed`
- Valid insert after failures succeeds

**Test Cases:**

| start | end | fade_in | lead_out | Expected |
|-------|-----|---------|----------|----------|
| -1 | 100 | NULL | NULL | REJECT (negative start) |
| 100 | 50 | NULL | NULL | REJECT (end < start) |
| 0 | 100 | 200 | NULL | REJECT (fade > end) |
| 0 | 100 | NULL | -5 | REJECT (lead < start) |
| 0 | 100 | 50 | 80 | ACCEPT |
| 0 | 100 | NULL | NULL | ACCEPT (NULLs valid) |

**Estimated Effort:** 45 minutes

---

## Test Data

**SQL for test setup:**
```sql
-- Insert test file first
INSERT INTO files (guid, path, status)
VALUES ('test-file-001', '/test/audio.mp3', 'AUDIO_DECODED');

-- Then test passage insertions
```

**Test Database:** In-memory SQLite for fast tests
