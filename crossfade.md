# Crossfade Design

**🎵 TIER 2 - DESIGN SPECIFICATION**

Defines crossfade timing and behavior. Derived from [requirements.md](requirements.md). See [Document Hierarchy](document_hierarchy.md).

> **Related Documentation:** [Architecture](architecture.md)

---

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

  1. **Start Time**: Beginning of passage audio
  2. **Fade-In Point**: When volume reaches 100%
  3. **Lead-In Point**: Latest time the previous passage may still be playing
  4. **Lead-Out Point**: Earliest time the next passage may start playing
  5. **Fade-Out Point**: When volume begins decreasing
  6. **End Time**: End of passage audio

  - **Lead-In/Lead-Out**: Define when this passage plays simultaneously with adjacent passages
  - **Fade-In/Fade-Out**: Define volume envelope (independent of simultaneous playback)

### Durations

- **Fade-In Duration** = Fade-In Point - Start Time (default: 0)
- **Lead-In Duration** = Lead-In Point - Start Time (default: 0)
- **Lead-Out Duration** = End Time - Lead-Out Point (default: 0)
- **Fade-Out Duration** = End Time - Fade-Out Point (default: 0)

### Constraints

Timing points must satisfy two independent constraint chains:

  **Fade Point Constraints:**
  - Start ≤ Fade-In ≤ Fade-Out ≤ End

  **Lead Point Constraints:**
  - Start ≤ Lead-In ≤ Lead-Out ≤ End

  **Cross-Chain Independence:**
  - Fade-In and Lead-In points are independent (either may come first, or may be equal)
  - Fade-Out and Lead-Out points are independent (either may come first, or may be equal)
  - All timing points may be equal to each other (resulting in 0-duration intervals)

  **Valid Examples:**
  1. Standard ordering (Fade inside Lead, no simultaneous play with adjacent passages):
  Start(0s) = Lead-In(0s) < Fade-In(2s) < Fade-Out(58s) < Lead-Out(60s) = End(60s)
  2. Inverted ordering (Lead inside Fade, no fade of this passage's level in or out):
  Start(0s) = Fade-In(0s) < Lead-In(2s) < Lead-Out(58s) < Fade-Out(60s) = End(60s)
  3. Interleaved ordering:
  Start(0s) < Fade-In(1s) < Lead-In(2s) < Lead-Out(58s) < Fade-Out(59s) < End(60s)
  4. All points equal (passage plays at constant volume, no simultaneous play with adjacent passages):
  Start(0s) = Fade-In(0s) = Lead-In(0s) = Lead-Out(60s) = Fade-Out(60s) = End(60s)
  5. Partial equality:
  Start(0s) = Fade-In(0s) = Lead-In(0s) < Lead-Out(58s) = Fade-Out(58s) < End(60s)

  **Validation Rules:**
  - Both constraint chains must be satisfied independently
  - No additional ordering restrictions exist between Fade and Lead points

  **Semantic Guidance:**

  While any ordering satisfying the constraints is valid, typical usage patterns include:

  - **Fade-In after Lead-In** (Fade-In ≥ Lead-In): Use when passage starts with quiet intro that needs gentle fade-in even after crossfade completes
  - **Lead-In after Fade-In** (Lead-In ≥ Fade-In): Use when passage has abrupt start that benefits from fade-in, but crossfade should end before full volume
  - **Fade-Out before Lead-Out** (Fade-Out ≤ Lead-Out): Use when passage should start fading out before next passage begins crossfading in
  - **Lead-Out before Fade-Out** (Lead-Out ≤ Fade-Out): Use when next passage should start while current passage is still at full volume

## Fade Curves

### Per-Passage Curve Selection

Each passage can independently configure its fade-in and fade-out curves:

**Fade-In Curve Options:**
- **Exponential**: Volume increases exponentially (slow start, fast finish) - natural-sounding
- **Cosine (S-Curve)**: Smooth acceleration and deceleration - gentle, musical
- **Linear**: Constant rate of change - precise, predictable

**Fade-Out Curve Options:**
- **Logarithmic**: Volume decreases logarithmically (fast start, slow finish) - natural-sounding
- **Cosine (S-Curve)**: Smooth acceleration and deceleration - gentle, musical
- **Linear**: Constant rate of change - precise, predictable

**Independence:** Fade-in and fade-out curves are selected independently. A passage may use any combination (e.g., exponential fade-in with linear fade-out).

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
- Fade-In Point: NULL (uses global Crossfade Time)
- Lead-In Point: NULL (uses global Crossfade Time)
- Lead-Out Point: NULL (uses global Crossfade Time)
- Fade-Out Point: NULL (uses global Crossfade Time)
- End Time: End of file (or user-defined offset)
- Fade-In Curve: NULL (uses global Fade Curve selection's fade-in component)
- Fade-Out Curve: NULL (uses global Fade Curve selection's fade-out component)

### Global Crossfade Time
A single global value "Crossfade Time" is used whan Fade-In Point, Fade-Out Point, Lead-In Point, Lead-Out Point are undefined.
- When Fade-In Point is undefined, the effective Fade-In point is: Start Time + Crossfade Time
- When Lead-In Point is undefined, the effective Lead-In point is: Start Time + Crossfade Time
- When Fade-Out Point is undefined, the effective Fade-Out point is: End Time - Crossfade Time
- When Lead-Out Point is undefined, the effective Lead-Out point is: End Time - Crossfade Time

- Crossfade Time is user editable
  - Minimum Crossfade Time is: 0.0 seconds
  - Maximum Crossfade Time is: 30.0 seconds
  
- When one passage is playing and another is following it in the queue, if the global user selected Crossfade Time is > 50% of either passage duration, 
  effective Crossfade Time is 50% of the shorter passage duration.

- Default (before user editing) Crossfade Time is 2.0 seconds
- User edited Crossfade Time is persisted across power-off sessions

### Global Fade Curve Selection

The Global Fade Curve setting selects a **paired** fade-in/fade-out curve combination used as the default for passages where `fade_in_curve` and/or `fade_out_curve` are undefined (NULL).

**Global Fade Curve Options:**

1. **Exponential In / Logarithmic Out** (default)
   - Fade-In: Exponential
   - Fade-Out: Logarithmic

2. **Linear In / Linear Out**
   - Fade-In: Linear
   - Fade-Out: Linear

3. **Cosine In / Cosine Out (S-Curve)**
   - Fade-In: Cosine
   - Fade-Out: Cosine

**Application:**
- When passage `fade_in_curve` is NULL: Use fade-in component of global setting
- When passage `fade_out_curve` is NULL: Use fade-out component of global setting
- When both are NULL: Use complete paired curve from global setting
- When one is defined and one is NULL: Mix passage-specific curve with global default

**Example:** Global setting is "Exponential In / Logarithmic Out"
- Passage A (both NULL): Uses exponential fade-in, logarithmic fade-out
- Passage B (fade-in = linear, fade-out = NULL): Uses linear fade-in, logarithmic fade-out
- Passage C (fade-in = NULL, fade-out = cosine): Uses exponential fade-in, cosine fade-out
- Passage D (both defined): Ignores global setting entirely

**Persistence:** User-selected global Fade Curve setting is persisted across power-off sessions.

### Commentary

The Global Crossfade Time and Fade Curve Selection should be good for most passages, but in cases where it is not the ability to individually tailor Fade-In, Fade-Out, Lead-In, Lead-Out points and Fade-In, Fade-Out curves gives the users the ability to address specific cases that don't work well with a simpler global selection.

## Database Storage

Passage table stores:
- `start_time` (seconds, nullable)
- `fade_in_point` (seconds, nullable)
- `lead_in_point` (seconds, nullable)
- `lead_out_point` (seconds, nullable)
- `fade_out_point` (seconds, nullable)
- `end_time` (seconds, nullable)
- `fade_in_curve` (nullable enum: 'exponential', 'cosine', 'linear'; NULL = use global default)
- `fade_out_curve` (nullable enum: 'logarithmic', 'cosine', 'linear'; NULL = use global default)

**Note:** NULL values for timing points use global Crossfade Time as described in Default Configuration. NULL values for curves use global Fade Curve selection.

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
- Fade-in applies
- If Fade-In Duration > 0, passage starts at zero volume and fades in

### Last Passage in Queue
- No next passage to crossfade to
- Fade-out applies if defined
- Passage plays to End Time

### User Skip During Crossfade
- Both passages stop immediately
- Next passage from the queue, after the one that was starting during crossfade, begins according to its own timing rules

### Pause During Crossfade
- Both passages pause at current position
- Resume applies 0.5s ramp to both passages
- Crossfade relationship preserved

### Queue Modification During Crossfade
- If next passage removed: current passage plays to completion
- If current passage removed: next passage treated as "first passage in queue"

----
End of document - Crossfade Design
