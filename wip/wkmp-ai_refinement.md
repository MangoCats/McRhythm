# wkmp-ai refinement notes

## Focused purpose

### In scope

The scope of wkmp-ai is being reduced and defined to focus exclusively on automatic ingest of audio files: 

#### Stage One

- listing of all potential audio files within the user identified target folder
- analysis of each file's path and filenames to assist in song identification
- compute each file's data content's hash for identification of exact duplicates 
- extraction of ID3 and other available metadata 
- definition of [ENT-MP-030] passage or passages' [XFD-PT-010] start and [XFD-PT-060] end times based on silence found in the file combined with identified [ENT-MP-010] song or songs' durations.
- audio fingerprinting of each file for AcoustID identification
- identification of the [ENT-MP-010] song or songs (if any) contained in each file
- definition of [XFD-PT-030] lead-in and [XFD-PT-040] lead-out points based on audio levels near the beginning and end of each passage
- retrieval of AcousticBrainz high-level musical flavor data for each identified song
  - use of Essentia for musical flavor identification in cases where AcousticBrainz data is unavailable
- recording of all of the above information in the database

Initial development of wkmp-ai will focus on batch import of large numbers of files found in or under the root folder.  Later stage development will add:

#### Stage Two

- scanning of folders outside the root folder for audio files to ingest into the database
- moving or copying those files into appropriate folders under the root after identification of the songs they contain

### Out of scope

- Quality control of audio files: identifying passages which contain skips, gaps and other audio quality issues.  This shall be the function of a new microservice (to be defined in greater detail elsewhere): wkmp-qa Quality Assurance.

- Manual passage creation, definition and editing by users: identification of fade-in and fade-out points in addition to lead-in lead-out start and end times.  User directed revision of songs' MBID and other metadata.  This shall be the function of a new microservice (to be defined in greater detail elsewhere): wkmp-pe Passage Editor.

## New wkmp-ai import workflow definition

1) First step in the workflow is a verifcation that the stored AcoustID API key is valid.  When the API key is invalid, the user is prompted to either enter an valid API key (which will be verified and if invalid the user will be given as many attempts as they want to enter a valid key or acknowledge the lack), or acknowledge the lack and all AcoustID functionality later in the workflow will be skipped.  When an invalid API key is acknowledged this choice is remembered for the entire import session, but the next time wkmp-ai starts an import session the validity of the key will be checked again and if invalid the choice will be presented again.  When the API key is verified as valid, the process continues silently to the next step - though DEBUG messages in the console log describe each step and outcome during the API key validation process.

2) Next step, the user identifies a folder to scan, with a default choice of the root folder initially presented.  Stage One wkmp-ai only accepts the root folder or sub-folders contained somewhere under the root folder (any number of levels deep.)  Stage Two implementation of wkmp-ai will enable importing of audio files from outside the root folder to appropriate locations within the root folder after they are ingested and identified.

3) SCANNING: The identified folder is scanned and a list of all potential audio files contained in it and all sub-folders under it (any depth), symlinks, junction points and shortcuts are not followed.

4) PROCESSING: Each file in the list is processed through the sequential stages identified in 4xN) with several files proceeding in parallel as optimal for the processor wkmp-ai is running on.  The number of files to process is defined in the database settings table parameter: ai_processing_thread_count which is initialized as undefined (NULL) by default.  When an ingest process is started and encounters this parameter as undefined, it makes a best-guess estimate of the optimal number of threads to use based on best available information about the current processor's core count (ai_processing_thread_count = core count +1), then stores this value to the ai_processing_thread_count field in the database.  Future runs of wkmp-ai will use whatever value is present in that field, allowing (advanced) users to change that number of threads as desired to tune their system performance during imports: full utilization for minimal import time, or leave some capacity leftover so the computer can be used for other things while import is in progress.

4a) FILENAME MATCHING: The database files table is searched for a file with matching path, filename and created / modified metadata.  If such a matching file is found and its status is marked 'INGEST COMPLETE' then no further processing of this file is done (it is complete.)  If a matching files are found but none has a status of 'INGEST COMPLETE' then processing continues using the existing fileId of the file with the matching path, name and metadata.  Otherwise this file is assigned a new row with a new unique fileId in the files table of the database, its path, filename and metadata are recorded in the database and PROCESSING continues:

4b) HASHING: The hash of the file is computed and recorded in the database under this file's fileId.  The database files table is searched for files with a matching hash.  If any files with matching hashes are identified, this file's fileId is added to the list of matching_hashes stored by all files with matching hashes, and those files' IDs are added to this file's matching_hashes.  If any of the matching_hashes' files has a status of 'INGEST COMPLETE' then this file's status is marked 'DUPLICATE HASH' and the fileId of the file with 'INGEST COMPLETE' status is put at the front of the matching_hashes list.  Otherwise, PROCESSING continues.

