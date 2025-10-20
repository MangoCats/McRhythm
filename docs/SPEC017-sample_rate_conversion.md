# Sample Rate Conversion Design

**🗂️ TIER 2 - DESIGN SPECIFICATION**

Defines HOW sample rate conversion and time representation are handled using a tick-based system. Derived from [Requirements](REQ001-requirements.md) playback precision requirements. See [Document Hierarchy](GOV001-document_hierarchy.md) and [Requirements Enumeration](GOV002-requirements_enumeration.md).

> **Related Documentation:** [Architecture](SPEC001-architecture.md) | [Decoder Buffer Design](SPEC016-decoder_buffer_design.md) | [Crossfade Design](SPEC002-crossfade.md) | [Database Schema](IMPL001-database_schema.md)

---

## Scope

**[SRC-SC-010]** The concepts described herein apply to timing precision across all WKMP microservices that handle audio timing, passage boundaries, and crossfade calculations.

## Problem Statement

**[SRC-PROB-010]** Audio files exist at many different sample rates. WKMP requires sample-accurate timing precision for passage boundaries and crossfade points across all source sample rates.

**[SRC-PROB-020]** Using floating-point seconds for timing introduces cumulative rounding errors that violate sample-accuracy requirements.

**[SRC-PROB-030]** Using integer sample counts tied to a specific sample rate (e.g., 44.1kHz) creates conversion errors when working with files at other sample rates.

**[SRC-PROB-040]** The system needs a universal time representation that can exactly express any sample boundary from any supported sample rate.

## Solution: Tick-Based Timing

**[SRC-SOL-010]** WKMP uses a **tick-based timing system** where one "tick" represents a fixed fraction of a second.

**[SRC-SOL-020]** The tick rate is chosen as the **Least Common Multiple (LCM)** of all supported audio sample rates.

**[SRC-SOL-030]** This ensures that any sample boundary from any supported sample rate can be represented exactly as an integer number of ticks.

## Common Audio Sample Rates

**[SRC-RATE-010]** WKMP supports these common audio sample rates:

- **[SRC-RATE-011]** 8,000 Hz (telephony, low-quality)
- **[SRC-RATE-012]** 11,025 Hz (old multimedia, 1/4 CD quality)
- **[SRC-RATE-013]** 16,000 Hz (wideband telephony)
- **[SRC-RATE-014]** 22,050 Hz (half CD quality)
- **[SRC-RATE-015]** 32,000 Hz (miniDV, NTSC DV)
- **[SRC-RATE-016]** 44,100 Hz (CD audio, most common)
- **[SRC-RATE-017]** 48,000 Hz (DAT, DVD, professional video)
- **[SRC-RATE-018]** 88,200 Hz (high-res audio, 2× CD)
- **[SRC-RATE-019]** 96,000 Hz (high-res audio, 2× DAT)
- **[SRC-RATE-020]** 176,400 Hz (ultra high-res, 4× CD)
- **[SRC-RATE-021]** 192,000 Hz (ultra high-res, 4× DAT)

## Tick Rate Calculation

**[SRC-TICK-010]** The tick rate is the Least Common Multiple (LCM) of all supported sample rates:

```
LCM(8000, 11025, 16000, 22050, 32000, 44100, 48000, 88200, 96000, 176400, 192000)
```

**[SRC-TICK-020]** After calculation:

```
LCM = 28,224,000 Hz
```

**[SRC-TICK-030]** Therefore, **one tick = 1/28,224,000 second** ≈ 35.4 nanoseconds.

**[SRC-TICK-040]** This tick rate exactly divides into all supported sample rates with zero remainder.

## Tick-to-Sample Conversion

**[SRC-CONV-010]** For any sample rate, the number of ticks per sample is:

```
ticks_per_sample = 28,224,000 ÷ sample_rate
```

**[SRC-CONV-020]** Common conversions:

