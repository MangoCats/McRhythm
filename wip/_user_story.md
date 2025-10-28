# User Story - Audio Import

Refrences: [REQ001 - Requirements](REQ001-requirements.md)

## Initial State

A new wkmp user has just installed the software.

Their music collection is organized under their /home/username/Music folder in various sub-folders, some representing artists, some representing albums, some random collections. Most files are named by song title, many filenames have additional information like artist or album or track number, most of those are accurate but some arr not.  The files themselves are mainly .mp3, some .ogg, some .flac, some .opus, some .wav.  There are also scattered image files in various formats, many are album cover art, but also random things and other random files here and there.  Within the music files there are various formats of tag information, a lot of ID3 mostly accurate, some not.  Most files represent a single song, some are continuous recordings of entire albums or live concerts.

## Next Step

In order to access the power of the WKMP program director, their music files need to be indexed with accurate MusicBrainz MBIDs.  This will be the index into the AcousticBrainz high level descriptors of the recordings that enable the program director to put together playlists which match the taste and mood of the listener at various times of day.

Ideally, this process will be as automated as possible for the user, gathering the available information about each audio file, identifying the MBID(s) of the recording(s) accurately through the most reliable information available.  
