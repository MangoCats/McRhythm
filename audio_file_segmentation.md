# Audio File Segmentation

**✂️ TIER 3 - IMPLEMENTATION SPECIFICATION**

Defines the workflow for segmenting a single audio file into multiple Passages. See [Document Hierarchy](document_hierarchy.md).

> **Related Documentation:** [Library Management](library_management.md) | [Requirements](requirements.md) | [Database Schema](database_schema.md)

---

## 1. Overview

**[AFS-OV-010]** This document specifies the process for segmenting a single, large audio file (e.g., a full CD rip, a vinyl album side) into multiple distinct Passages, each corresponding to a single Recording. The workflow is designed to be as automated as possible while providing the user with full control to review and manually adjust the results.

## 2. Segmentation Workflow

The process is a guided, step-by-step workflow within the McRhythm UI (Full version only).

### Step 1: Source Media Identification

**[AFS-SRC-010]** The user initiates the workflow by selecting a large audio file for import. The first prompt asks the user to identify the source media type to set appropriate defaults for silence detection. The options are:
- CD
- Vinyl
- Cassette (with Dolby Noise Reduction)
- Cassette (without Dolby Noise Reduction)
- Other

### Step 2: Automatic Silence Detection

**[AFS-SIL-010]** Based on the source media selection, the system uses default parameters to scan the audio file for periods of silence that indicate track boundaries.

**[AFS-SIL-020]** **Default Parameters:**
- **Silence Threshold:**
  - CD: -80dB
  - Vinyl: -60dB
  - Cassette (with Dolby): -70dB
  - Cassette (without Dolby): -50dB
  - Other: -60dB
- **Minimum Silence Duration:** 0.5 seconds

**[AFS-SIL-030]** The user interface shall present these default values and allow the user to edit them before starting the scan. This allows for tuning on a case-by-case basis (e.g., for a particularly noisy vinyl record).

**[AFS-SIL-040]** The system scans the file and creates preliminary Passage boundaries at the midpoint of each detected silent period. The result is a list of initial time-stamped segments.

### Step 3: MusicBrainz Release & Recording Matching

**[AFS-MB-010]** To automatically identify the segments, the system leverages the audio characteristics of the entire file and its derived segments.

1.  **AcoustID Fingerprinting:** The system generates an AcoustID fingerprint for the *entire* audio file using the ChromaPrint algorithm.
2.  **MusicBrainz Picard Integration:** This fingerprint is used to query the MusicBrainz database, similar to the functionality of MusicBrainz Picard, to find matching Releases (albums).
3.  **Candidate List:** The system presents the user with a list of the most likely Release matches, including album title, artist, and track count.

**[AFS-MB-020]** The user selects the most likely Release from the list. If no suitable match is found, the user can opt to proceed with manual segmentation and identification.

**[AFS-MB-030]** Once a Release is selected, the system attempts to align the automatically detected segments with the track list of the selected MusicBrainz Release. It does this by generating fingerprints for each *individual segment* and matching them against the Recordings on the release. This helps correct for errors in the silence detection (e.g., if two tracks have no silence between them).

### Step 4: User Review and Manual Adjustment

**[AFS-REV-010]** The user is presented with a review screen that shows:
- The audio waveform for the entire file.
- The proposed Passage boundaries overlaid on the waveform.
- The matched Recording/Song information for each segment from the selected MusicBrainz Release.

**[AFS-REV-020]** From this screen, the user has full manual control to:
- **Adjust Boundaries:** Drag the start and end points of any passage.
- **Add Passages:** Create new passage boundaries for missed tracks.
- **Delete Passages:** Remove incorrectly identified segments.
- **Re-assign Songs:** If a segment was matched to the wrong Recording, the user can choose the correct Recording from the release's tracklist.

### Step 5: Ingestion and Analysis

**[AFS-ING-010]** Once the user indicates they are satisfied with the segmentation and metadata, the system performs the final ingestion:

1.  **Passage Creation:** For each segment, a new Passage is created in the McRhythm database, linked to the source audio file and with the correct start/end times.
2.  **Song Association:** The appropriate Song record (including Recording, Artist, and Work) is associated with each new Passage.
3.  **Album Passage:** A single overarching Passage, encompassing the entire audio file, is also created. This allows the user to play the entire album side as a single unit if desired.
4.  **AcousticBrainz Lookup:** The system then queries the AcousticBrainz database for each new Recording ID to fetch the high-level characterization data (Musical Flavor).
5.  **Local Analysis:** If no AcousticBrainz data is available for a Recording, a local analysis job using Essentia is queued to compute the Musical Flavor locally.
