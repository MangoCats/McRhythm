# McRhythm Crossfade System Diagrams

## Legend
```
Time axis:  ────────────────────────────────────────→
Volume:     ▁▂▃▄▅▆▇█  (0% to 100%)
Track audio: ████████████  (solid block)
```

---

## 1. Basic Track - No Fades or Leads (All Times = 0)

```
Track Timeline:
Time:      0s                                        30s
           ├──────────────────────────────────────────┤
           START                                     END
           FADE-IN                                FADE-OUT
           LEAD-IN                                LEAD-OUT

Volume:    ████████████████████████████████████████████  (100% throughout)

Audio:     ████████████████████████████████████████████
```

---

## 2. Track with Fade-In Only (Fade-in = 3s)

```
Track Timeline:
Time:      0s      3s                                30s
           ├───────┼─────────────────────────────────┤
           START   FADE-IN                          END
           LEAD-IN LEAD-IN                       LEAD-OUT
                                                   FADE-OUT

Volume:    ▁▃▅▇████████████████████████████████████████
           └─────┘ (Fade curve)
           0→100%

Audio:     ████████████████████████████████████████████
```

---

## 3. Track with Fade-Out Only (Fade-out = 4s)

```
Track Timeline:
Time:      0s                              26s     30s
           ├───────────────────────────────┼───────┤
           START                        FADE-OUT  END
           FADE-IN                      LEAD-OUT  LEAD-OUT
           LEAD-IN

Volume:    ████████████████████████████████▇▅▃▁
                                           └─────┘ (Fade curve)
                                           100→0%

Audio:     ████████████████████████████████████████████
```

---

## 4. Track with Both Fades (Fade-in = 2s, Fade-out = 3s)

```
Track Timeline:
Time:      0s   2s                          27s   30s
           ├────┼───────────────────────────┼─────┤
           START FADE-IN                 FADE-OUT END
           LEAD-IN                       LEAD-OUT LEAD-OUT

Volume:    ▁▅████████████████████████████████▇▃▁
           └──┘                              └───┘
           0→100%                           100→0%

Audio:     ████████████████████████████████████████████
```

---

## 5. Track with Leads and Fades (Fade-in=2s, Lead-in=5s, Lead-out=6s, Fade-out=3s)

```
Track Timeline:
Time:      0s   2s      5s              24s   27s   30s
           ├────┼───────┼────────────────┼─────┼─────┤
           START│    LEAD-IN         LEAD-OUT  │    END
           │    FADE-IN                      FADE-OUT│
           LEAD-IN                                LEAD-OUT

Volume:    ▁▅████████████████████████████████████▇▃▁
           └──┘                                  └───┘
              ↑                                  ↑
           Fade region                      Fade region

Lead-in:   ├──────────┤ (5s from start)
Lead-out:              ├──────────┤ (6s before end)
```

**Notes:**
- Lead-in point (5s) is where crossfade can begin from previous track
- Lead-out point (24s) is where next track can start its crossfade
- Fade-in must complete before or at lead-in point
- Fade-out must start at or after lead-out point

---

## 6. Crossfade Example A: Following Track has LONGER Lead-in

**Track A:** Lead-out = 5s, Fade-out = 2s (duration 30s)
**Track B:** Lead-in = 8s, Fade-in = 3s (duration 25s)

```
Track A Timeline:
Time:      0s                      23s  25s    28s 30s
           ├───────────────────────┼────┼───────┼───┤
           START                LEAD-OUT│    FADE-OUT END
                                      FADE-OUT     │
                                                LEAD-OUT

Volume A:  ████████████████████████████████████▇▅▃▁
                                                └──┘

Track B Timeline (starts when A reaches lead-out at 25s):
Time:      0s      3s         8s                    25s
           ├───────┼──────────┼──────────────────────┤
           START   │       LEAD-IN                  END
           │    FADE-IN
           LEAD-IN

Volume B:  ▁▃▆█████████████████████████████████████████
           └───┘

COMBINED TIMELINE (Track A time reference):
Time:      0s                      23s  25s    28s 30s
           │                        │   │       │   │
Track A:   ████████████████████████████████████▇▅▃▁
Track B:                                ▁▃▆████████████...
           ├─────────────────────────────┼─────────┤
                                         └─5s overlap─┘
                   Track A playing       │ Both playing
                                         └→ B starts at A's 25s mark
```

**Crossfade Rules Applied:**
- Track A has lead-out time = 5s (starts at 25s, ends at 30s)
- Track B has lead-in time = 8s (from 0s to 8s)
- B's lead-in (8s) > A's lead-out (5s) → LONGER lead-in case
- **Result:** Track B starts at its start time (0s) when Track A reaches lead-out point (25s)
- Overlap duration = A's lead-out time = 5s

---

## 7. Crossfade Example B: Following Track has SHORTER Lead-in

**Track A:** Lead-out = 8s, Fade-out = 3s (duration 30s)
**Track B:** Lead-in = 5s, Fade-in = 2s (duration 25s)

