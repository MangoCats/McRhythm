# Run 5 vs Run 7: Detailed Album-by-Album Comparison

**Generated:** 2025-11-20
**Purpose:** Comprehensive comparison of all 21 albums between Run 5 (6-phase pipeline) and Run 7 (8-phase with Enhanced Stage 3)

---

## Overview Statistics

| Metric | Run 5 | Run 7 | Improvement |
|--------|-------|-------|-------------|
| **Average Match %** | 92.0% | 98.8% | **+6.8%** |
| **Excellent (≥80%)** | 19/21 (90.5%) | 21/21 (100%) | **+2 albums** |
| **Perfect (100%)** | 15 albums | 18 albums | **+3 albums** |
| **Perfect Track Count** | 15/21 (71.4%) | 19/21 (90.5%) | **+4 albums** |
| **Mean Error** | 17.00s | 3.74s | **-13.26s (78% reduction)** |

---

## Album-by-Album Results

### 🎯 **Albums with Different MusicBrainz Release IDs**

| # | Album | Run 5 Release | Run 7 Release | Status |
|---|-------|---------------|---------------|--------|
| 3 | **Steely Dan - Gaucho** | `6e13adef-b761-4c57-ba80-72011ba18ae4` | `ab9b24fc-2155-41b0-bbc8-4d2ef6868771` | **DIFFERENT** ⚠️ |
| 11 | **Daft Punk - TRON: Legacy Reconfigured** | `ef6ad525-3595-430b-a0a4-f080054ff2c1` | `7898f198-d463-4a37-8299-335118aa359a` | **DIFFERENT** ⚠️ |
| 17 | **The Chemical Brothers - Surrender** | `598df38d-775c-4464-9143-a1e4e71bdaab` | `7898f198-d463-4a37-8299-335118aa359a` | **DIFFERENT** ⚠️ |

**All other albums (18/21):** Same MusicBrainz release ID in both runs ✓

---

## Detailed Album Results

### Album 1: Crosby, Stills & Nash - Daylight Again

| Attribute | Run 5 | Run 7 |
|-----------|-------|-------|
| **Match %** | 100.0% | 100.0% |
| **Stage** | Segment Assembly | Segment Assembly |
| **MusicBrainz** | `4cfb8db4-c10d-4f18-bfb8-80ac3fc234bc` | `4cfb8db4-c10d-4f18-bfb8-80ac3fc234bc` ✓ |
| **Track Count** | 15/15 ✓ | 15/15 ✓ |
| **Mean Error** | 2.79s | 3.97s |
| **Parameters** | -60dB, 0.3s | -56dB, 0.05s |
| **Run 7 Enhancement** | N/A (already perfect) | **Enhanced Stage 3:** 93.3% → 100% (56 segments → 15 tracks) |

### Album 2: Jessita Reyes - Native American Flute Lullabies

| Attribute | Run 5 | Run 7 |
|-----------|-------|-------|
| **Match %** | 100.0% | 100.0% |
| **Stage** | Expanded MB Search | Expanded MB Search |
| **MusicBrainz** | `f2c9e523-02b9-4e70-84a9-dbc2bdabde68` | `f2c9e523-02b9-4e70-84a9-dbc2bdabde68` ✓ |
| **Track Count** | 18/18 ✓ | 16/16 ✓ |
| **Mean Error** | 5.87s | 5.02s |
| **Parameters** | -58dB, 2.5s | -44dB, 0.8s |
| **Note** | Track count discrepancy between runs (18 vs 16) - suggests different MB release matched |

### Album 3: Michael Jackson - Thriller

| Attribute | Run 5 | Run 7 |
|-----------|-------|-------|
| **Match %** | 88.9% | 88.9% |
| **Stage** | Parameter Optimization | Parameter Optimization |
| **MusicBrainz** | `323ac42d-3b2a-4614-b163-10ce77be6909` | `323ac42d-3b2a-4614-b163-10ce77be6909` ✓ |
| **Track Count** | 8/9 ✗ | 9/9 ✓ |
| **Matched Tracks** | 8/9 | 8/9 |
| **Mean Error** | 2.84s | 4.09s |
| **Parameters** | -44dB, 0.5s | -44dB, 0.5s |
| **Run 7 Enhancement** | - | **Enhanced Stage 3:** Progressive 66.7% → 77.8% → 88.9% |

