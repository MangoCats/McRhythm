# Crossfade Design

**🎵 TIER 2 - DESIGN SPECIFICATION**

Defines crossfade timing and behavior. Derived from [requirements.md](requirements.md). See [Document Hierarchy](document_hierarchy.md).

> **Related Documentation:** [Architecture](architecture.md)

---

## Overview

**[XFD-OV-010]** McRhythm supports sophisticated crossfading between passages, allowing smooth transitions with configurable fade curves and overlap timing. Each passage has six timing points that control how it begins, ends, and overlaps with adjacent passages.

## Timing Points

**[XFD-TP-010]** Each passage has six timing points defined relative to the audio file:

```
Start    Fade-In   Lead-In            Lead-Out   Fade-Out    End
  |         |         |                    |         |         |
  |---------|---------|--------------------|---------|---------|
  |         |         |                    |         |         |
  0%        |         |                    |         |        100%
       Fade-In   Lead-In Point        Lead-Out  Fade-Out
        Point                           Point     Point
```
**Figure 1:** Fade inside Lead

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
**Figure 2:** Lead inside Fade

Fade-In / Fade-Out soften the passage start / end volume profiles, for example when taking a passage
from the middle of a continuous piece of music.

Lead-In / Lead-Out define portions of the passage where it is permissible / non-intrusive for a crossfade
operation to take place.

### Point Definitions

  1. **[XFD-PT-010] Start Time**: Beginning of passage audio
  2. **[XFD-PT-020] Fade-In Point**: When volume reaches 100%
  3. **[XFD-PT-030] Lead-In Point**: Latest time the previous passage may still be playing
  4. **[XFD-PT-040] Lead-Out Point**: Earliest time the next passage may start playing
  5. **[XFD-PT-050] Fade-Out Point**: When volume begins decreasing
  6. **[XFD-PT-060] End Time**: End of passage audio

  - **[XFD-PT-070] Lead-In/Lead-Out**: Define when this passage plays simultaneously with adjacent passages
  - **[XFD-PT-080] Fade-In/Fade-Out**: Define volume envelope (independent of simultaneous playback)

### Durations

- **[XFD-DUR-010] Fade-In Duration** = Fade-In Point - Start Time
- **[XFD-DUR-020] Lead-In Duration** = Lead-In Point - Start Time
- **[XFD-DUR-030] Lead-Out Duration** = End Time - Lead-Out Point
- **[XFD-DUR-040] Fade-Out Duration** = End Time - Fade-Out Point

### Constraints

**[XFD-CONS-010]** Timing points must satisfy two independent constraint chains:

  **[XFD-CONS-011] Fade Point Constraints:**
  - Start ≤ Fade-In ≤ Fade-Out ≤ End

  **[XFD-CONS-012] Lead Point Constraints:**
  - Start ≤ Lead-In ≤ Lead-Out ≤ End

  **[XFD-CONS-020] Cross-Chain Independence:**
  - Fade-In and Lead-In points are independent (either may come first, or may be equal)
  - Fade-Out and Lead-Out points are independent (either may come first, or may be equal)
  - All timing points may be equal to each other (resulting in 0-duration intervals)

  **Valid Examples:**
  1. Fade inside Lead, no simultaneous play with adjacent passages:
  Start(0s) = Lead-In(0s) < Fade-In(2s) < Fade-Out(58s) < Lead-Out(60s) = End(60s)
  2. Fade inside Lead, simultaneously play with adjacent passages possible:
  Start(0s) < Lead-In(5s) < Fade-In(7s) < Fade-Out(53s) < Lead-Out(55s) < End(60s)
  3. Lead inside Fade, no fade of this passage's level in or out:
  Start(0s) = Fade-In(0s) < Lead-In(2s) < Lead-Out(58s) < Fade-Out(60s) = End(60s)
  4. Interleaved ordering:
  Start(0s) < Fade-In(1s) < Lead-In(2s) <  Fade-Out(58s) < Lead-Out(59s) < End(60s)
  5. Zero-duration lead and fade windows, passage plays at constant volume, no simultaneous play with adjacent passages:
  Start(0s) = Fade-In(0s) = Lead-In(0s) = Lead-Out(60s) = Fade-Out(60s) = End(60s)
  6. Partial equality:
  Start(0s) = Fade-In(0s) = Lead-In(0s) < Lead-Out(58s) = Fade-Out(58s) < End(60s)

  **[XFD-CONS-030] Validation Rules:**
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