```
Track A Timeline:
Time:      0s                  19s 22s          30s
           ├───────────────────┼───┼────────────┤
           START            LEAD-OUT│          END
                                 FADE-OUT       │
                                             LEAD-OUT

Volume A:  ████████████████████████████████████▇▅▃▁
                                            └────┘

Track B Timeline:
Time:      0s   2s      5s                      25s
           ├────┼───────┼────────────────────────┤
           START│    LEAD-IN                    END
           │ FADE-IN
           LEAD-IN

Volume B:  ▁▅████████████████████████████████████████
           └──┘

COMBINED TIMELINE (Track A time reference):
Time:      0s                  19s 22s  25s    30s
           │                    │   │    │      │
Track A:   ████████████████████████████████████▇▅▃▁
Track B:                             ▁▅████████████...
           ├─────────────────────────┼───────┤
                                     └─5s overlap─┘
               Track A playing       │ Both playing
                                     └→ B starts at A's 25s mark
```

**Crossfade Rules Applied:**
- Track A has lead-out time = 8s (starts at 22s, ends at 30s)
- Track B has lead-in time = 5s (from 0s to 5s)
- B's lead-in (5s) < A's lead-out (8s) → SHORTER lead-in case
- **Result:** Track B starts when Track A has 5s remaining (at A's 25s mark)
- Track A time remaining = 30s - 25s = 5s = Track B's lead-in time ✓
- Overlap duration = B's lead-in time = 5s

---

## 8. Complex Example: Full Crossfade Chain

Three tracks demonstrating continuous crossfading:

```
Track 1: Lead-in=4s, Fade-in=1s, Lead-out=6s, Fade-out=2s, Duration=30s
Track 2: Lead-in=7s, Fade-in=2s, Lead-out=5s, Fade-out=1s, Duration=28s
Track 3: Lead-in=5s, Fade-in=1s, Duration=25s

Timeline:
Time (seconds):
0        10        20   24   30   35        50   51   56
├─────────┼─────────┼────┼────┼────┼─────────┼────┼────┤

Track 1:
████████████████████████████▇▃▁
└fade 1s─┘                 └─fade 2s─┘
    └──lead-in 4s─┘    └──lead-out 6s──┘
                              ↓ (at 24s)
Track 2:                      ▁▄██████████████████████▇▁
                              └fade 2s─┘              └fade 1s─┘
                                  └──lead-in 7s─┘ └─lead-out 5s─┘
                                                          ↓ (at 51s)
Track 3:                                                  ▁███████...
                                                          └fade 1s─┘
                                                            └─lead-in 5s─┘

Crossfade Analysis:
1→2: Track 1 lead-out (6s) < Track 2 lead-in (7s) → LONGER lead-in case
     Track 2 starts at its 0s when Track 1 reaches 24s (lead-out point)
     Overlap: 6s (from Track 1's 24s to 30s)

2→3: Track 2 lead-out (5s) = Track 3 lead-in (5s) → Equal (treated as shorter/equal)
     Track 3 starts when Track 2 has 5s remaining (at Track 2's 23s = global 47s+23s=70s - 19s=51s)
     Overlap: 5s (from Track 2's 51s to 56s)
```

---

## 9. Fade Curve Profiles

### Linear Fade
```
Volume
100% ███████████████████████
 75% ████████████████
 50% ██████████
 25% █████
  0%
     └──────────────────────→ Time
     0s                     3s
```

### Exponential Fade-In / Logarithmic Fade-Out
```
Fade-In (Exponential - slow start, fast finish):
Volume
100% ███████████████████████
 75%                ████████
 50%           █████
 25%      ████
  0% ███
     └──────────────────────→ Time

Fade-Out (Logarithmic - fast start, slow finish):
Volume
100% ███
 75% ████████
 50%           █████
 25%                ████████
  0%                        ███
     └──────────────────────→ Time
```

### Cosine/S-Curve Fade (smooth acceleration/deceleration)
```
Fade-In (S-curve):
Volume
100% ███████████████████████
 75%            ███████████
 50%       ██████
 25%   ████
  0% ██
     └──────────────────────→ Time

Fade-Out (S-curve - mirror of fade-in):
Volume
100% ██
 75%   ████
 50%       ██████
 25%            ███████████
  0%                        ███
     └──────────────────────→ Time
```

---

## 10. Edge Cases

### Case A: No Lead Times (Fade-in=2s, Fade-out=2s, Lead-in=0, Lead-out=0)
```
Cannot crossfade (no overlap window)
Track 1: ████████████████████████████▇▃▁
Track 2:                                ▁▃▇████████████...
         └──────────────────────────────┘
         Fade-out completes, then Track 2 fade-in begins (no overlap)
```

### Case B: Lead Without Fade (Lead-in=5s, Lead-out=5s, Fade-in=0, Fade-out=0)
```
Hard crossfade (both at full volume during overlap)
Track 1: ████████████████████████████
Track 2:                    █████████████████████...
         ├───────────────────┼────────┤
                             └─5s @ 200%─┘
         (May cause clipping if both tracks are loud)
```

### Case C: Pause Resume Fade (0.5s fade-in, independent of track fades)
```
Before Pause:
Track:   ████████████████▐ ← Pause at 16s
                         │
After Resume:            ↓
Track:                   ▁▃▇████████████████████
                         └─0.5s resume fade─┘
         (Resumes from 16s with 0.5s fade regardless of track's fade settings)
```

---

## Notes

1. **All fade times are configurable per track** with defaults of 0
2. **Fade profiles are global** (one setting applies to all tracks)
3. **Lead-in defines when a track CAN start** during previous track
4. **Lead-out defines when next track SHOULD start** during current track
5. **Fades are independent** - both tracks apply their own fades during overlap
6. **Resume from pause** uses a fixed 0.5s fade, independent of track fade settings
7. **Volume during overlap** is the sum of both tracks' faded volumes (may exceed 100% if not managed)