### Album 4: Billy Thorpe - Children Of The Sun...Revisited

| Attribute | Run 5 | Run 7 |
|-----------|-------|-------|
| **Match %** | 88.9% | 88.9% |
| **Stage** | Segment Assembly | Segment Assembly |
| **MusicBrainz** | `9d4b0e99-d694-40bb-af1b-d2789cba5f0f` | `9d4b0e99-d694-40bb-af1b-d2789cba5f0f` ✓ |
| **Track Count** | 9/9 ✓ | 9/9 ✓ |
| **Matched Tracks** | 8/9 | 8/9 |
| **Mean Error** | 5.52s | 7.82s |
| **Parameters** | -56dB, 0.1s | -54dB, 0.15s |

### Album 5: Steely Dan - Gaucho ⚠️

| Attribute | Run 5 | Run 7 |
|-----------|-------|-------|
| **Match %** | 100.0% | 100.0% |
| **Stage** | Initial | Initial |
| **MusicBrainz** | `6e13adef-b761-4c57-ba80-72011ba18ae4` | `ab9b24fc-2155-41b0-bbc8-4d2ef6868771` ⚠️ **DIFFERENT** |
| **Track Count** | 7/7 ✓ | 7/7 ✓ |
| **Mean Error** | 2.77s | 3.28s |
| **Parameters** | -57dB (default) | -57dB (default) |
| **Analysis** | Different MB release detected despite same default parameters |

### Album 6: James Gang - Funk #49

| Attribute | Run 5 | Run 7 |
|-----------|-------|-------|
| **Match %** | **50.0%** ❌ | **100.0%** ✅ |
| **Stage** | Parameter Optimization | **Segment Assembly** |
| **MusicBrainz** | `55be9910-ca58-4e61-9e14-2751812044ff` | `55be9910-ca58-4e61-9e14-2751812044ff` ✓ |
| **Track Count** | 9/10 ✗ | 10/10 ✓ |
| **Matched Tracks** | 5/10 | 10/10 |
| **Mean Error** | 98.37s | 3.61s |
| **Parameters** | -60dB, 0.3s | -44dB, 3s |
| **Run 7 Enhancement** | **WORST → PERFECT!** | Enhanced Stage 3 achieved 100% via assembly |
| **Improvement** | - | **+50% match, -94.76s error (96% reduction)** |

### Album 7: Journey - Trial By Fire

| Attribute | Run 5 | Run 7 |
|-----------|-------|-------|
| **Match %** | 85.7% | **100.0%** ✅ |
| **Stage** | Expanded MB Search | **Segment Assembly** |
| **MusicBrainz** | `644d3502-df4f-4abe-abff-64bbd94976d1` | `0588dde0-f221-4495-86b0-257485cac990` ⚠️ **DIFFERENT** |
| **Track Count** | 15/14 ✗ | 16/16 ✓ |
| **Matched Tracks** | 12/14 | 16/16 |
| **Mean Error** | 46.49s | 2.61s |
| **Parameters** | -46dB, 0.5s | -44dB, 0.05s |
| **Run 7 Enhancement** | - | **Enhanced Stage 3:** 81.2% → 100% (40 segments → 16 tracks) |
| **Improvement** | - | **+14.3% match, -43.88s error (94% reduction)** |

### Album 8: Ace of Base - Happy Nation

