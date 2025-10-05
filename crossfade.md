# Crossfade Design

> **Related Documentation:** [Requirements](requirements.md) | [Architecture](architecture.md)

## Overview

McRhythm supports sophisticated crossfading between passages, allowing smooth transitions with configurable fade curves and overlap timing. Each passage has six timing points that control how it begins, ends, and overlaps with adjacent passages.

## Timing Points

Each passage has six timing points defined relative to the audio file:

```
Start    Fade-In   Lead-In            Lead-Out   Fade-Out    End
  |         |         |                    |         |         |
  |---------|---------|--------------------|---------|---------|
  |         |         |                    |         |         |
  0%        |         |                    |         |        100%
       Fade-In   Lead-In Time         Lead-Out  Fade-Out
        Time                            Time      Time
```

### Point Definitions

1. **Start Time**: Beginning of the passage audio (offset in file)
2. **Fade-In Point**: When audio reaches full volume after fading in
3. **Lead-In Point**: When the passage is considered "fully playing" for crossfade purposes
4. **Lead-Out Point**: When the passage begins its crossfade overlap with the next passage
5. **Fade-Out Point**: When audio begins fading out
6. **End Time**: End of the passage audio (offset in file)

### Time Intervals

- **Fade-In Time** = Fade-In Point - Start Time (default: 0)
- **Lead-In Time** = Lead-In Point - Start Time (default: 0)
- **Lead-Out Time** = End Time - Lead-Out Point (default: 0)
- **Fade-Out Time** = End Time - Fade-Out Point (default: 0)

### Constraints

- Start ≤ Fade-In ≤ Lead-In ≤ Lead-Out ≤ Fade-Out ≤ End
- Fade-In Point ≤ Fade-Out Point
- Lead-In Point ≤ Lead-Out Point
- All times may be equal (resulting in 0-duration intervals)

## Fade Curves

Each passage can independently configure its fade-in and fade-out curves:

### Exponential In / Logarithmic Out
- **Fade-In**: Volume increases exponentially (slow start, fast finish)
- **Fade-Out**: Volume decreases logarithmically (fast start, slow finish)
- Best for: Natural-sounding transitions

### Cosine (S-Curve)
- Smooth acceleration and deceleration
- Best for: Gentle, musical transitions

### Linear
- Constant rate of change
- Best for: Precise, predictable crossfades

## Crossfade Behavior

### Case 1: Following Passage Has Longer Lead-In Time

When `Lead-Out Time of Passage A ≤ Lead-In Time of Passage B`:

```
Passage A: |---------------------------|********|
                                       Lead-Out↑

Passage B:          |********|-------------------|
                    ↑Start   ↑Lead-In

Timeline:  |-----------------------------|----|
           A playing alone               Both playing
```

**Timing**: Passage B starts at its Start Time when Passage A reaches its Lead-Out Point.

**Example**:
- Passage A: Lead-Out Time = 3 seconds
- Passage B: Lead-In Time = 5 seconds
- Result: Passage B starts 3 seconds before Passage A ends (they overlap for 3 seconds)

### Case 2: Following Passage Has Shorter Lead-In Time

When `Lead-Out Time of Passage A > Lead-In Time of Passage B`:

```
Passage A: |---------------------------|********|
                                           Lead-Out↑

Passage B:                      |**|-----------------|
                                ↑Start ↑Lead-In

Timeline:  |--------------------------|-----|
           A playing alone            Both playing
```

**Timing**: Passage B starts at its Start Time when Passage A has `Lead-In Time of B` remaining before its End Time.

**Example**:
- Passage A: Lead-Out Time = 5 seconds, currently at time point where 2 seconds remain
- Passage B: Lead-In Time = 2 seconds
- Result: Passage B starts now (they overlap for 2 seconds)

### Case 3: No Overlap (Zero Lead Times)

When both Lead-In Time and Lead-Out Time are 0:

```
Passage A: |---------------------------|
                                       ↑End

Passage B:                             |-----------------|
                                       ↑Start

Timeline:  |---------------------------||-----------------|
           A playing                   B playing
```

**Timing**: Passage B starts immediately when Passage A ends (no gap, no overlap).

## Fade Behavior During Crossfade

Fades operate independently of crossfade overlap:

### Example: Crossfade with Fades

```
Passage A has:
- Lead-Out Time: 4s
- Fade-Out Time: 2s (starts 2s before end)

Passage B has:
- Lead-In Time: 3s
- Fade-In Time: 1s (ends 1s after start)

Timeline:
Passage A: |---------------|FF|
                           ↑Fade-Out starts (2s before end)
                              ↑End

Passage B:          |F|---------|
                    ↑Start
                     ↑Fade-In ends (1s after start)

Combined:   |---------------|FF|
                        |F|---------|
                        ↑B starts (4s before A ends)
```

- Passage B starts 4 seconds before Passage A ends (when A reaches Lead-Out)
- Passage A fades out during its final 2 seconds
- Passage B fades in during its first 1 second
- Both fades occur independently, mixing the two audio streams

## Volume Calculations

During crossfade overlap, audio streams are mixed:

```
Final Output = (Passage A Audio × A Volume) + (Passage B Audio × B Volume)
```

Where volume at any time `t` is determined by:
- Current passage fade state (fade-in/fade-out curve)
- Resume-from-pause fade (0.5s exponential ramp, if applicable)
- User-controlled master volume

### Resume from Pause

When resuming from Pause:
- 0.5 second exponential volume ramp from 0% to 100%
- Applied AFTER any crossfade calculations
- Affects all playing passages simultaneously

## Default Configuration

New passages are created with:
- Start Time: Beginning of file (or user-defined offset)
- Fade-In Point: = Start Time (Fade-In Time = 0)
- Lead-In Point: = Start Time (Lead-In Time = 0)
- Lead-Out Point: = End Time (Lead-Out Time = 0)
- Fade-Out Point: = End Time (Fade-Out Time = 0)
- End Time: End of file (or user-defined offset)
- Fade-In Curve: Exponential
- Fade-Out Curve: Logarithmic

## Database Storage

Passage table stores:
- `start_time` (seconds)
- `fade_in_point` (seconds)
- `lead_in_point` (seconds)
- `lead_out_point` (seconds)
- `fade_out_point` (seconds)
- `end_time` (seconds)
- `fade_in_curve` (enum: exponential, cosine, linear)
- `fade_out_curve` (enum: logarithmic, cosine, linear)

## User Interface Considerations

### Visual Editor
Should display:
- Waveform with six draggable markers
- Preview of crossfade overlap with adjacent passages
- Real-time audio preview of crossfade region

### Validation
- Enforce temporal constraints when user moves markers
- Warn if Lead-Out Time > remaining passage duration
- Suggest sensible defaults based on passage characteristics

## Edge Cases

### First Passage in Queue
- No previous passage to crossfade from
- Fade-in and Lead-in apply if defined
- If Lead-In > 0, passage may start partially silent/faded

### Last Passage in Queue
- No next passage to crossfade to
- Fade-out applies if defined
- Passage plays to End Time

### User Skip During Crossfade
- Both passages stop immediately
- Next passage begins according to its own timing rules

### Pause During Crossfade
- Both passages pause at current position
- Resume applies 0.5s ramp to both passages
- Crossfade relationship preserved

### Queue Modification During Crossfade
- If next passage removed: current passage plays to completion
- If current passage removed: next passage treated as "first passage in queue"