| Sample Rate | Ticks per Sample |
|-------------|------------------|
| 8,000 Hz    | 3,528           |
| 11,025 Hz   | 2,560           |
| 16,000 Hz   | 1,764           |
| 22,050 Hz   | 1,280           |
| 32,000 Hz   | 882             |
| 44,100 Hz   | 640             |
| 48,000 Hz   | 588             |
| 88,200 Hz   | 320             |
| 96,000 Hz   | 294             |
| 176,400 Hz  | 160             |
| 192,000 Hz  | 147             |

## Sample-to-Tick Conversion

**[SRC-CONV-030]** To convert sample counts to ticks:

```
ticks = samples × (28,224,000 ÷ sample_rate)
```

**[SRC-CONV-040]** Example: 5 seconds at 44.1kHz

```
samples = 5.0 × 44,100 = 220,500 samples
ticks = 220,500 × 640 = 141,120,000 ticks
```

**[SRC-CONV-050]** These 141,120,000 ticks exactly represent:
- 220,500 samples at 44.1kHz
- 240,000 samples at 48kHz
- 40,000 samples at 8kHz
- All with zero conversion error

## Tick-to-Time Conversion

**[SRC-TIME-010]** To convert ticks to wall-clock seconds:

```
seconds = ticks ÷ 28,224,000
```

**[SRC-TIME-020]** Example: 141,120,000 ticks

```
seconds = 141,120,000 ÷ 28,224,000 = 5.0 seconds (exact)
```

## Precision and Range

**[SRC-PREC-010]** WKMP uses **64-bit signed integers** (`i64`) for tick values.

**[SRC-PREC-020]** Maximum representable time:

```
max_ticks = 2^63 - 1 = 9,223,372,036,854,775,807 ticks
max_seconds = max_ticks ÷ 28,224,000 ≈ 326,869,873 seconds
max_years ≈ 10.36 years
```

**[SRC-PREC-030]** This range is sufficient for all practical audio passage lengths.

**[SRC-PREC-040]** Precision at tick level: ~35.4 nanoseconds per tick (far exceeds audio sampling precision).

## Database Storage

**[SRC-DB-010]** Passage timing fields are stored as **INTEGER** (SQLite `i64`) tick values:

- **[SRC-DB-011]** `start_time` - Passage start boundary (ticks from file start)
- **[SRC-DB-012]** `end_time` - Passage end boundary (ticks from file start)
- **[SRC-DB-013]** `fade_in_point` - Fade-in completion point (ticks from file start)
- **[SRC-DB-014]** `fade_out_point` - Fade-out start point (ticks from file start)
- **[SRC-DB-015]** `lead_in_point` - Lead-in end point (ticks from file start)
- **[SRC-DB-016]** `lead_out_point` - Lead-out start point (ticks from file start)

**[SRC-DB-020]** NULL values indicate use of global defaults (see **[XFD-DEF-020]** Crossfade Design).

## API Representation

**[SRC-API-010]** The REST API uses **milliseconds** (unsigned integers) for human readability:

**[SRC-API-020]** API-to-database conversion:

```
ticks = milliseconds × 28,224
```

**[SRC-API-030]** Database-to-API conversion:

```
milliseconds = ticks ÷ 28,224  (rounded to nearest integer)
```

**[SRC-API-040]** Example API request:

```json
{
  "file_path": "path/to/file.mp3",
  "start_time_ms": 0,
  "end_time_ms": 234500,
  "fade_in_point_ms": 2000,
  "fade_out_point_ms": 232500
}
```

**[SRC-API-050]** Internal storage (after conversion):

```
start_time: 0 ticks
end_time: 6,618,528,000 ticks (234500ms × 28224)
fade_in_point: 56,448,000 ticks (2000ms × 28224)
fade_out_point: 6,562,080,000 ticks (232500ms × 28224)
```

## Working Sample Rate