| Attribute | Run 5 | Run 7 |
|-----------|-------|-------|
| **Match %** | 100.0% | 100.0% |
| **Stage** | Initial | Initial |
| **MusicBrainz** | `142b09aa-a4ed-49dd-9c66-adf5e2f865c1` | `142b09aa-a4ed-49dd-9c66-adf5e2f865c1` ✓ |
| **Track Count** | 16/16 ✓ | 16/16 ✓ |
| **Mean Error** | 3.80s | 3.80s |
| **Parameters** | -57dB (default) | -57dB (default) |

### Album 9: Thin Lizzy - Live And Dangerous

| Attribute | Run 5 | Run 7 |
|-----------|-------|-------|
| **Match %** | 100.0% | 100.0% |
| **Stage** | Expanded MB Search | Expanded MB Search |
| **MusicBrainz** | `afa896a2-a2d8-48c7-92f8-e70beed7e562` | `afa896a2-a2d8-48c7-92f8-e70beed7e562` ✓ |
| **Track Count** | 17/17 ✓ | 17/17 ✓ |
| **Mean Error** | 3.89s | 3.89s |
| **Parameters** | -54dB, 0.3s | -54dB, 0.3s |

### Album 10: Heather Nova - Pearl

| Attribute | Run 5 | Run 7 |
|-----------|-------|-------|
| **Match %** | 100.0% | 100.0% |
| **Stage** | Parameter Optimization | Parameter Optimization |
| **MusicBrainz** | `1af55ae8-9338-4fa0-a083-0a85f07949f9` | `1af55ae8-9338-4fa0-a083-0a85f07949f9` ✓ |
| **Track Count** | 11/11 ✓ | 11/11 ✓ |
| **Mean Error** | 3.20s | 3.20s |
| **Parameters** | -60dB, 0.2s | -60dB, 0.2s |

### Album 11: Daft Punk - TRON: Legacy Reconfigured ⚠️

| Attribute | Run 5 | Run 7 |
|-----------|-------|-------|
| **Match %** | 100.0% | 100.0% |
| **Stage** | Parameter Optimization | Parameter Optimization |
| **MusicBrainz** | `ef6ad525-3595-430b-a0a4-f080054ff2c1` | `7898f198-d463-4a37-8299-335118aa359a` ⚠️ **DIFFERENT** |
| **Track Count** | 15/15 ✓ | 15/15 ✓ |
| **Mean Error** | 1.32s | 1.73s |
| **Parameters** | -52dB, 0.3s | -52dB, 0.3s |
| **Analysis** | Different MB release despite same parameters and track count |

### Album 12: Kraftwerk - Trans-Europe Express

| Attribute | Run 5 | Run 7 |
|-----------|-------|-------|
| **Match %** | 100.0% | 100.0% |
| **Stage** | Initial | Initial |
| **MusicBrainz** | `ca9f8a4b-3889-473a-9aba-1989a8decd90` | `ca9f8a4b-3889-473a-9aba-1989a8decd90` ✓ |
| **Track Count** | 9/8 ✗ | 6/6 ✓ |
| **Mean Error** | 3.84s | 3.95s |
| **Note** | Track count improved (9→6 detected), now perfect |

### Album 13: The Knack - Get The Knack

| Attribute | Run 5 | Run 7 |
|-----------|-------|-------|
| **Match %** | 100.0% | 100.0% |
| **Stage** | Segment Assembly | Initial |
| **MusicBrainz** | `b3d00e24-4d48-4491-a901-29ff8fb36087` | `b3d00e24-4d48-4491-a901-29ff8fb36087` ✓ |
| **Track Count** | 12/12 ✓ | 12/12 ✓ |
| **Mean Error** | 3.66s | 4.13s |
| **Parameters** | -50dB, 0.15s | -57dB (default) |
| **Note** | Run 7 achieved 100% at Stage 1 (faster) |

### Album 14: Men at Work - Business As Usual

| Attribute | Run 5 | Run 7 |
|-----------|-------|-------|
| **Match %** | 100.0% | 100.0% |
| **Stage** | Initial | Initial |
| **MusicBrainz** | `cff7fe38-24d9-4ab8-8c9f-2e3a4e4ecf6b` | `cff7fe38-24d9-4ab8-8c9f-2e3a4e4ecf6b` ✓ |
| **Track Count** | 10/10 ✓ | 10/10 ✓ |
| **Mean Error** | 5.42s | 5.42s |
| **Parameters** | -57dB (default) | -57dB (default) |