**[XFD-CURV-010]** Each passage can independently configure its fade-in and fade-out curves:

**[XFD-CURV-020] Fade-In Curve Options:**
- **[XFD-EXP-010] Exponential**: Volume increases exponentially (slow start, fast finish) - natural-sounding
- **[XFD-COS-010] Cosine (S-Curve)**: Smooth acceleration and deceleration - gentle, musical
- **[XFD-LIN-010] Linear**: Constant rate of change - precise, predictable

**[XFD-CURV-030] Fade-Out Curve Options:**
- **[XFD-EXP-020] Logarithmic**: Volume decreases logarithmically (fast start, slow finish) - natural-sounding
- **[XFD-COS-020] Cosine (S-Curve)**: Smooth acceleration and deceleration - gentle, musical
- **[XFD-LIN-020] Linear**: Constant rate of change - precise, predictable

**[XFD-CURV-040] Independence:** Fade-in and fade-out curves are selected independently. A passage may use any combination (e.g., exponential fade-in with linear fade-out).

## Crossfade Behavior

### Case 1: Following Passage Has Longer Lead-In Duration

**[XFD-BEH-C1-010]** When `Lead-Out Duration of Passage A ≤ Lead-In Duration of Passage B`:

```
Passage A: |---------------------------|******|
                         Lead-Out Point↑   End↑

Passage B:                             |*********|-------------------|
                                       ↑Start    ↑Lead-In Point

Timeline:  |---------------------------|------|------------------------|
           A playing alone             Both playing
                                              B playing alone
```

**[XFD-BEH-C1-020] Timing**: Passage B starts at its Start Time when Passage A reaches its Lead-Out Point.

**Example**:
- Passage A: Lead-Out Duration = 3 seconds
- Passage B: Lead-In Duration = 5 seconds
- Result: Passage B starts 3 seconds before Passage A ends (they overlap for 3 seconds)

### Case 2: Following Passage Has Shorter Lead-In Duration

**[XFD-BEH-C2-010]** When `Lead-Out Duration of Passage A > Lead-In Duration of Passage B`:

```
Passage A: |---------------------------|*************|
                         Lead-Out Point↑          End↑

Passage B:                                   |*******|-----------------|
                                             ↑Start  ↑Lead-In Point

Timeline:  |---------------------------------|-------|-----------------|
           A playing alone                   Both playing
                                                     B playing alone
```

**[XFD-BEH-C2-020] Timing**: Passage B starts at its Start Time when Passage A has `Lead-In Duration of B` remaining before its End Time.

**Example**:
- Passage A: Lead-Out Duration = 5 seconds, currently at time point where 2 seconds remain
- Passage B: Lead-In Duration = 2 seconds
- Result: Passage B starts now (they overlap for 2 seconds)

### Case 3: No Overlap (Zero Lead Durations)

**[XFD-BEH-C3-010]** When both Lead-In Duration and Lead-Out Duration are 0:

```
Passage A: |---------------------------|
                                       ↑End

Passage B:                             |-----------------|
                                       ↑Start

Timeline:  |---------------------------|------------------|
           A playing                   B playing
```

**[XFD-BEH-C3-020] Timing**: Passage B starts immediately when Passage A ends (no gap, no overlap).

## Fade Behavior During Crossfade

**[XFD-FADE-010]** Fades operate independently of crossfade overlap

## Volume Calculations

**[XFD-VOL-010]** During crossfade overlap, audio streams are mixed:

```
Final Output = (Passage A Audio × A Volume) + (Passage B Audio × B Volume)
```

**[XFD-VOL-020]** Where volume at any time `t` is determined by:
- Current passage fade state (fade-in/fade-out curve)
- Resume-from-pause fade (0.5s exponential ramp, if applicable)
- User-controlled master volume

### Clipping Prevention
**[XFD-VOL-030]** There is no clipping prevention at runtime. If the crossfade configuration results in net volume > 100% clipping will occur.

