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
       Fade-In   Lead-In Point        Lead-Out  Fade-Out
        Point                           Point     Point
```
Fade-In may be equal to or after Lead-In,
Fade-Out may be equal to or after Lead-Out:
```
Start    Lead-In   Fade-In            Fade-Out   Lead-Out    End
  |         |         |                    |         |         |
  |---------|---------|--------------------|---------|---------|
  |         |         |                    |         |         |
  0%        |         |                    |         |        100%
       Lead-In   Fade-In Point        Fade-Out  Lead-Out
        Point                           Point     Point
```
Fade-In / Fade-Out soften the passage start / end volume profiles, for example when taking a passage
from the middle of a continuous piece of music.

Lead-In / Lead-Out define portions of the passage where it is permissible / non-intrusive for a crossfade
operation to take place.

### Point Definitions

Start time, end time, and all points each define a time in the audio file:

1. **Start Time**: Beginning of the passage audio
2. **Fade-In Point**: When audio reaches full volume after fading in
3. **Lead-In Point**: The time point by which the previous passage must have ended. The
                      system enforces this by calculating crossfade start timing based on Lead-In and
                      Lead-Out Durations (see Crossfade Behavior cases).
4. **Lead-Out Point**: When the next passage may start playing for crossfade
5. **Fade-Out Point**: When audio begins fading out
6. **End Time**: End of the passage audio

### Durations

- **Fade-In Duration** = Fade-In Point - Start Time (default: 0)
- **Lead-In Duration** = Lead-In Point - Start Time (default: 0)
- **Lead-Out Duration** = End Time - Lead-Out Point (default: 0)
- **Fade-Out Duration** = End Time - Fade-Out Point (default: 0)

### Constraints

- Start ≤ Fade-In ≤ Fade-Out ≤ End
- Start ≤ Lead-In ≤ Lead-Out ≤ End
- All times may be equal (resulting in 0-duration intervals)
- Fade-In and Fade-Out points do not restrict Lead-In and Lead-Out points
- Lead-In and Lead-Out points do not restrict Fade-In and Fade-Out points

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

### Case 1: Following Passage Has Longer Lead-In Duration

When `Lead-Out Duration of Passage A ≤ Lead-In Duration of Passage B`:

```
Passage A: |---------------------------|******|
                         Lead-Out Point↑   End↑

Passage B:                             |*********|-------------------|
                                       ↑Start    ↑Lead-In Point

Timeline:  |---------------------------|------|------------------------|
           A playing alone             Both playing
                                              B playing alone
```

**Timing**: Passage B starts at its Start Time when Passage A reaches its Lead-Out Point.

**Example**:
- Passage A: Lead-Out Duration = 3 seconds
- Passage B: Lead-In Duration = 5 seconds
- Result: Passage B starts 3 seconds before Passage A ends (they overlap for 3 seconds)

### Case 2: Following Passage Has Shorter Lead-In Duration

When `Lead-Out Duration of Passage A > Lead-In Duration of Passage B`:

```
Passage A: |---------------------------|*************|
                         Lead-Out Point↑          End↑

Passage B:                                   |*******|-----------------|
                                             ↑Start  ↑Lead-In Point

Timeline:  |---------------------------------|-------|-----------------|
           A playing alone                   Both playing
                                                     B playing alone
```

**Timing**: Passage B starts at its Start Time when Passage A has `Lead-In Duration of B` remaining before its End Time.

**Example**:
- Passage A: Lead-Out Duration = 5 seconds, currently at time point where 2 seconds remain
- Passage B: Lead-In Duration = 2 seconds
- Result: Passage B starts now (they overlap for 2 seconds)

### Case 3: No Overlap (Zero Lead Durations)

When both Lead-In Duration and Lead-Out Duration are 0:

```
Passage A: |---------------------------|
                                       ↑End

Passage B:                             |-----------------|
                                       ↑Start

Timeline:  |---------------------------|------------------|
           A playing                   B playing
```

**Timing**: Passage B starts immediately when Passage A ends (no gap, no overlap).

## Fade Behavior During Crossfade

Fades operate independently of crossfade overlap

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
- Fade-In Point: = Start Time (Fade-In Duration = 0)
- Lead-In Point: = Start Time (Lead-In Duration = 0)
- Lead-Out Point: = End Time (Lead-Out Duration = 0)
- Fade-Out Point: = End Time (Fade-Out Duration = 0)
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
- Warn if any duration > 50% of passage duration
- Suggest sensible defaults based on passage characteristics

## Edge Cases

### First Passage in Queue
- No previous passage to crossfade from
- Fade-in and Lead-in apply if defined
- If Fade-In Duration > 0, passage starts at zero volume and fades in

### Last Passage in Queue
- No next passage to crossfade to
- Fade-out applies if defined
- Passage plays to End Time

### User Skip During Crossfade
- Both passages stop immediately
- Next passage from the queue begins according to its own timing rules

### Pause During Crossfade
- Both passages pause at current position
- Resume applies 0.5s ramp to both passages
- Crossfade relationship preserved

### Queue Modification During Crossfade
- If next passage removed: current passage plays to completion
- If current passage removed: next passage treated as "first passage in queue"

