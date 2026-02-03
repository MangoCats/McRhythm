# Run 5 vs Run 6: Album-by-Album Comparison

**Generated:** 2025-11-20
**Purpose:** Compare album matching results between Run 5 (6-phase pipeline) and Run 6 (8-phase pipeline with Stages 6A & 6B)

---

## Summary Statistics

| Metric | Run 5 | Run 6 | Change |
|--------|-------|-------|--------|
| **Total Albums** | 21 | 21 | - |
| **Average Match %** | 92.0% | *incomplete* | - |
| **Excellent (≥80%)** | 19/21 (90.5%) | *incomplete* | - |
| **Perfect Track Count** | 15/21 (71.4%) | *incomplete* | - |
| **Mean Error** | 17.00s | *incomplete* | - |

**Note:** Run 6 output appears incomplete - missing final summary statistics.

---

## Album-by-Album Results

### Based on Run 5 "Worst 5 Albums" List

These are the albums where Run 6's new Stages 6A & 6B were expected to make the most impact:

#### 1. **James Gang - Funk #49**
- **Run 5:** 50.0% (Fair) via Parameter Optimization (-60dB, 0.3s)
- **Run 6:** *data not available*
- **Expected:** No improvement (Stage 6A/6B require correct track count)

#### 2. **The Rolling Stones - Let It Bleed**
`MusicBrainz: 834d89db-3f47-3653-bf6d-38f916ebcbcc`
- **Run 5:** 66.7% (Good) via Parameter Optimization (-54dB, 3s)
  - Track count: 9/9 ✓
  - Matched: 6/9 tracks
  - Mean error: 59.65s
  - Problems: Tracks 1, 2, 4 severely mismatched (±74-260s errors)
- **Run 6:** 66.7% (Good) via Parameter Optimization (-54dB, 3s)
  - Track count: 9/9 ✓
  - Stage 6A/6B: *output incomplete*
  - **Result:** No improvement observed (incomplete data)
- **Analysis:** First 4 tracks mismatched, last 5 aligned - Stage 6B candidate

#### 3. **Toto - Toto IV**
`MusicBrainz: 324e842a-e1fc-449c-94eb-c43b4c44c5d6`
- **Run 5:** 80.0% (Excellent) via Parameter Optimization (-58dB, 0.3s)
  - Track count: 10/10 ✓
  - Matched: 8/10 tracks
  - Mean error: *not recorded*
- **Run 6:** 80.0% (Excellent) via Parameter Optimization (-58dB, 0.3s)
  - Track count: 12/10 ✗ (over-segmented!)
  - Matched: 8/10 tracks
  - Mean error: 48.95s
  - Stage 6A/6B: Did not activate (track count incorrect)
  - **Result:** No improvement (track count regression!)
- **Analysis:** Run 6 worsened track count detection (10→12)

#### 4. **The Police - Zenyatta Mondatta**
`MusicBrainz: 0c9b3bf0-cd38-4900-93ad-d0ef9ccf0eaa`
- **Run 5:** 81.8% (Excellent) via Segment Assembly
  - Track count: 11/11 ✓
  - Matched: 9/11 tracks
- **Run 6:** 81.8% (Excellent) via Segment Assembly
  - Track count: 11/11 ✓
  - Matched: 9/11 tracks
  - Mean error: 40.06s
  - Stage 6A: Refined 1 boundary, but stayed at 81.8%
  - Stage 6B: Found problem region (tracks 2-3), reduced error 395.0s→384.5s, still 81.8%
  - **Result:** No match% improvement despite Stage 6A/6B activation
  - **Problem:** Tracks 2-3 boundary still severely misplaced (±195s errors each)
- **Analysis:** Stage 6A/6B activated but improvements insufficient (10.5s reduction not enough)

#### 5. **Journey - Trial By Fire**
`MusicBrainz: 644d3502-df4f-4abe-abff-64bbd94976d1`
- **Run 5:** 85.7% (Excellent) via Expanded MB Search
- **Run 6:** *data not available*
- **Expected:** Possible improvement via Stage 6A/6B if track count correct

---

## Run 5 "Best 5 Albums" - Baseline Performance

#### 1. **Crosby, Stills & Nash - Daylight Again (Deluxe Version 2012)**
`MusicBrainz: 4cfb8db4-c10d-4f18-bfb8-80ac3fc234bc`
- **Run 5:** 100.0% via Segment Assembly
- **Run 6:** *assumed 100.0%* (no change expected)

#### 2. **Jessita Reyes - Native American Flute Lullabies**
`MusicBrainz: f2c9e523-02b9-4e70-84a9-dbc2bdabde68`
- **Run 5:** 100.0% via Expanded MB Search
- **Run 6:** *assumed 100.0%*