**[XFD-VOL-040]** During user editing of fade-in, fade-out, lead-in, lead-out points, the user interface displays the passage audio amplitude over time graphically (see [Visual Editor](#visual-editor)
section), and warns the user if the peak audio amplitude exceeds 50% during any portion of the lead-in or lead-out time windows that is not also covered by
corresponding fade-in or fade-out time windows. This warning indicates potential for clipping if this passage overlaps with another passage also at high volume.

**[XFD-VOL-050] Amplitude Measurement:**
  - Peak amplitude is measured from the audio file's waveform data
  - Measured as the maximum absolute sample value in the relevant time window
  - Does not include fade curve or master volume adjustments
  - Checks the raw audio amplitude to detect potential clipping before fades are applied
    - Warning is shown if > 50% amplitude is detected in relevant time window and no fade-in/fade-out curve is applied at that point in the passage playback 

### Resume from Pause

**[XFD-PAUS-010]** When resuming from Pause:
- **[XFD-PAUS-011]** 0.5 second exponential volume ramp from 0% to 100%
- **[XFD-PAUS-012]** Applied AFTER any crossfade calculations
  - all crossfade calculations complete, then multiply the result by the resume from pause ramp value
  - ((A_audio × A_fade) + (B_audio × B_fade)) × resume_ramp × master
- **[XFD-PAUS-013]** Affects all playing passages simultaneously
- **[XFD-PAUS-020]** Rationale:
  - 0.5 second duration prevents audible "pop" when resuming
  - Exponential curve provides natural-sounding volume restoration
  - Applied to all playing passages ensures synchronized behavior during overlap
  
## Default Configuration

**[XFD-DEF-010]** New passages are created with:
- Start Time: NULL (uses start of file)
- Fade-In Point: NULL (uses global Crossfade Time)
- Lead-In Point: NULL (uses global Crossfade Time)
- Lead-Out Point: NULL (uses global Crossfade Time)
- Fade-Out Point: NULL (uses global Crossfade Time)
- End Time: NULL (uses end of file)
- Fade-In Curve: NULL (uses global Fade Curve selection's fade-in component)
- Fade-Out Curve: NULL (uses global Fade Curve selection's fade-out component)

### Global Crossfade Time
**[XFD-DEF-020]** A single global value "Crossfade Time" is used when Fade-In Point, Fade-Out Point, Lead-In Point, Lead-Out Point are undefined.
- When Fade-In Point is undefined, the effective Fade-In point is: Start Time + Crossfade Time (Clamped when needed)
- When Lead-In Point is undefined, the effective Lead-In point is: Start Time + Crossfade Time (Clamped when needed)
- When Fade-Out Point is undefined, the effective Fade-Out point is: End Time - Crossfade Time (Clamped when needed)
- When Lead-Out Point is undefined, the effective Lead-Out point is: End Time - Crossfade Time (Clamped when needed)

**[XFD-DEF-030] NULL Point Computation Timing:**
  - NULL timing points are NOT pre-computed or stored as effective values in the database
  - Instead, NULL values remain NULL in storage and are resolved dynamically when a passage is currently playing and a following passage is waiting in the queue to play next
  - When both passage identities are first known, the system:
    1. Determines the effective Crossfade Time (applying 50% clamping if needed per Crossfade Time Clamping below)
    2. Computes effective timing points for any NULL values using the clamped Crossfade Time
    3. Applies Crossfade Behavior Cases 1/2/3 using the computed effective durations
  - User-defined (non-NULL) timing points are never modified by clamping
  - This ensures the 50% constraint is satisfied for all applications of undefined lead-in and lead-out times.
  - Fade-In and Fade-Out NULL points are also computed using the clamped Crossfade Time, ensuring consistent behavior across all timing points.

**[XFD-DEF-040]** Crossfade Time is user editable:
  - Minimum Crossfade Time is: 0.0 seconds
  - Maximum Crossfade Time is: 30.0 seconds
- Default (before user editing) Crossfade Time is 2.0 seconds
- User edited Crossfade Time is persisted across power-off sessions

### Crossfade Time Clamping
**[XFD-DEF-050]** User defined fade-in, fade-out, lead-in, lead-out durations are constrained during editing and setting as described above.

**[XFD-DEF-060]** The global Crossfade Time is clamped during playback as follows:
  - When one passage is playing and the queue contains another passage to play next, the global user selected Crossfade Time is compared with
    the End Time - Start Time durations of either or both passages.  If the global user selected Crossfade Time is > 50% of either passage duration,
    effective Crossfade Time is 50% of the shorter passage duration.
  - If a playing passage has a defined (non NULL) lead-out duration, that is unaffected by Crossfade clamping
  - If a passage next for play in the queue has a defined (non NULL) lead-in duration, that is unaffected by Crossfade clamping

Example:
- Typical scenario, lead-out of passage A and lead-in of passage B are both NULL:
  - Crossfade Time set to 5 seconds
  - Passage A is 180 seconds duration, lead-out duration undefined
  - Passage B is 240 seconds duration, lead-in duration undefined
  - When Passage A is playing and Passage B is in the queue to play next, Crossfade Time is compared to the passage times and found to be < 50% of both, so it is used as is.
  
- Scenario A where Crossfade Time Clamping comes into effect, lead-out of passage A and lead-in of passage B are both NULL:
  - Crossfade Time set to 30 seconds
  - Passage A is 40 seconds duration, lead-out duration undefined
  - Passage B is 240 seconds duration, lead-in duration undefined
  - When Passage A is playing and Passage B is in the queue to play next, Crossfade Time is compared to the passage times and found to be > 50% of passage A's duration, so the 
    effective Crossfade Time used to play passage A and B overlapping is 20 seconds, 50% of passage A's duration.  When passage A reaches 20 seconds from its end, simultaneous
    play of passage B begins.
  
- Scenario B where Crossfade Time Clamping comes into effect, lead-out of passage A and lead-in of passage B are both NULL:
  - Crossfade Time set to 30 seconds
  - Passage A is 180 seconds duration, lead-out duration undefined
  - Passage B is 30 seconds duration, lead-in duration undefined
  - When Passage A is playing and Passage B is in the queue to play next, Crossfade Time is compared to the passage times and found to be > 50% of passage B's duration, so the
    effective Crossfade Time used to play passage A and B overlapping is 15 seconds, 50% of passage B's duration.  When passage A reaches 15 seconds from its end, simultaneous
    play of passage B begins.
  
- Scenario C where Crossfade Time Clamping comes into effect, lead-out of passage A and lead-in of passage B are both NULL:
  - Crossfade Time set to 25 seconds
  - Passage A is 30 seconds duration, lead-out duration undefined
  - Passage B is 40 seconds duration, lead-in duration undefined
  - When Passage A is playing and Passage B is in the queue to play next, Crossfade Time is compared to the passage times and found to be > 50% of both passage A and B's duration,
    so the effective Crossfade Time used to play passage A and B overlapping is 15 seconds, 50% of the shorter passage A's duration.  When passage A reaches 15 seconds from its end,
    simultaneous play of passage B begins.

- Scenario D where Crossfade Time Clamping comes into effect, lead-out of passage A is set to 20 seconds lead-in of passage B is NULL:
  - Crossfade Time set to 30 seconds
  - Passage A is 30 seconds duration, lead-out duration set to 20 seconds
  - Passage B is 50 seconds, lead-in duration undefined
  - When Passage A is playing and Passage B is in the queue to play next, Crossfade Time is compared to the passage B's duration and found to be > 50%, so the effective Crossfade Time
    used for passage B's lead-in duration is 15 seconds, 50% of the shorter passage A's duration.  When passage A reaches 15 seconds from its end, simultaneous play of passage B begins.

- Scenario E where Crossfade Time Clamping comes into effect, lead-out of passage A is NULL, lead-in of passage B is set to 20 seconds:
  - Crossfade Time set to 25 seconds
  - Passage A is 32 seconds duration, lead-out duration undefined
  - Passage B is 50 seconds, lead-in duration is set to 20 seconds
  - When Passage A is playing and Passage B is in the queue to play next, Crossfade Time is compared to the passage A's duration and found to be > 50%, so the effective Crossfade Time
    used for passage A's lead-out duration is 16 seconds, 50% of the shorter passage A's duration.  When passage A reaches 16 seconds from its end, simultaneous play of passage B begins.

### Global Fade Curve Selection

**[XFD-DEF-070]** The Global Fade Curve setting selects a **paired** fade-in/fade-out curve combination used as the default for passages where `fade_in_curve` and/or `fade_out_curve` are undefined (NULL).

**[XFD-DEF-071] Global Fade Curve Options:**

1. **Exponential In / Logarithmic Out** (default)
   - Fade-In: Exponential
   - Fade-Out: Logarithmic

2. **Linear In / Linear Out**
   - Fade-In: Linear
   - Fade-Out: Linear

3. **Cosine In / Cosine Out (S-Curve)**
   - Fade-In: Cosine
   - Fade-Out: Cosine

**[XFD-DEF-072] Application:**
- When passage `fade_in_curve` is NULL: Use fade-in component of global setting
- When passage `fade_out_curve` is NULL: Use fade-out component of global setting
- When both are NULL: Use complete paired curve from global setting
- When one is defined and one is NULL: Mix passage-specific curve with global default

**Example:** Global setting is "Exponential In / Logarithmic Out"
- Passage A (both NULL): Uses exponential fade-in, logarithmic fade-out
- Passage B (fade-in = linear, fade-out = NULL): Uses linear fade-in, logarithmic fade-out
- Passage C (fade-in = NULL, fade-out = cosine): Uses exponential fade-in, cosine fade-out
- Passage D (both defined): Ignores global setting entirely

**[XFD-DEF-080] Persistence:** User-selected global Fade Curve setting is persisted across power-off sessions.

### Commentary

The Global Crossfade Time and Fade Curve Selection should be good for most passages, but in cases where it is not the ability to individually tailor Fade-In, Fade-Out, Lead-In, Lead-Out points and Fade-In, Fade-Out curves gives the users the ability to address specific cases that don't work well with a simpler global selection.

## Database Storage

**[XFD-DB-010]** Passage table stores:
- `start_time` (seconds, nullable)
- `fade_in_point` (seconds, nullable)
- `lead_in_point` (seconds, nullable)
- `lead_out_point` (seconds, nullable)
- `fade_out_point` (seconds, nullable)
- `end_time` (seconds, nullable)
- `fade_in_curve` (nullable enum: 'exponential', 'cosine', 'linear'; NULL = use global default)
- `fade_out_curve` (nullable enum: 'logarithmic', 'cosine', 'linear'; NULL = use global default)

**[XFD-DB-020]** NULL values for timing points use global Crossfade Time as described in Default Configuration. NULL values for curves use global Fade Curve selection.

### Settings Table

**[XFD-DB-030]** Global settings are persisted in a key-value settings table.

TODO: add reference to database_schema.md when the settings table is defined in there.

## User Interface Considerations

### Visual Editor
**[XFD-UIX-010]** Should display:
- **[XFD-VIS-010]** Waveform with six draggable markers
- **[XFD-VIS-020]** Preview of crossfade overlap with adjacent passages
- **[XFD-VIS-030]** Real-time audio preview of crossfade region

### Validation
**[XFD-UIX-020]** Validation requirements:
- **[XFD-VAL-010]** Enforce temporal constraints when user moves markers
- **[XFD-VAL-020]** Warn if any duration > 50% of passage duration
- **[XFD-VAL-030]** Suggest sensible defaults based on passage characteristics

## Edge Cases

### First Passage in Queue
**[XFD-EDGE-010]** First passage behavior:
- **[XFD-FIRST-010]** No previous passage to crossfade from
- **[XFD-FIRST-020]** Fade-in applies
- **[XFD-FIRST-030]** If Fade-In Duration > 0, passage starts at zero volume and fades in

### Last Passage in Queue
**[XFD-EDGE-020]** Last passage behavior:
- **[XFD-LAST-010]** No next passage to crossfade to
- **[XFD-LAST-020]** Fade-out applies if defined
- **[XFD-LAST-030]** Passage plays to End Time

### User Skip During Crossfade
**[XFD-EDGE-030]** Skip during crossfade:
- **[XFD-SKIP-010]** Both passages stop immediately
- **[XFD-SKIP-020]** Next passage from the queue, after the one that was starting during crossfade, begins according to its own timing rules

### Pause During Crossfade
**[XFD-EDGE-040]** Pause during crossfade:
- **[XFD-PAUS-030]** Both passages pause at current position
- **[XFD-PAUS-040]** Resume applies 0.5s ramp to both passages
- **[XFD-PAUS-050]** Crossfade relationship preserved

### Queue Modification During Crossfade
**[XFD-EDGE-050]** Queue modification during crossfade:
- **[XFD-QMOD-010]** If next passage removed: current passage plays to completion
- **[XFD-QMOD-020]** If current passage removed: next passage treated as "first passage in queue"

----
End of document - Crossfade Design
