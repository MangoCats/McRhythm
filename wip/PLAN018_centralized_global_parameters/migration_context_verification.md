# Context Verification for Hardcoded Value Replacement

**CRITICAL SAFETY PROCEDURE**

## Problem

Multiple parameters share the same default value:
- **DBD-PARAM-020** (working_sample_rate): 44100
- **DBD-PARAM-085** (decoder_resume_hysteresis_samples): 44100
- **DBD-PARAM-088** (mixer_min_start_level): 22050
- **DBD-PARAM-080** (playout_ringbuffer_headroom): 4410

**Risk:** Grep for `44100` will match BOTH working_sample_rate AND decoder_resume_hysteresis_samples usages. Blindly replacing all `44100` with `PARAMS.working_sample_rate` would break hysteresis logic.

---

## Context Verification Procedure (MANDATORY)

**For EACH grep match, verify context before replacement:**

### Step 1: Examine Surrounding Code

Read ±10 lines around match, look for:

1. **DBD-PARAM tags in comments**
   - `// [DBD-PARAM-020]` → working_sample_rate
   - `// [DBD-PARAM-085]` → decoder_resume_hysteresis_samples
   - **Action:** Replace with parameter matching tag

2. **Variable names**
   - `sample_rate`, `rate`, `sr` → working_sample_rate
   - `hysteresis`, `resume_threshold`, `gap` → decoder_resume_hysteresis_samples
   - **Action:** Replace with parameter matching semantic meaning

3. **Function/struct context**
   - `samples_to_ticks(frames, 44100)` → working_sample_rate
   - `PlayoutRingBuffer::new(661941, 4410, 44100)` → Check parameter order in constructor
   - **Action:** Verify parameter semantics before replacement

4. **Code comments**
   - `// 1 second at 44.1kHz` → Could be working_sample_rate OR hysteresis (ambiguous)
   - `// Resume threshold = hysteresis + headroom` → decoder_resume_hysteresis_samples
   - **Action:** Use comment semantics to disambiguate

5. **Mathematical expressions**
   - `duration_seconds * 44100` → working_sample_rate (time → samples conversion)
   - `free_space >= 44100 + headroom` → decoder_resume_hysteresis_samples (threshold logic)
   - **Action:** Understand calculation purpose

---

### Step 2: Classification Decision Tree

```
Found hardcoded 44100:

1. Is there a DBD-PARAM tag in ±5 lines?
   YES → Use parameter matching tag
   NO  → Continue to step 2

2. Is variable named sample_rate, rate, sr, freq, frequency?
   YES → Use working_sample_rate
   NO  → Continue to step 3

3. Is variable named hysteresis, resume_*, gap, threshold?
   YES → Use decoder_resume_hysteresis_samples
   NO  → Continue to step 4

4. Is this in a timing conversion function (ticks_to_samples, samples_to_ticks)?
   YES → Use working_sample_rate
   NO  → Continue to step 5

5. Is this in buffer pause/resume logic?
   YES → Use decoder_resume_hysteresis_samples
   NO  → Continue to step 6

6. Is this a test file?
   YES → SKIP (test files may use hardcoded values legitimately)
   NO  → Continue to step 7

7. AMBIGUOUS - Manual review required
   - Document in migration notes
   - Consult SPEC016 for parameter definition
   - Ask user if uncertain
```

---

### Step 3: Verification Checklist (Per Match)

For each hardcoded value found:

**Context Check:**
- [ ] Read ±10 lines of code around match
- [ ] Identified DBD-PARAM tag (if present)
- [ ] Understood variable semantic meaning
- [ ] Understood calculation/logic purpose

**Classification:**
- [ ] Determined which parameter this hardcoded value represents
- [ ] Verified against SPEC016 parameter definition
- [ ] Documented reasoning if ambiguous

**Replacement:**
- [ ] Replaced with correct PARAMS.parameter_name
- [ ] Did NOT blindly replace all instances of value
- [ ] Verified replacement compiles
- [ ] Verified tests pass

---

## Examples: Correct vs Incorrect Replacements

### Example 1: working_sample_rate (CORRECT)

**Code Found:**
```rust
// wkmp-ap/src/playback/mixer.rs:145
let tick_increment = samples_to_ticks(frames_read, 44100);
```

**Context Analysis:**
- **Function:** `samples_to_ticks` (timing conversion)
- **Variable name:** Implicit sample rate parameter
- **Semantics:** Converting audio frames to tick units
- **DBD-PARAM tag:** None visible

**Decision:** This is working_sample_rate (timing conversion requires actual device sample rate)

**Replacement:**
```rust
let sample_rate = *wkmp_common::params::PARAMS.working_sample_rate.read().unwrap();
let tick_increment = samples_to_ticks(frames_read, sample_rate);
```

---

### Example 2: decoder_resume_hysteresis_samples (CORRECT)

**Code Found:**
```rust
// wkmp-ap/src/playback/buffer_manager.rs:124
// [DBD-PARAM-085] decoder_resume_hysteresis_samples
let resume_threshold = 44100 + headroom;
```

**Context Analysis:**
- **DBD-PARAM tag:** DBD-PARAM-085 explicitly present
- **Variable name:** `resume_threshold`
- **Semantics:** Buffer pause/resume hysteresis logic
- **Comment:** Mentions hysteresis