**[SRC-WSR-010]** WKMP defines a **working_sample_rate** for internal mixing operations ([SPEC016 DBD-PARAM-020](SPEC016-decoder_buffer_design.md#operating-parameters), default: 44,100 Hz).

See [SPEC016 Resampling - DBD-RSMP-010] for resampling behavior when file sample rate != working_sample_rate.

**[SRC-WSR-020]** At the decoder-buffer boundary, timing transitions from tick-based to sample-count-based:

**[SRC-WSR-030]** Tick-to-sample conversion at working_sample_rate:

```
samples = (ticks × working_sample_rate) ÷ 28,224,000
```

**[SRC-WSR-040]** For 44.1kHz working rate:

```
samples = ticks ÷ 640
```

**[SRC-WSR-050]** All buffer positions, mixer calculations, and output operations use sample counts at working_sample_rate (not ticks).

## Timing System Coexistence

**[SRC-COEX-010]** WKMP uses two timing representations that coexist:

1. **Tick-based timing** (universal precision):
   - Database storage
   - API communication
   - Passage boundary definitions
   - Crossfade calculations

2. **Sample-count timing** (at working_sample_rate):
   - Buffer management
   - Mixer operations
   - Output ring buffer positions
   - Real-time playback tracking

**[SRC-COEX-020]** Conversion between systems occurs at decoder-buffer chain initialization when passage timing points are loaded from database.

See [SPEC016 Fade In/Out handlers](SPEC016-decoder_buffer_design.md#fade-inout-handlers) for implementation of tick-to-sample conversion:
- [DBD-FADE-010]: Fade handler receives timing in ticks
- [DBD-FADE-020]: Converts to sample counts at working_sample_rate
- [DBD-FADE-030]: Applies fade curves during decode (pre-buffer)

SPEC017 describes conversion point; SPEC016 implements it.

## Implementation Notes

**[SRC-IMPL-010]** Tick arithmetic uses standard integer operations (addition, subtraction, multiplication, division).

**[SRC-IMPL-020]** All divisions by 28,224,000 result in exact integer quotients when operating on tick-aligned values.

**[SRC-IMPL-030]** Rounding is only needed when:
- Converting ticks to milliseconds for API responses
- Converting ticks to floating-point seconds for logging/display

**[SRC-IMPL-040]** Internal calculations maintain integer precision throughout the decode-buffer-mix pipeline.

## Example: Crossfade Timing

**[SRC-EXAM-010]** Two passages with different source sample rates:

**Passage A:**
- File: 44.1kHz MP3
- Duration: 180 seconds (7,938,000 samples at source rate)
- lead_out_point: 3 seconds before end = 177 seconds
- In ticks: 177s × 28,224,000 = 4,995,648,000 ticks

**Passage B:**
- File: 48kHz FLAC
- Duration: 240 seconds (11,520,000 samples at source rate)
- lead_in_point: 5 seconds after start
- In ticks: 5s × 28,224,000 = 141,120,000 ticks

**[SRC-EXAM-020]** Crossfade calculation (see **[XFD-IMPL-010]** Crossfade Design):

```
crossfade_duration_ticks = min(passage_a_lead_out_duration_ticks, passage_b_lead_in_duration_ticks)
                         = min(3s × 28,224,000, 5s × 28,224,000)
                         = min(84,672,000, 141,120,000)
                         = 84,672,000 ticks (exactly 3 seconds)
```

**[SRC-EXAM-030]** This crossfade duration converts exactly to:
- 132,300 samples at 44.1kHz (Passage A)
- 144,000 samples at 48kHz (Passage B)
- Both with zero conversion error

---

**Document Version:** 1.0
**Created:** 2025-10-19
**Status:** Current
**Tier:** 2 - Design Specification
**Document Code:** SRC (Sample Rate Conversion)

**Change Log:**
- v1.0 (2025-10-19): Initial specification with requirement IDs
  - Renamed from NEW-sample_rate_conversion.md to SPEC017-sample_rate_conversion.md
  - Applied GOV001 tier designation (Tier 2 - Design Specification)
  - Applied GOV002 requirement enumeration scheme (SRC document code)
  - Added all requirement IDs ([SRC-XXX-NNN] format)
  - Added Related Documentation section
  - Added proper metadata and change log
  - Integrated into WKMP documentation hierarchy
  - Expanded with database storage, API representation, and working sample rate sections
  - Added implementation notes and coexistence explanation

**Maintained By:** Audio engineer, technical lead

----
End of document - Sample Rate Conversion Design