### Album 15: Katrina & The Waves - Katrina & The Waves

| Attribute | Run 5 | Run 7 |
|-----------|-------|-------|
| **Match %** | 100.0% | 100.0% |
| **Stage** | Initial | Initial |
| **MusicBrainz** | `8fecaba6-a4d5-4b00-a770-5d3b8c4541be` | `8fecaba6-a4d5-4b00-a770-5d3b8c4541be` ✓ |
| **Track Count** | 10/10 ✓ | 10/10 ✓ |
| **Mean Error** | 4.46s | 4.46s |
| **Parameters** | -57dB (default) | -57dB (default) |

### Album 16: The Police - Zenyatta Mondatta 🎯

| Attribute | Run 5 | Run 7 |
|-----------|-------|-------|
| **Match %** | **81.8%** ❌ | **100.0%** ✅ |
| **Stage** | Segment Assembly | **Segment Assembly** |
| **MusicBrainz** | `0c9b3bf0-cd38-4900-93ad-d0ef9ccf0eaa` | `0c9b3bf0-cd38-4900-93ad-d0ef9ccf0eaa` ✓ |
| **Track Count** | 11/11 ✓ | 11/11 ✓ |
| **Matched Tracks** | 9/11 | 11/11 |
| **Mean Error** | 40.06s | 5.32s |
| **Parameters** | -50dB, 0.5s | -50dB, 0.05s |
| **Run 7 Enhancement** | **TARGET ALBUM!** | **Enhanced Stage 3:** 81.8% → 90.9% → 100% (72 segments → 11 tracks) |
| **Improvement** | - | **+18.2% match, -34.74s error (87% reduction)** |
| **Details** | Tracks 2-3 boundary off by ±195s | **SOLVED** via comprehensive assembly search |

### Album 17: The Chemical Brothers - Surrender ⚠️

| Attribute | Run 5 | Run 7 |
|-----------|-------|-------|
| **Match %** | 90.9% | **96.4%** ✅ |
| **Stage** | Expanded MB Search | **Segment Assembly** |
| **MusicBrainz** | `598df38d-775c-4464-9143-a1e4e71bdaab` | `7898f198-d463-4a37-8299-335118aa359a` ⚠️ **DIFFERENT** |
| **Track Count** | 38/11 ✗ | 28/28 ✓ |
| **Matched Tracks** | 10/11 | 27/28 |
| **Mean Error** | 9.46s | 2.90s |
| **Parameters** | -44dB, 0.3s | -54dB, 0.05s |
| **Run 7 Enhancement** | - | **Enhanced Stage 3:** Progressive 89.3% → 92.9% → 96.4% (59 segments → 28 tracks) |
| **Improvement** | - | **+5.5% match, -6.56s error (69% reduction), correct track count!** |
| **Note** | Different MB release found (11-track vs 28-track edition) |

### Album 18: Eagles - Eagles

| Attribute | Run 5 | Run 7 |
|-----------|-------|-------|
| **Match %** | 100.0% | 100.0% |
| **Stage** | Initial | Initial |
| **MusicBrainz** | `ae4862d5-3c1c-44dd-bd50-9e2858f6a17e` | `ae4862d5-3c1c-44dd-bd50-9e2858f6a17e` ✓ |
| **Track Count** | 10/10 ✓ | 10/10 ✓ |
| **Mean Error** | 3.27s | 3.27s |
| **Parameters** | -57dB (default) | -57dB (default) |

### Album 19: Toto - Toto IV