4c) EXTRACTING: ID3 and any other relevant available metadata is extracted and recorded under this fileId in the files table of the database, merged with any existing metadata, overwriting any metadata with identical keys: new read takes precedent over existing metadata, old data is not removed unless directly replaced.  PROCESSING continues.

4d) SEGMENTING: audio data in the file is decoded and analyzed to find sections of silence.  The silence_threshold_dB is stored / read to / from the database settings table with a default value of 35dB RMS.  Similarly, silence_min_duration_ticks is stored / read to / from the database settings table in units of SPEC017 ticks with a default value equivalent to 300 milliseconds.  Any audio segment continuously below silence_threshold_dB for at least silence_min_duration_ticks is considered "silence" for the entire duration it remains below silence_threshold_dB.  Each segment of audio between identified silence is a potential passage.  SEGMENTING continues through the following 4dN) phases, working to ultimately decide which song or songs are contained in each potential passage or passages.  Adjacent potential passages may need to be merged to a single passage in (those rare) cases where the song contains embedded silence. Audio files with less than minimum_passage_audio_duration_ticks (a database settings table parameter with units of SPEC017 ticks, default value equivalent to 100 milliseconds) of non-silence are marked with status 'NO AUDIO' in the database files table and no further PROCESSING is done on the file.  Each potential passage must be at least minimum_passage_audio_duration_ticks long. 

4d1) FINGERPRINTING: the audio data of each potential passage is analyzed by ChromaPrint and potential MBID song identities are provided by AcoustID for each potential passage.

4d2) SONG MATCHING: Song metadata provided by MusicBrainz for the fingerprint MBID is combined with ID3 metadata, file and path names which may be used to look up other candidate MusicBrainz MBIDs and their metadata, compared with potential passage durations (including combinations of adjacent potential passages) to match the potential passages with their best candidate MBIDs.  The match_quality (f32 0.0-1.0) of the final selected MBID for the potential passages is rated: "High, Medium, Low or no confidence" based on how well the various potential passages' metadata matches the best candidate MBID.  All potential passages in the audio file are considered for potential matching assignments and the assignments with the best overall match_quality for all passages is the one used for all passages.  When necessary, potential passages are combined to create the final set of passages matched to MBIDs.  No confidence potential passages are given entries in the passages database table but they have no assocaited song.  Note that wkmp-pe and other microservices may define passages with multiple songs, but the wkmp-ai audio ingest process may only assign zero or one songs to a passage.

4d3) RECORDING: Once the best possible combination of MBID song assignements to potential passages has been determined, the potential passages (merged when appropriate) are recorded as new passages in the database passage table, associated with the audio file's fileId and their SPEC002 start time and end time in SPEC017 units of ticks. When MBIDs are not yet found in the songs table, new entries are created for them.  Existing songs entries with matching MBIDs have the new passages' passageIds added to their list of passages which contain the song.

4d4) AMPLITUDE: for each of the finalized passages, both those with a song defined and those without (but not those designated "NO AUDIO", read the passage's audio data from the start time forward until the audio amplitude exceeds lead-in_threshold_dB (a database settings table parameter with default value of 45dB), or 25% of  the total passage time has been analyzed, mark this point as the lead-in point for the passage in the database passage table under this passageId.  Similarly, analyze backwards from the end time until the audio amplitude exceeds lead-out_threshold_dB (a database settings table parameter with default value of 40dB), or 25% of the total passage time has been analyzed, mark this point as the lead-out point for the passage in the database passage table under this passageId.  Note, the wkmp-ai import process leaves passage fade-in, fade-out points empty, or unchanged when overwriting existing passage information.  Once this lead-in and lead-out determination has been finished for a passage, the passage's status is marked 'INGEST COMPLETE' in the status column on its row of the passage table in the database.  (Note: passages with no automatically associated song may receive 'FLAVORING' and/or song associations through user guided processes in other microservices like wkmp-pe, but that is outside the scope of wkmp-ai operations.)

4d5) FLAVORING: for each of the finalized passages, if the passage is not associated with a song, then the passage's SEGMENTING process is complete.  For passages which are associated with a song (MBID), if the song's entry in the database song table has a status of 'FLAVOR READY' then the passage's SEGMENTING process is complete. 

4d5a) Otherwise, an attempt is made to look up the high-level AcousticBrainz profile for the song using its MBID and if this is successful, the high-level profile for the song is stored (as a JSON object) in the flavor column of the song's row in the database songs table, and the song's status is marked 'FLAVOR READY' in the songs table and the song's flavoring process is complete and the associated passage's SEGMENTING process is complete.