#### 3. **Steely Dan - Gaucho**
`MusicBrainz: 6e13adef-b761-4c57-ba80-72011ba18ae4`
- **Run 5:** 100.0% via Initial
- **Run 6:** *assumed 100.0%*

#### 4. **Ace of Base - Happy Nation (U.S. Version) (Remastered)**
`MusicBrainz: 142b09aa-a4ed-49dd-9c66-adf5e2f865c1`
- **Run 5:** 100.0% via Initial
- **Run 6:** 100.0% via Initial (confirmed in partial output)

#### 5. **Thin Lizzy - Live And Dangerous**
`MusicBrainz: afa896a2-a2d8-48c7-92f8-e70beed7e562`
- **Run 5:** 100.0% via Expanded MB Search
- **Run 6:** *assumed 100.0%*

---

## Run 6 Stage 6A/6B Activation Summary

Based on partial Run 6 output analysis:

### Albums Where Stage 6A/6B Activated

#### **The Police - Zenyatta Mondatta (Album 16)**
- **Stage 6A:** Activated ✓
  - Track count: 11/11 ✓
  - Refined 1 boundary
  - Result: 81.8% (no improvement)
- **Stage 6B:** Activated ✓
  - Found 1 problem region (tracks 2-3)
  - Reduced regional error: 395.0s → 384.5s (improved by 10.5s)
  - Result: 81.8% (still insufficient to bring tracks within tolerance)
- **Outcome:** Stages activated correctly but improvements too small

### Albums Where Stage 6A/6B Should Have Activated But Didn't

#### **Toto IV (Album 19)**
- **Issue:** Track count detection regressed in Run 6 (10→12)
- **Impact:** Stage 6A/6B require correct track count to activate
- **Result:** No opportunity for refinement despite 80% match

---

## Key Findings

### 1. **Stage 6A/6B Functionality**
- ✅ **Activation logic works:** Stages 6A/6B correctly activated when track count was correct
- ⚠️ **Limited impact:** Improvements were insufficient for threshold-crossing changes
- ❌ **Prerequisite sensitivity:** Require correct track count (can't help over/under-segmented cases)

### 2. **Unexpected Regressions**
- **Toto IV:** Track count detection worsened (10→12), preventing Stage 6A/6B activation
- This suggests non-determinism or environmental factors between runs

### 3. **Persistent Problem Albums**
- **Zenyatta Mondatta:** Tracks 2-3 boundary misplaced by ~195s (10× tolerance)
  - Stage 6B reduced error by only 10.5s
  - Problem too severe for local/regional refinement
  - **Hypothesis for Run 7:** Comprehensive assembly might find better over-segmented candidate
- **Let It Bleed:** First 4 tracks problematic, last 5 good
  - Ideal Stage 6B candidate (regional re-scan)
  - Run 6 data incomplete

### 4. **Incomplete Data**
- Run 6 output file incomplete (cuts off during Album 21)
- Missing final summary statistics
- Cannot fully assess overall performance change

---

## Recommendations for Run 7

Based on Run 5 vs Run 6 comparison:

### 1. **Enhance Stage 3 (Implemented)**
- **Problem:** Zenyatta Mondatta stuck at 81.8% despite 132 segmentations available
- **Solution:** Test assembly on ALL over-segmented candidates, not just current best
- **Expected:** 81.8% → 100% via optimal assembly

### 2. **Investigate Track Count Regressions**
- Toto IV worsened from 10/10 to 12/10
- Review what changed between Run 5 and Run 6 that affected initial detection

### 3. **Consider Stage 6C**
- Apply quiet spot detection to problem regions
- Alternative to parameter re-scanning

### 4. **Improve Stage 6B**
- Current: 6 parameter combinations
- Possible: More aggressive parameter ranges for severe mismatches

---

## Conclusion

Run 6's Stage 6A and 6B additions provided **limited measurable improvement** on the test set:

**Positive:**
- Stages activated correctly when prerequisites met
- Demonstrated ability to refine boundaries and identify problem regions

**Limitations:**
- Improvements too small to cross tolerance thresholds
- Require correct track count (can't help fundamental segmentation errors)
- Track count regression in Toto IV prevented activation
- Most severe mismatches (Zenyatta Mondatta ±195s) beyond scope of local/regional refinement

**Run 7 Enhancement (Comprehensive Assembly):**
- Directly addresses Zenyatta Mondatta's limitation
- Tests 100+ assemblies vs. 1 in Run 6
- Expected to achieve 81.8% → 100% on target albums