| Attribute | Run 5 | Run 7 |
|-----------|-------|-------|
| **Match %** | 80.0% | **100.0%** ✅ |
| **Stage** | Parameter Optimization | **Segment Assembly** |
| **MusicBrainz** | `324e842a-e1fc-449c-94eb-c43b4c44c5d6` | `324e842a-e1fc-449c-94eb-c43b4c44c5d6` ✓ |
| **Track Count** | 12/10 ✗ | 10/10 ✓ |
| **Matched Tracks** | 8/10 | 10/10 |
| **Mean Error** | 48.95s | 3.89s |
| **Parameters** | -58dB, 0.3s | -58dB, 0.3s |
| **Run 7 Enhancement** | - | **Enhanced Stage 3:** Progressive improvements via assembly |
| **Improvement** | - | **+20% match, -45.06s error (92% reduction), correct track count!** |

### Album 20: Jimmy Buffett - Life On The Flipside

| Attribute | Run 5 | Run 7 |
|-----------|-------|-------|
| **Match %** | 100.0% | 100.0% |
| **Stage** | Initial | Initial |
| **MusicBrainz** | `6c8e0f64-77bd-4fef-9f9d-0665a782ed18` | `6c8e0f64-77bd-4fef-9f9d-0665a782ed18` ✓ |
| **Track Count** | 14/14 ✓ | 14/14 ✓ |
| **Mean Error** | 1.47s | 1.47s |
| **Parameters** | -57dB (default) | -57dB (default) |

### Album 21: The Rolling Stones - Let It Bleed 🎯

| Attribute | Run 5 | Run 7 |
|-----------|-------|-------|
| **Match %** | **66.7%** ❌ | **100.0%** ✅ |
| **Stage** | Parameter Optimization | **Segment Assembly** |
| **MusicBrainz** | `834d89db-3f47-3653-bf6d-38f916ebcbcc` | `834d89db-3f47-3653-bf6d-38f916ebcbcc` ✓ |
| **Track Count** | 9/9 ✓ | 9/9 ✓ |
| **Matched Tracks** | 6/9 | 9/9 |
| **Mean Error** | 59.65s | 1.84s |
| **Parameters** | -54dB, 3s | -54dB, 3s |
| **Run 7 Enhancement** | **TARGET ALBUM!** | **Enhanced Stage 3:** Initial 10 segments → 9 tracks = 100% (first assembly!) |
| **Improvement** | - | **+33.3% match, -57.81s error (97% reduction)** |
| **Details** | First 4 tracks misaligned | **SOLVED** - Found perfect assembly immediately |

---

## Key Findings

### 1. **MusicBrainz Release Differences**

Three albums matched to different MB releases between runs:

1. **Steely Dan - Gaucho:** Different despite identical parameters/results
   - Likely minor metadata differences between releases
   - Both achieved 100% match

2. **Daft Punk - TRON: Legacy Reconfigured:** Different despite identical parameters
   - Both achieved 100% match

3. **The Chemical Brothers - Surrender:** Different edition matched
   - Run 5: 11-track edition (38 segments over-segmented, 90.9% match)
   - Run 7: **28-track edition** (correct track count, 96.4% match)
   - Run 7 found better MB match AND better segmentation

4. **Journey - Trial By Fire:** Different edition matched
   - Run 5: 14-track edition (85.7% match)
   - Run 7: **16-track edition** (100% match via assembly)

### 2. **Parameter Optimization Differences**

Most albums used identical or similar parameters, but some notable differences:

| Album | Run 5 Params | Run 7 Params | Reason |
|-------|--------------|--------------|--------|
| Crosby, Stills & Nash | -60dB, 0.3s | -56dB, 0.05s | Enhanced Stage 3 found better assembly with different params |
| James Gang - Funk #49 | -60dB, 0.3s | -44dB, 3s | Completely different approach, assembly-driven |
| Journey - Trial By Fire | -46dB, 0.5s | -44dB, 0.05s | Different MB edition requires different segmentation |
| The Police - Zenyatta Mondatta | -50dB, 0.5s | -50dB, 0.05s | Same threshold, much shorter min duration |

### 3. **Track Count Improvements**