4d5b) Otherwise, Essentia is used to analyze the audio data of the passage and create a high-level profile to record in the flavor column of the flavor column of the song's row in the database songs table, and the song's status is marked 'FLAVOR READY' in the songs table and the flavoring process is complete and the associated passage's SEGMENTING process is complete.

4d5c) If there is a problem with the Essentia process then the song's status is marked 'FLAVORING FAILED' in the songs table and the flavoring process is complete and the associated passage's SEGMENTING process is complete.

4e) PASSAGES COMPLETE: when all potential passages segmenting process, and any associated song's FLAVORING process is complete, then the audio file they came is marked 'INGEST COMPLETE' in the status column of the audio file's fileId row of the files table in the database.

5) FILES COMPLETE: when all files from the 3) SCANNING list have been dispositioned as 'INGEST COMPLETE' or 'DUPLICATE HASH' or 'NO AUDIO' then PROCESSING and the entire file ingest workflow is complete.

### New wkmp-ai import workflow user interface progress displays

For each of the stages in the workflow: SCANNING, PROCESSING, FILENAME MATCHING, HASHING, EXTRACTING, SEGMENTING, FINGERPRINTING, SONG MATCHING, RECORDING, AMPLITUDE, FLAVORING, PASSAGES COMPLETE, and FILES COMPLETE, a section of the user interface shall show a real-time (SSE driven) updated display of relevant status and statistics pertaining to what the stage has done so far while the stage is working.

SCANNING shall show: 'scanning' while the files scan is in progress and 'X total files found, Y potential audio files' when all files under the folder (X = the total number of files) have been classified as potential audio files (Y = the number of files that may contain audio) or not.

PROCESSING shall show: 'Processing X to Y of Z' where X is the count of files that have completed processing, Y is the count of files that have started processing, and Z is the total count of files from SCANNING.

FILENAME MATCHING shall show: 'N completed filenames found' where N is the count of the number of matching files found with status marked 'INGEST COMPLETE'

HASHING shall show: 'N hashes computed, M matches found' where N is the number of audio file hashes computed and M is the number of computed hashes which were found to match files with a status of 'INGEST COMPLETE'

EXTRACTING shall show: 'Metadata successfully extracted from X files, Y failures' where X is the number of file metadata extractions which successfully read at least one item of metadata from inside the file's data, and Y is the number of extractions which yielded no metadata.

SEGMENTING shall show 'X files, Y potential passages, Z finalized passages, W songs identified' where X is the number of audio files which have started the segmenting process, Y is the total number of potential passages identified in all of those files, and Z is the total number of finalized passages identified in all segmented audio files, and W is the number of songs that were successfully identified in all audio files processed.

FINGERPRINTING shall show 'X potential passages fingerprinted, Y successfully matched song identities' where X is the number of potential passages run through the Chromaprint process and Y is the number of successful song matches returned by AcoustID on those Chromaprint results.

SONG MATCHING shall show 'W high, X medium, Y low, Z no confidence' where W is the count of finalized passages matched to MBIDs with high confidence, X is the count of finalized passages matched to MBIDs with high confidence, Y is the count of finalized passages matched to MBIDs with low confidence, and Z is the count of finalized passages that did not match to MBIDs with at least low confidence.

RECORDING shall show a (vertically scrollable window containing a) list of song titles from the MBIDs assigned to passages, followed by 'in' and the path/filename of the audio file the song was found in.  For passages which did not successfully match a MBID song title, they are shown as "unidentified passage in N" where N is the audio file path and filename.  This list shall be updated as passages are recorded in the database by the 4d4) RECORDING part of the workflow.

AMPLITUDE shall show a (vertically scrollable window containing a) list of song titles, or "unidentified passage" each followed by the total length of the passage in seconds, then followed by 'lead-in' and the number of milliseconds in the SPEC002 lead-in duration, then followed by 'lead-out' and the number of milliseconds in the SPEC002 lead-out duration.

FLAVORING shall show 'W pre-existing, X by AcousticBrainz, Y by Essentia, Z could not be flavored' where W is the number of songs found with pre-existing 'FLAVOR READY' status, X is the number of songs which successfully retrieved flavor information from AcousticBrainz, Y is the number of songs successfully flavored by the Essentia process, and Z is the number of songs attempted to be flavored but failed both AcousticBrainz and Essentia.

PASSAGES COMPLETE shall show the number of finalized passages which have been completed (recorded in the passage table) through the SEGMENTING process

FILES COMPLETE shall show the number of files which have completed PROCESSING.

 