**Decision:** This is decoder_resume_hysteresis_samples (explicit tag + semantics match)

**Replacement:**
```rust
// [DBD-PARAM-085] decoder_resume_hysteresis_samples
let hysteresis = *wkmp_common::params::PARAMS.decoder_resume_hysteresis_samples.read().unwrap();
let resume_threshold = hysteresis + headroom;
```

---

### Example 3: AMBIGUOUS (REQUIRES INVESTIGATION)

**Code Found:**
```rust
// wkmp-ap/src/some_file.rs:200
let buffer_size = 44100;
```

**Context Analysis:**
- **Variable name:** `buffer_size` (ambiguous - could be time-based or samples)
- **DBD-PARAM tag:** None visible
- **Semantics:** Unclear without more context

**Decision:** INVESTIGATE - Read surrounding function to understand purpose

**Investigation Steps:**
1. Read entire function containing this line
2. Check function name and doc comments
3. Look for usage of `buffer_size` variable
4. Determine if this is:
   - Audio buffer (use working_sample_rate for sizing calculation)
   - Hysteresis gap (use decoder_resume_hysteresis_samples)
   - Constant unrelated to parameters (leave as-is or extract to named constant)

---

## Parameters with Shared Default Values

**Special Attention Required:**

| Default Value | Parameters |
|---------------|------------|
| **44100** | working_sample_rate (DBD-PARAM-020), decoder_resume_hysteresis_samples (DBD-PARAM-085) |
| **22050** | mixer_min_start_level (DBD-PARAM-088) [also 44100/2] |
| **4410** | playout_ringbuffer_headroom (DBD-PARAM-080), decode_chunk_size (DBD-PARAM-065) / 5.67 |
| **0.95** | pause_decay_factor (DBD-PARAM-090) [unique] |

**Grep Strategy:**
1. Search for value: `rg "44100" --type rust -C 5`
2. For EACH match:
   - Apply decision tree
   - Classify as PARAM-020 or PARAM-085
   - Document reasoning
   - Replace only after verification

---

## Updated Step 2 (Per-Parameter Migration)

**Original (Unsafe):**
```bash
# Step 2: Find All Hardcoded References
rg "44100" --type rust -g '!*test*.rs'
rg "DBD-PARAM-020" --type rust
```

**UPDATED (Safe with Context Verification):**

```bash
# Step 2: Find All Hardcoded References WITH CONTEXT
rg "44100" --type rust -g '!*test*.rs' -C 10 > migration_matches.txt

# For EACH match in migration_matches.txt:
# 1. Read context (±10 lines)
# 2. Identify DBD-PARAM tag if present
# 3. Analyze variable semantics
# 4. Classify using decision tree
# 5. Document: file:line, value, parameter identified, reasoning
# 6. Replace ONLY verified matches

# Example documentation format:
# mixer.rs:145 | 44100 | working_sample_rate | Reason: samples_to_ticks timing conversion
# buffer_manager.rs:124 | 44100 | decoder_resume_hysteresis_samples | Reason: DBD-PARAM-085 tag + resume logic
```

**Create migration_log.md:**
```markdown
# Parameter Migration Log: [parameter_name]

| File:Line | Hardcoded Value | Parameter | Reasoning | Replaced? |
|-----------|----------------|-----------|-----------|-----------|
| mixer.rs:145 | 44100 | working_sample_rate | samples_to_ticks call | YES |
| buffer_manager.rs:124 | 44100 | decoder_resume_hysteresis_samples | DBD-PARAM-085 tag | YES |
| tests/timing.rs:50 | 44100 | N/A | Test constant | NO (test file) |
```

---

## Verification After Replacement

After replacing hardcoded values, verify:

1. **Compilation:** `cargo build --workspace`
2. **Test Suite:** `cargo test --workspace`
3. **Semantic Check:** Re-read modified lines, verify replacement makes sense
4. **Performance Check:** For timing-critical parameters, verify no performance regression

---

## Rollback if Context Mismatch Detected

**If replacement causes test failure:**

1. **Immediate rollback:** `git reset --hard HEAD`
2. **Review migration_log.md:** Identify which replacement was incorrect
3. **Re-analyze context:** Which hardcoded value was misidentified?
4. **Correct classification:** Update decision tree reasoning
5. **Re-attempt migration:** With corrected parameter identification

---

## Integration with TC-M-003-01 (Manual Code Review)

**TC-M-003-01 Step 2 ENHANCED:**

After automated grep, for each match:
- [ ] Read ±10 lines of context
- [ ] Apply decision tree from this document
- [ ] Document classification in migration_log.md
- [ ] Verify DBD-PARAM tag matches (if present)
- [ ] Check semantic meaning matches parameter definition
- [ ] Only replace after verification

**CRITICAL RULE:** Never bulk-replace all instances of a hardcoded value. Always verify context.

---

## Document Status

**Created:** 2025-11-02
**Purpose:** Prevent incorrect parameter replacements during migration
**Applies to:** All 15 parameter migrations, especially:
- working_sample_rate (DBD-PARAM-020)
- decoder_resume_hysteresis_samples (DBD-PARAM-085)
- mixer_min_start_level (DBD-PARAM-088)

**This procedure is MANDATORY for Step 2 of per-parameter migration process.**