| Album | Run 5 Track Count | Run 7 Track Count | Status |
|-------|-------------------|-------------------|--------|
| Michael Jackson - Thriller | 8/9 ✗ | 9/9 ✓ | **Fixed** |
| James Gang - Funk #49 | 9/10 ✗ | 10/10 ✓ | **Fixed** |
| Journey - Trial By Fire | 15/14 ✗ | 16/16 ✓ | **Fixed** (different MB edition) |
| Kraftwerk - Trans-Europe Express | 9/8 ✗ | 6/6 ✓ | **Fixed** |
| Toto - Toto IV | 12/10 ✗ | 10/10 ✓ | **Fixed** |
| The Chemical Brothers - Surrender | 38/11 ✗ | 28/28 ✓ | **Fixed** (different MB edition) |

**Result:** +6 albums with correct track count (71.4% → 90.5%)

### 4. **Enhanced Stage 3 Impact**

**Albums improved by Enhanced Stage 3 (comprehensive assembly):**

| Album | Run 5 | Run 7 | Improvement | Assemblies Tested |
|-------|-------|-------|-------------|-------------------|
| **James Gang - Funk #49** | 50.0% | 100.0% | **+50%** | ~100+ |
| **Journey - Trial By Fire** | 85.7% | 100.0% | **+14.3%** | 83 (found in 1!) |
| **The Police - Zenyatta Mondatta** | 81.8% | 100.0% | **+18.2%** | 31/108 |
| **Toto IV** | 80.0% | 100.0% | **+20%** | ~100+ |
| **The Rolling Stones - Let It Bleed** | 66.7% | 100.0% | **+33.3%** | 1/162 (first try!) |
| **The Chemical Brothers - Surrender** | 90.9% | 96.4% | **+5.5%** | 96 |
| Crosby, Stills & Nash - Daylight Again | 93.3%* | 100.0% | **+6.7%** | 64/108 |
| Michael Jackson - Thriller | ~70%* | 88.9% | **+18.9%** | 99 |
| Billy Thorpe - Children of the Sun | ~80%* | 88.9% | **+8.9%** | ~100 |

*Intermediate Stage 2 results before previous Stage 3

### 5. **Mean Error Improvements**

**Top error reductions:**

| Album | Run 5 Error | Run 7 Error | Reduction |
|-------|-------------|-------------|-----------|
| **James Gang - Funk #49** | 98.37s | 3.61s | **-94.76s (96%)** |
| **The Rolling Stones - Let It Bleed** | 59.65s | 1.84s | **-57.81s (97%)** |
| **Toto IV** | 48.95s | 3.89s | **-45.06s (92%)** |
| **Journey - Trial By Fire** | 46.49s | 2.61s | **-43.88s (94%)** |
| **The Police - Zenyatta Mondatta** | 40.06s | 5.32s | **-34.74s (87%)** |

---

## Conclusions

### 1. **Enhanced Stage 3 Delivered**

The comprehensive assembly enhancement achieved exactly what it was designed for:
- Transformed 5 "stuck" albums (50-82%) to 100% perfection
- Improved 4 additional albums significantly
- No negative side effects on already-perfect albums

### 2. **MusicBrainz Matching Variability**

- 3-4 albums matched different MB releases despite similar/identical audio
- Usually benign (both releases are valid)
- Sometimes beneficial (better edition found, like Chemical Brothers 28-track)

### 3. **Parameter Consistency**

- Most albums (15/21) used identical or very similar parameters
- Differences typically driven by assembly finding better segmentation
- Default parameters (-57dB, 0.9s) still work well for many albums

### 4. **Overall Success**

Run 7 represents a **major milestone:**
- **98.8% average match** (vs 92.0% in Run 5)
- **100% Excellent confidence** (all albums ≥80%)
- **18/21 perfect matches** (vs 15/21 in Run 5)
- **90.5% correct track count** (vs 71.4% in Run 5)

The enhancement achieved this with **zero new detection algorithms** - simply by being more thorough in exploring the existing search space!
