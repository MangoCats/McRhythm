# SPEC016/SPEC017 Cross-Reference Linking Guide

**Document Type:** Validation Guide
**Created:** 2025-10-19
**Purpose:** Comprehensive guide for how other documents should reference SPEC016 (Decoder Buffer Design) and SPEC017 (Sample Rate Conversion)

---

## Overview

SPEC016 and SPEC017 are **Tier 2 authoritative design specifications** that define:

- **SPEC016:** Operating parameters, decoder-buffer architecture, tick-based timing system
- **SPEC017:** Sample rate conversion formulas, tick system design, dual timing coexistence

These specifications contain **129 linkable concepts** that other documents should reference for:
- Operating parameters (9 in SPEC016)
- Conversion formulas (7 in SPEC017)
- Component definitions (decoder, resampler, fade handler, mixer, etc.)
- Requirement IDs (63 in SPEC016, 50 in SPEC017)
- Database field specifications (6 tick-based fields)

---

## Table of Contents

1. [Linkable Concepts Catalog](#linkable-concepts-catalog)
2. [Linking Patterns](#linking-patterns)
3. [Glossary Entries](#glossary-entries)
4. [Integration Points](#integration-points)
5. [Recommended Anchor IDs](#recommended-anchor-ids)
6. [Cross-Reference Examples](#cross-reference-examples)
7. [Best Practices](#best-practices)

---

## Linkable Concepts Catalog

### SPEC016: Decoder Buffer Design (63 requirement IDs)

#### Operating Parameters (9 parameters)

| Parameter ID | Name | Default | Description | Anchor Suggestion |
|--------------|------|---------|-------------|-------------------|
| [DBD-PARAM-020] | `working_sample_rate` | 44100 Hz | Internal sample rate for all mixing | #working-sample-rate |
| [DBD-PARAM-030] | `output_ringbuffer_size` | 8192 samples | Max stereo samples in output buffer (185ms @ 44.1kHz) | #output-ringbuffer-size |
| [DBD-PARAM-040] | `output_refill_period` | 90 ms | Wall clock time between mixer buffer checks | #output-refill-period |
| [DBD-PARAM-050] | `maximum_decode_streams` | 12 | Max concurrent audio decoders | #maximum-decode-streams |
| [DBD-PARAM-060] | `decode_work_period` | 5000 ms | Time between decode job priority re-evaluation | #decode-work-period |
| [DBD-PARAM-070] | `playout_ringbuffer_size` | 661,941 samples | Per-passage buffer size (15.01s @ 44.1kHz) | #playout-ringbuffer-size |
| [DBD-PARAM-080] | `playout_ringbuffer_headroom` | 441 samples | Reserved samples for resampler overshoot (0.01s @ 44.1kHz) | #playout-ringbuffer-headroom |
| [DBD-PARAM-090] | `pause_decay_factor` | 0.96875 (31/32) | Exponential decay multiplier during pause | #pause-decay-factor |
| [DBD-PARAM-100] | `pause_decay_floor` | 0.0001778 | Amplitude threshold for zero output during pause | #pause-decay-floor |

#### Component Definitions

| Component | Requirement ID | Description | Anchor Suggestion |
|-----------|----------------|-------------|-------------------|
| Decoder-buffer chain | [DBD-OV-040] | Pipeline: Decoder→Resampler→Fade→Buffer→Mixer | #decoder-buffer-chain |
| Decoder | [DBD-DEC-010] to [DBD-DEC-080] | Decodes audio from source files | #decoder |
| Resampler | [DBD-RSMP-010], [DBD-RSMP-020] | Converts audio to working_sample_rate | #resampler |
| Fade handler | [DBD-FADE-010] to [DBD-FADE-060] | Applies fade-in/out curves | #fade-handler |
| Buffer | [DBD-BUF-010] to [DBD-BUF-060] | Ring buffer for decoded audio | #buffer |
| Mixer | [DBD-MIX-010] to [DBD-MIX-052] | Combines passages, applies volume/crossfade | #mixer |
| Output | [DBD-OUT-010] | Sends audio to cpal output library | #output |
| Backpressure | [DBD-BUF-050] | Flow control via buffer headroom | #backpressure |

#### Requirement ID Categories

- **Scope:** [DBD-SC-010]
- **Overview:** [DBD-OV-010] through [DBD-OV-080] (8 IDs)
- **Parameters:** [DBD-PARAM-010] through [DBD-PARAM-100] (10 IDs)
- **Dataflow:** [DBD-FLOW-010] through [DBD-FLOW-110] (9 IDs)
- **Decoders:** [DBD-DEC-010] through [DBD-DEC-080] (8 IDs)
- **Resampling:** [DBD-RSMP-010], [DBD-RSMP-020] (2 IDs)
- **Fade Handlers:** [DBD-FADE-010] through [DBD-FADE-060] (6 IDs)
- **Buffers:** [DBD-BUF-010] through [DBD-BUF-060] (6 IDs)
- **Mixer:** [DBD-MIX-010] through [DBD-MIX-052] (8 IDs)
- **Output:** [DBD-OUT-010] (1 ID)
- **Sample Format:** [DBD-FMT-010], [DBD-FMT-020] (2 IDs)

### SPEC017: Sample Rate Conversion (50 requirement IDs)

#### Conversion Formulas (7 formulas)

| Formula ID | Name | Formula | Description | Anchor Suggestion |
|------------|------|---------|-------------|-------------------|
| [SRC-CONV-010] | ticks_per_sample | `ticks_per_sample = 28,224,000 ÷ sample_rate` | Calculate ticks per audio sample | #ticks-per-sample |
| [SRC-CONV-030] | samples_to_ticks | `ticks = samples × (28,224,000 ÷ sample_rate)` | Convert sample counts to ticks | #samples-to-ticks |
| [SRC-TIME-010] | ticks_to_seconds | `seconds = ticks ÷ 28,224,000` | Convert ticks to wall-clock seconds | #ticks-to-seconds |
| [SRC-API-020] | api_to_database | `ticks = milliseconds × 28,224` | API milliseconds → database ticks | #api-to-database |
| [SRC-API-030] | database_to_api | `milliseconds = ticks ÷ 28,224` (rounded) | Database ticks → API milliseconds | #database-to-api |
| [SRC-WSR-030] | ticks_to_samples | `samples = (ticks × working_sample_rate) ÷ 28,224,000` | Ticks → samples at working rate | #ticks-to-samples-working |
| [SRC-WSR-040] | ticks_to_samples_44100 | `samples = ticks ÷ 640` | Simplified conversion @ 44.1kHz | #ticks-to-samples-44100 |

#### Key Constants

| Constant | Value | Requirement ID | Description | Anchor Suggestion |
|----------|-------|----------------|-------------|-------------------|
| TICK_RATE | 28,224,000 Hz | [SRC-TICK-020] | LCM of all supported sample rates | #tick-rate |
| TICK_DURATION | ~35.4 nanoseconds | [SRC-TICK-030] | Duration of one tick (1/28,224,000 s) | #tick-duration |

#### Database Fields (6 tick-based fields)

| Field ID | Field Name | Type | Description |
|----------|------------|------|-------------|
| [SRC-DB-011] | `start_time` | INTEGER (i64) | Passage start boundary (ticks from file start) |
| [SRC-DB-012] | `end_time` | INTEGER (i64) | Passage end boundary (ticks from file start) |
| [SRC-DB-013] | `fade_in_point` | INTEGER (i64) | Fade-in completion point (ticks) |
| [SRC-DB-014] | `fade_out_point` | INTEGER (i64) | Fade-out start point (ticks) |
| [SRC-DB-015] | `lead_in_point` | INTEGER (i64) | Lead-in end point (ticks) |
| [SRC-DB-016] | `lead_out_point` | INTEGER (i64) | Lead-out start point (ticks) |

#### Requirement ID Categories

- **Scope:** [SRC-SC-010]
- **Problem Statement:** [SRC-PROB-010] through [SRC-PROB-040] (4 IDs)
- **Solution:** [SRC-SOL-010] through [SRC-SOL-030] (3 IDs)
- **Sample Rates:** [SRC-RATE-010] through [SRC-RATE-021] (12 IDs)
- **Tick Rate:** [SRC-TICK-010] through [SRC-TICK-040] (4 IDs)
- **Conversion:** [SRC-CONV-010] through [SRC-CONV-050] (5 IDs)
- **Time:** [SRC-TIME-010], [SRC-TIME-020] (2 IDs)
- **Precision:** [SRC-PREC-010] through [SRC-PREC-040] (4 IDs)
- **Database:** [SRC-DB-010] through [SRC-DB-020] (8 IDs)
- **API:** [SRC-API-010] through [SRC-API-050] (5 IDs)
- **Working Sample Rate:** [SRC-WSR-010] through [SRC-WSR-050] (5 IDs)
- **Coexistence:** [SRC-COEX-010], [SRC-COEX-020] (2 IDs)
- **Implementation:** [SRC-IMPL-010] through [SRC-IMPL-040] (4 IDs)
- **Examples:** [SRC-EXAM-010] through [SRC-EXAM-030] (3 IDs)

---

## Linking Patterns

### Pattern 1: Parameter Reference

**When:** Referencing an operating parameter defined in SPEC016

**Format:**
```markdown
The working sample rate (see [DBD-PARAM-020]) defaults to 44.1kHz.
```

**Example in context:**
```markdown
All decoded audio is converted to the working_sample_rate ([DBD-PARAM-020]
from SPEC016) before buffering, ensuring consistent mixer operation.
```

### Pattern 2: Concept Deep Link

**When:** Referencing a major architectural component or concept

**Format:**
```markdown
For complete decoder-buffer chain architecture, see
[SPEC016](SPEC016-decoder_buffer_design.md#decoder-buffer-chain).
```

**Example in context:**
```markdown
The playback engine uses a decoder-buffer chain architecture
(see [SPEC016 - Decoder-Buffer Chain](SPEC016-decoder_buffer_design.md#decoder-buffer-chain))
where each passage gets a dedicated pipeline: Decoder → Resampler → Fade → Buffer.
```

### Pattern 3: Formula Reference

**When:** Citing a conversion formula defined in SPEC017

**Format:**
```markdown
Sample-to-tick conversion uses [SRC-CONV-030]:
ticks = samples × (28,224,000 ÷ sample_rate)
```

**Example in context:**
```markdown
Passage timing points are converted from sample counts to ticks using the formula
defined in [SRC-CONV-030]: `ticks = samples × (28,224,000 ÷ sample_rate)`. This
ensures sample-accurate timing across all supported audio sample rates.
```

### Pattern 4: Cross-Reference Between Specs

**When:** Showing how two specifications interact

**Format:**
```markdown
Crossfade timing calculations ([XFD-IMPL-010]) determine WHEN passages overlap.
Mixer implementation ([DBD-MIX-040]) determines HOW overlapping audio is combined.
```

**Example in context:**
```markdown
The crossfade system has two distinct responsibilities:
- **Timing (SPEC002):** [XFD-IMPL-010] calculates when passages should overlap
- **Execution (SPEC016):** [DBD-MIX-040] defines how overlapping audio is mixed

SPEC002 provides the crossfade_duration and passage_b_start_time. SPEC016's mixer
uses these values to perform sample-accurate mixing of the two audio streams.
```

### Pattern 5: Database Schema Reference

**When:** Linking database implementation to tick system design

**Format:**
```markdown
Passage timing fields are stored as INTEGER tick values per [SRC-DB-010]:
- start_time ([SRC-DB-011])
- end_time ([SRC-DB-012])
- fade_in_point ([SRC-DB-013])
```

**Example in context:**
```markdown
The `passages` table (see IMPL001) stores timing points as INTEGER tick values:
- `start_time` ([SRC-DB-011]): Passage start boundary
- `end_time` ([SRC-DB-012]): Passage end boundary
- `fade_in_point` ([SRC-DB-013]): Fade-in completion point

These INTEGER columns store ticks (1/28,224,000 second) as defined in
[SPEC017 - Tick System](SPEC017-sample_rate_conversion.md#tick-system).
```

### Pattern 6: Implementation Guidance

**When:** Providing implementation instructions based on spec

**Format:**
```markdown
**Implementation Note:** Decoder priority scheduling follows [DBD-PARAM-060]:
re-evaluate decode job priorities every decode_work_period (default: 5000ms).
```

**Example in context:**
```markdown
**Decoder Pool Scheduling:**

The decoder pool scheduler must:
1. Pause current decoder every `decode_work_period` ([DBD-PARAM-060], default: 5000ms)
2. Evaluate all pending decode jobs for priority
3. Resume highest-priority job (may be same as paused job)

See [SPEC016 - Decode Work Period](SPEC016-decoder_buffer_design.md#decode-work-period)
for complete scheduling specification.
```

---

## Glossary Entries

Use these standardized definitions when creating glossary sections in other documents:

### Major Concepts

**working_sample_rate** ([DBD-PARAM-020])
Internal sample rate for all mixing operations (default: 44100Hz). All decoded audio is converted to this rate before buffering, ensuring consistent mixer operation regardless of source file sample rate.

**tick** ([SRC-TICK-030])
Universal time unit, 1/28,224,000 second (~35.4 nanoseconds). The tick system enables sample-accurate timing across all supported audio sample rates by using the LCM of those rates.

**tick_rate** ([SRC-TICK-020])
28,224,000 Hz - The Least Common Multiple (LCM) of all supported audio sample rates (8kHz, 11.025kHz, 16kHz, 22.05kHz, 32kHz, 44.1kHz, 48kHz, 88.2kHz, 96kHz, 176.4kHz, 192kHz).

**decoder-buffer chain** ([DBD-OV-040])
Pipeline architecture for audio processing: Decoder → Resampler → Fade In/Out Handler → Buffer → Mixer → Output. Each passage in the queue gets a dedicated chain when within `maximum_decode_streams` of the "now playing" position.

**backpressure** ([DBD-BUF-050])
Flow control mechanism using buffer headroom. When a playout buffer has ≤ `playout_ringbuffer_headroom` samples of free space, the decoder pauses until more space is available. Prevents buffer overflow and maintains sample-accurate timing.

**playout_ringbuffer** ([DBD-PARAM-070])
Per-passage decoded audio buffer holding `playout_ringbuffer_size` stereo samples (default: 661,941 samples = 15.01 seconds @ 44.1kHz). Each decoder-buffer chain has its own ring buffer.

**output_ringbuffer** ([DBD-PARAM-030])
Buffer between mixer and cpal output library, holding `output_ringbuffer_size` stereo samples (default: 8192 samples = 185ms @ 44.1kHz). Refilled every `output_refill_period` (default: 90ms).

**sample-accurate timing** ([SRC-SOL-030])
Guarantee that any sample boundary from any supported sample rate can be represented exactly as an integer number of ticks with zero conversion error.

---

## Integration Points

### SPEC016 ↔ SPEC002: Mixer Execution ↔ Crossfade Timing

**Relationship:** SPEC002 calculates WHEN passages overlap; SPEC016 defines HOW overlapping audio is mixed.

**SPEC002 provides:**
- Crossfade duration (ticks)
- Passage B start time (ticks)
- Fade curve types (exponential, cosine, linear)

**SPEC016 uses:**
- [DBD-MIX-040]: Mixer takes samples from both passages during overlap
- [DBD-MIX-040]: Applies volume levels calculated by SPEC002 fade curves
- [DBD-MIX-040]: Combines passages: `(audio_a × volume_a) + (audio_b × volume_b)`

**Linking example:**
```markdown
During crossfade overlap ([XFD-IMPL-010] from SPEC002), the mixer ([DBD-MIX-040]
from SPEC016) reads samples from both the "now playing" and "playing next" buffers,
applies their respective volume curves, and adds the results.
```

### SPEC017 ↔ IMPL001: Tick Storage ↔ Database Schema

**Relationship:** SPEC017 defines tick-based timing storage format; IMPL001 implements as INTEGER columns.

**SPEC017 defines:**
- [SRC-DB-011] through [SRC-DB-016]: Six tick-based timing fields
- INTEGER (i64) storage type
- NULL semantics (use global defaults)

**IMPL001 implements:**
- `passages` table with INTEGER columns
- CHECK constraints on timing point ordering
- NULL handling for default values

**Linking example:**
```markdown
The `passages.start_time` column (IMPL001) stores tick values as defined in
[SRC-DB-011]: INTEGER representing ticks from file start. NULL indicates use
of file beginning (0 ticks).
```

### SPEC016 ↔ SPEC017: Working Sample Rate ↔ Tick-to-Sample Conversion

**Relationship:** SPEC016 defines working_sample_rate parameter; SPEC017 defines conversion from ticks to samples at that rate.

**SPEC016 defines:**
- [DBD-PARAM-020]: working_sample_rate (default: 44100 Hz)
- All mixing operates at this sample rate
- Resampler converts to this rate ([DBD-RSMP-010])

**SPEC017 defines:**
- [SRC-WSR-030]: Formula for tick-to-sample conversion at working rate
- [SRC-WSR-040]: Optimized formula for 44.1kHz (most common case)
- [SRC-WSR-050]: Buffer operations use sample counts, not ticks

**Linking example:**
```markdown
When loading passage timing points from database (ticks), the decoder-buffer chain
converts them to sample counts using [SRC-WSR-030]:
`samples = (ticks × working_sample_rate) ÷ 28,224,000`.

For the default working_sample_rate ([DBD-PARAM-020] = 44100 Hz), this simplifies
to [SRC-WSR-040]: `samples = ticks ÷ 640`.
```

### SPEC016 ↔ SPEC013/SPEC014: Authoritative Design ↔ Implementation Docs

**Relationship:** SPEC016 is authoritative for decoder-buffer design; SPEC013/SPEC014 describe implementation and rationale.

**Authority hierarchy:**
1. **SPEC016 (authoritative):** WHAT the decoder-buffer chain must do
2. **SPEC013 (implementation):** HOW the single-stream architecture implements it
3. **SPEC014 (rationale):** WHY design decisions were made

**When conflicting:**
- SPEC016 takes precedence
- Update SPEC013/SPEC014 to match SPEC016
- If SPEC016 needs change, use formal change control (Tier 2 design change)

**Linking example:**
```markdown
The single-stream playback architecture (SPEC013) implements the decoder-buffer
chain design specified in [SPEC016 - Decoder Buffer Design]. If there are
discrepancies, SPEC016 is authoritative.

See [SPEC016 - Operating Parameters](SPEC016-decoder_buffer_design.md#operating-parameters)
for authoritative parameter definitions.
```

### SPEC017 ↔ SPEC002: Tick System ↔ Crossfade Calculations

**Relationship:** SPEC017 defines universal time units; SPEC002 uses ticks for all crossfade timing.

**SPEC017 provides:**
- [SRC-TICK-030]: Tick definition (1/28,224,000 second)
- [SRC-CONV-030]: Sample-to-tick conversion
- [SRC-DB-010]: Tick storage format

**SPEC002 uses:**
- [XFD-IMPL-010]: Crossfade calculations in ticks
- [XFD-DB-010]: Timing point storage in ticks
- [XFD-EXAM-020]: Example crossfade calculations in ticks

**Linking example:**
```markdown
Crossfade duration is calculated in ticks ([SRC-TICK-030] from SPEC017):

```pseudocode
crossfade_duration_ticks = min(passage_a_lead_out_duration_ticks,
                               passage_b_lead_in_duration_ticks)
```

This ensures sample-accurate crossfades regardless of source sample rates.
```

---

## Recommended Anchor IDs

### For Future SPEC016 Enhancement

When adding HTML anchors to SPEC016 markdown:

**Operating Parameters:**
- `#working-sample-rate` ([DBD-PARAM-020])
- `#output-ringbuffer-size` ([DBD-PARAM-030])
- `#output-refill-period` ([DBD-PARAM-040])
- `#maximum-decode-streams` ([DBD-PARAM-050])
- `#decode-work-period` ([DBD-PARAM-060])
- `#playout-ringbuffer-size` ([DBD-PARAM-070])
- `#playout-ringbuffer-headroom` ([DBD-PARAM-080])
- `#pause-decay-factor` ([DBD-PARAM-090])
- `#pause-decay-floor` ([DBD-PARAM-100])
- `#operating-parameters` (section header)

**Components:**
- `#decoder-buffer-chain` ([DBD-OV-040])
- `#decoder` ([DBD-DEC-010])
- `#resampler` ([DBD-RSMP-010])
- `#fade-handler` ([DBD-FADE-010])
- `#buffer` ([DBD-BUF-010])
- `#mixer` ([DBD-MIX-010])
- `#output` ([DBD-OUT-010])
- `#backpressure` ([DBD-BUF-050])

**Requirement ID Anchors:**
Each [DBD-XXX-NNN] should map to lowercase `#dbd-xxx-nnn` for direct linking.

### For Future SPEC017 Enhancement

When adding HTML anchors to SPEC017 markdown:

**Tick System:**
- `#tick-system` (section header)
- `#tick-rate` ([SRC-TICK-020])
- `#tick-duration` ([SRC-TICK-030])

**Conversion Formulas:**
- `#ticks-per-sample` ([SRC-CONV-010])
- `#samples-to-ticks` ([SRC-CONV-030])
- `#ticks-to-seconds` ([SRC-TIME-010])
- `#api-to-database` ([SRC-API-020])
- `#database-to-api` ([SRC-API-030])
- `#ticks-to-samples-working` ([SRC-WSR-030])
- `#ticks-to-samples-44100` ([SRC-WSR-040])

**Concepts:**
- `#working-sample-rate-conversion` ([SRC-WSR-010])
- `#timing-coexistence` ([SRC-COEX-010])
- `#supported-sample-rates` ([SRC-RATE-010])

**Requirement ID Anchors:**
Each [SRC-XXX-NNN] should map to lowercase `#src-xxx-nnn` for direct linking.

---

## Cross-Reference Examples

### Example 1: Crossfade Implementation (SPEC002 → SPEC016/SPEC017)

**Context:** SPEC002 describes crossfade timing algorithm

```markdown
## Crossfade Timing Calculation

When Passage A is playing and Passage B is queued next:

**Step 1: Convert timing points to universal ticks**

Using [SRC-CONV-030] from SPEC017:
```pseudocode
passage_a_lead_out_ticks = (end_time - lead_out_point) × (28,224,000 ÷ sample_rate_a)
passage_b_lead_in_ticks = lead_in_point × (28,224,000 ÷ sample_rate_b)
```

**Step 2: Calculate crossfade duration**

```pseudocode
crossfade_duration_ticks = min(passage_a_lead_out_ticks, passage_b_lead_in_ticks)
```

**Step 3: Convert to working sample rate for mixer**

The mixer ([DBD-MIX-040] from SPEC016) operates at working_sample_rate
([DBD-PARAM-020], default: 44100 Hz). Convert crossfade duration:

```pseudocode
crossfade_duration_samples = (crossfade_duration_ticks × 44100) ÷ 28,224,000
                            = crossfade_duration_ticks ÷ 640  // Optimized [SRC-WSR-040]
```

**Step 4: Mixer execution**

During crossfade, the mixer ([DBD-MIX-040]) reads samples from both playout
buffers ([DBD-PARAM-070]) and combines them according to their volume curves.
```

### Example 2: Database Schema (IMPL001 → SPEC017)

**Context:** IMPL001 describes `passages` table schema

```markdown
### `passages` Table

Timing Point Storage (Tick-Based System):

The `passages` table stores all timing points as INTEGER tick values, where one
tick = 1/28,224,000 second ([SRC-TICK-030] from SPEC017). This enables
sample-accurate timing across all supported audio sample rates.

| Column | Type | Description | SPEC017 Reference |
|--------|------|-------------|-------------------|
| start_time | INTEGER | Passage start boundary (ticks from file start) | [SRC-DB-011] |
| end_time | INTEGER | Passage end boundary (ticks from file start) | [SRC-DB-012] |
| fade_in_point | INTEGER | Fade-in completion point (ticks) | [SRC-DB-013] |
| fade_out_point | INTEGER | Fade-out start point (ticks) | [SRC-DB-014] |
| lead_in_point | INTEGER | Lead-in end point (ticks) | [SRC-DB-015] |
| lead_out_point | INTEGER | Lead-out start point (ticks) | [SRC-DB-016] |

**NULL Semantics:** NULL values indicate use of global defaults. See [SRC-DB-020]
for complete NULL handling specification.

**API Conversion:** When exposed via REST API, tick values are converted to
milliseconds using [SRC-API-030]: `milliseconds = ticks ÷ 28,224` (rounded).
```

### Example 3: Implementation Guide (wkmp-ap module docs → SPEC016)

**Context:** Developer documentation for audio player module

```markdown
## Decoder Pool Configuration

The decoder pool manages concurrent audio decoding operations. Configuration
parameters are defined in [SPEC016 - Operating Parameters]:

| Setting | Config Key | Default | Reference |
|---------|-----------|---------|-----------|
| Max concurrent decoders | `maximum_decode_streams` | 12 | [DBD-PARAM-050] |
| Priority re-evaluation | `decode_work_period` | 5000 ms | [DBD-PARAM-060] |

### Decoder Scheduling Algorithm

Every `decode_work_period` ([DBD-PARAM-060]), the scheduler:

1. Pauses the currently running decoder
2. Evaluates all pending decode jobs for priority (position in queue)
3. Resumes the highest-priority job
4. If the paused decoder is still highest priority, it continues immediately

See [SPEC016 - Decoders](SPEC016-decoder_buffer_design.md#decoders) for complete
specification, particularly [DBD-DEC-040] (serial decode for cache coherency).

### Buffer Management

Each decoder-buffer chain ([DBD-OV-040]) has:
- **Playout buffer:** [DBD-PARAM-070] = 661,941 samples (15.01s @ 44.1kHz)
- **Headroom reserve:** [DBD-PARAM-080] = 441 samples (0.01s @ 44.1kHz)

When buffer has ≤ headroom samples free, decoder pauses ([DBD-BUF-050] - backpressure).
```

### Example 4: API Documentation (SPEC007 → SPEC017)

**Context:** SPEC007 describes REST API endpoints

```markdown
## POST /playback/enqueue

Enqueue a passage for playback.

**Request Body (JSON):**

```json
{
  "file_path": "albums/song.mp3",
  "start_time_ms": 0,
  "end_time_ms": 234500,
  "fade_in_point_ms": 2000,
  "fade_out_point_ms": 232500,
  "lead_in_point_ms": 5000,
  "lead_out_point_ms": 229500
}
```

**Timing Value Conversion:**

All timing values are unsigned integer milliseconds. The API layer converts them
to ticks for database storage using [SRC-API-020]:

```
ticks = milliseconds × 28,224
```

**Example:** `fade_in_point_ms: 2000` → `fade_in_point: 56,448,000 ticks`

**Database Storage:** Converted tick values are stored in the `passages` table
as INTEGER fields per [SRC-DB-010] through [SRC-DB-016].

**Retrieval Conversion:** When reading from database for API responses, ticks
are converted to milliseconds using [SRC-API-030]:

```
milliseconds = ticks ÷ 28,224  (rounded to nearest integer)
```

See [SPEC017 - API Representation](SPEC017-sample_rate_conversion.md#api-representation)
for complete conversion specification.
```

---

## Best Practices

### When to Use Requirement IDs vs Document Links

**Use Requirement IDs ([DBD-XXX-NNN], [SRC-XXX-NNN]):**
- ✅ When citing a specific requirement within a paragraph
- ✅ When multiple requirements apply to a concept
- ✅ For traceability in code comments
- ✅ In validation documents and test plans

**Use Document Links:**
- ✅ When directing readers to a section for complete context
- ✅ For conceptual overviews spanning multiple requirements
- ✅ In user-facing documentation (less technical)
- ✅ When anchors provide better navigation

**Combine both:**
- ✅ Best practice: Include both requirement ID and document link
  ```markdown
  The working sample rate ([DBD-PARAM-020] from
  [SPEC016](SPEC016-decoder_buffer_design.md#working-sample-rate)) defaults to 44.1kHz.
  ```

### How to Cite Operating Parameters

**Pattern:**
```markdown
Parameter_name ([REQUIREMENT-ID], default: value)
```

**Examples:**
```markdown
- working_sample_rate ([DBD-PARAM-020], default: 44100 Hz)
- maximum_decode_streams ([DBD-PARAM-050], default: 12)
- playout_ringbuffer_size ([DBD-PARAM-070], default: 661,941 samples)
```

**In prose:**
```markdown
All decoded audio is converted to the working_sample_rate ([DBD-PARAM-020],
default: 44100 Hz) before buffering.
```

### How to Reference Formulas vs Implementing Them

**Referencing (Design/Architecture docs):**
```markdown
Sample-to-tick conversion uses [SRC-CONV-030]:
`ticks = samples × (28,224,000 ÷ sample_rate)`.
```

**Implementing (Code/Implementation docs):**
```markdown
**Sample-to-Tick Conversion Implementation:**

Per [SRC-CONV-030] from SPEC017:

```rust
/// Convert sample count to ticks
///
/// Implements [SRC-CONV-030]: ticks = samples × (28,224,000 ÷ sample_rate)
fn samples_to_ticks(samples: i64, sample_rate: u32) -> i64 {
    const TICK_RATE: i64 = 28_224_000;
    samples * (TICK_RATE / sample_rate as i64)
}
```
```

**Key distinction:**
- **Reference:** Cite the requirement ID and formula
- **Implement:** Show the requirement ID, formula, AND code

### Best Practices for Bidirectional Cross-References

**Problem:** SPEC016 and SPEC002 both describe crossfading - how to link without circular dependency?

**Solution:** Clearly distinguish WHAT vs HOW:

**In SPEC002 (Crossfade Timing):**
```markdown
## Crossfade Execution

This specification defines WHEN passages overlap (timing calculations).
For HOW overlapping audio is mixed, see [SPEC016 - Mixer](SPEC016-decoder_buffer_design.md#mixer).

Crossfade timing algorithm ([XFD-IMPL-010]) calculates:
- crossfade_duration (ticks)
- passage_b_start_time (ticks)

These values are passed to the mixer ([DBD-MIX-040] from SPEC016), which performs
the actual audio mixing.
```

**In SPEC016 (Mixer Implementation):**
```markdown
## Mixer

The mixer ([DBD-MIX-040]) implements playback and pause modes, volume control,
and crossfade execution.

**Crossfade Inputs:** The mixer receives crossfade timing from
[SPEC002 - Crossfade Timing](SPEC002-crossfade.md#crossfade-start-calculation):
- crossfade_duration
- passage_b_start_time
- fade curve types

**Mixer Behavior:** When in crossfade overlap:
- Read samples from both "now playing" and "playing next" buffers
- Apply volume curves (calculated per SPEC002 fade formulas)
- Mix: `(audio_a × volume_a) + (audio_b × volume_b)`
```

**Key principles:**
1. Each spec has **primary authority** over its domain
2. Cross-references clarify **boundaries** ("see X for timing")
3. Use **asymmetric linking** (design → design is OK; avoid impl → design)

### When to Update This Guide

This linking guide should be updated when:

- ✅ New requirement IDs added to SPEC016 or SPEC017
- ✅ New operating parameters defined
- ✅ New conversion formulas added
- ✅ New integration points with other specs discovered
- ✅ Anchors are added to SPEC016/SPEC017 markdown (update recommendations)
- ✅ Linking patterns evolve based on usage

Update process:
1. Update `phase3-linking-guide.json` (data)
2. Update this markdown guide (documentation)
3. Notify all document maintainers of new linkable concepts

---

## Summary Statistics

**Total Linkable Concepts:** 129
- SPEC016: 72 concepts (63 requirement IDs + 9 parameters)
- SPEC017: 57 concepts (50 requirement IDs + 7 formulas)

**Operating Parameters:** 9 (all in SPEC016)

**Conversion Formulas:** 7 (all in SPEC017)

**Component Definitions:** 8 (decoder, resampler, fade handler, buffer, mixer, output, backpressure, decoder-buffer chain)

**Database Fields:** 6 tick-based timing fields (SPEC017 → IMPL001)

**Glossary Terms:** 8 major concepts defined

**Integration Points:** 6 documented cross-spec relationships

**Recommended Anchors:** 27 total (15 for SPEC016, 12 for SPEC017)

---

## Version History

**Version 1.0** (2025-10-19)
- Initial comprehensive linking guide
- Cataloged all 129 linkable concepts from SPEC016/SPEC017
- Defined 6 linking patterns with examples
- Created 8 glossary entries for major concepts
- Documented 6 integration points with other specs
- Provided 27 anchor ID recommendations
- Included 4 detailed cross-reference examples

**Maintained By:** Documentation lead, technical lead

---

**End of SPEC016/SPEC017 Linking Guide**
