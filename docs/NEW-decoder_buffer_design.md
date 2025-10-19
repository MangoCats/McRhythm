# Decoder Buffer Design

**Status:** NEW concept for integration into the system designs and specifications

## Scope

The concepts described herein apply primarily to the wkmp-ap Audio Player microservice.

## Glossary

### Core Components

  **Decoder-Buffer Chain**: A complete audio processing pipeline assigned to a single passage, consisting of:
  - **Decoder**: Reads and decompresses audio data from the source file
  - **Resampler**: Converts audio from source sample rate to working_sample_rate (if needed)
  - **Fade In/Out Handler**: Applies crossfade curves and discards samples outside passage boundaries
  - **Playout Ring Buffer**: Stores processed stereo samples ready for mixing and output

  **Working Sample Rate**: The standardized sample rate (default 44,100 Hz) used internally for all mixing and playback operations. All source audio not already at this rate is converted to this rate before buffering.

  **Passage**: A defined segment of an audio file with specific start/end times and optional crossfade parameters.
   Each passage gets its own dedicated decoder-buffer chain.

  **Queue Position**: Location in the playback queue:
  - **Now Playing** (position 0): Currently audible passage
  - **Playing Next** (position 1): Next passage, may be crossfading with "now playing"
  - **Positions 2-11** (default): Passages being pre-decoded into buffers
  - **Positions 12+** (default): Waiting for assignment to decoder-buffer chains

### Sample Terminology

  **Sample**: A single audio measurement at one point in time. In this document:
  - **Mono sample**: Single f32 value representing one channel
  - **Stereo sample** or **(stereo) sample**: Pair of f32 values (left + right channels) = 8 bytes
  - All buffer sizes are specified in stereo samples unless explicitly stated otherwise

  **Sample-Accurate**: Timing precision measured in individual samples rather than milliseconds, ensuring exact repeatability. At 44.1kHz, one sample = ~0.0227ms precision.

### Buffer Types

  **Playout Ring Buffer**: Per-passage circular buffer holding decoded, resampled, fade-applied stereo samples awaiting playback (default: 15 seconds capacity per buffer).

  **Output Ring Buffer**: Single system-wide circular buffer between mixer and audio output device (default: 185ms capacity). Fed by mixer combining active passage buffers.

### Operating Modes

  **Playing Mode**: Mixer actively consumes samples from buffers, applies volume/crossfades, and feeds output ring buffer. Passages advance through queue as they complete.

  **Paused Mode**: Mixer outputs exponentially-decaying silence based on last playing sample values. No samples consumed from buffers; now playing passage remains in queue.  API queue manipulations are possible during Paused mode.

### Dataflow Concepts

  **Backpressure**: Flow control mechanism where full buffers signal decoders to pause until space becomes available, preventing memory overflow.

  **Buffer Headroom**: Reserved capacity (default: 10ms) at the top of each playout ring buffer to accommodate resampler output after decoder pause signal.

  **Decode Priority**: Ordering system ensuring passages closest to playback position receive decoding resources first. Evaluated every decode_work_period (default: 5 seconds).

  **Crossfade Window**: Time period where two passages play simultaneously, with "now playing" fading out while "playing next" fades in. Mixer combines samples from both buffers during this window.

## Overview

The Audio Player plays audio from source files that are encoded, often compressed.  The audio is decoded, converted to the working_sample_period when necessary, and buffered for playback as uncompressed stereo sample values.  Separate buffers are created for each [**[ENT-MP-030]**](REQ002-entity_definitions.md) passage.

The playback system reads audio from the buffers, applies [**[REQ-CTL-040]**](REQ001-requirements.md) volume, [**[XFD-OV-010]**](SPEC002-crossfade.md) crossfade and [**[REQ-XFD-030]**](REQ001-requirements.md) other amplitude modifications before sending the final computed stereo audio sample levels to the [**[SSP-OUT-010]**](SPEC013-single_stream_playback.md) output system.

A simplified view of the audio processing chain is:

```mermaid
graph LR
    API[API] --> Queue[Queue]
    Queue --> Decoder1[Decoder 1]
    Queue --> Decoder2[Decoder 2]
    Queue -.-> DecoderDots[...]
    Queue --> DecoderN[Decoder N]
    
    Decoder1 --> Resampler1[Resampler 1]
    Decoder2 --> Resampler2[Resampler 2]
    DecoderDots -.-> ResamplerDots[...]
    DecoderN --> ResamplerN[Resampler N]
    
    Resampler1 --> Fade1[Fade In/Out 1]
    Resampler2 --> Fade2[Fade In/Out 2]
    ResamplerDots -.-> FadeDots[...]
    ResamplerN --> FadeN[Fade In/Out N]
    
    Fade1 --> Buffer1[Buffer 1]
    Fade2 --> Buffer2[Buffer 2]
    FadeDots -.-> BufferDots[...]
    FadeN --> BufferN[Buffer N]

    Buffer1 --> Mixer[Mixer]
    Buffer2 --> Mixer
    BufferDots -.-> Mixer
    BufferN --> Mixer
    
    Mixer --> Output[Output]
    
    style DecoderDots fill:none,stroke:none
    style ResamplerDots fill:none,stroke:none
    style BufferDots fill:none,stroke:none
```
The system allocates maximum_decode_streams decoder-buffer chains.  Each chain is assigned 1:1 to a passage in the queue, when the passage is less than maximum_decode_streams or less from the first position in the queue.

The first position in the queue is also referred to as the "now playing" passage.

The next position in the queue is also referred to as the "playing next" passage.

Note that each decoder-buffer chain is NOT associated with a particular position in the queue.  Each decoder-buffer chain is assigned to a passage in the queue and remains associated with that passage as the passage advances toward the now playing queue position.


## Related Documents

These documents define essential terms and concepts described herein. Please also read and understand these documents before taking action based on this document:
- [SPEC002 Crossfade](SPEC002-crossfade.md)
- [REQ002 Entity Definitions](REQ002-entity_definitions.md)
- [NEW Sample Rate Conversion](NEW-sample_rate_conversion.md)

## Operating Parameters

These defined values are stored in the global settings table of the database, where they are read once at startup for run-time use.  Changes of operating parameters' values may require a complete system restart for proper operation.

- working_sample_rate: the sample rate that all decoded audio is converted to before buffering.  Default value: 44100Hz.  When audio comes out of the decoder at the working_sample_rate, the sample rate conversion process shall be bypassed.

- output_ringbuffer_size: the maximum number of (stereo) samples that the output ring buffer (between the mixer and the output) can contain. Default value: 8192, equivalant to 185ms of audio at 44.1kHz.

- output_refill_period: the number of wall clock milliseconds between mixer checks of the output ring buffer state.  Each output_refill_period the mixer passes enough (stereo) samples to fill the output ring buffer from the active decoder-buffer chain(s) and its mixer algorithms.  Default value: 90ms, equivalant to just under half of the output ring buffer capacity.

- maximum_decode_streams: the maximum number of audio decoders that will operate on passages in the queue.  Default value: 12.  When the queue has more passages than this, only the passages closest to being played will be decoded into buffers awaiting play, other passages will start decoding when they advance to within maximum_decode_streams of the "now playing" first position in the queue.  

- decode_work_period: the number of wall clock milliseconds between decode job priority evaluation.  Default value: 5000.  Once every decode_work_period the currently working decoder is paused and the list of pending decode jobs is evaluated to determine the highest priority job and switch to decoding it.  If the currently working decoder is still the highest priority job, then it continues.  When a decoding job reaches the end of the passage, or receives a buffer full indication from the playout buffer it is filling, it pauses and the next highest priority decoding job is resumed immediately.  The decode_work_period serves to allow decodes to continue uninterrupted while still serving the highest priority jobs often enough to ensure their buffers do not run empty.

- playout_ringbuffer_size: the number of (stereo) samples that the decoded / resampled audio buffers contain.  Default value: 661941, equivalent to 15.01 seconds of audio at 44.1kHz.  At 8 bytes per sample, with 12 playout buffers total, that's 60MB of playout buffer.

- playout_ringbuffer_headroom: the number of (stereo) samples that the buffer reserves to handle additional samples that may arrive from the resampler after the decoder pauses due to a buffer full condition.  Default value 441, equivalent to 0.01 seconds of audio at 44.1kHz.

- pause_decay_factor: when in pause mode, instead of playing samples from the decoder-buffer chain(s), the mixer starts at the last played (stereo) sample values and recursively multiplies them by this pause_decay_factor at every subsequent sample, creating an exponential decay to zero, hopefully reducing audible "pop" from the sudden stop of going to pause mode.  Default value: 0.96875 (31/32).

- pause_decay_floor: when the absolute value of the pause mode output sample values drop below this pause_decay_floor, the mixer no longer bothers doing the multiplication and simply outputs 0.0  Default value: 0.0001778

## Dataflow

### Backpressure

Playback has two modes: Playing and Paused.  When in playing mode, audio data is fed from the buffers to the mixer and then to the output system via the output ringbuffer.  When paused, the mixer outputs silence to the output ringbuffer: a flat line, and no samples are consumed from the buffers. When no samples are consumed from the buffers, the buffers do not finish playing and so they are not removed from the queue.

When in Playing mode, the mixer operates on samples from one or more buffers, calculating values to pass to the output ring buffer.  When the buffer associated with a passage in the queue reaches its end point, the passage is removed from the queue and the next passage in the queue either starts playing if there was no [crossfade](SPEC002-crossfade.md) between them, or continues playing if it already started as a crossfade.

### API -> queue

The wkmp-ap audio player is given passage definitions to enqueue via the API, either from the user interface, the program director, or other sources.  This queue of passage definitions is served in a First In First Out (FIFO) order for decoding and buffering.

### Decoders

Each passage defines a portion of an audio file to play.  When a passage's position in the queue comes up within maximum_decode_streams of the first (now playing) position, an available decoder-buffer chain is assigned to it and it becomes eligible for decoding.

Each passage gets a dedicated decoder instance which works through the audio file, pausing when its buffer is full, resuming as data is read from the buffer into the mixer.

Decoding is handled serially in priority order, only one decode runs at a time to preserve cache coherency and reduce maximum processor loads, to avoid spinning up the cooling fans.

Decoding starts from the beginning of the audio file, even if the start point is after that.

Seek time estimation in compressed file decoding can be inaccurate, especially for variable bit rate encoded files.  Once the audio data has been decoded, it is "exact sample accurate" repeatable and predictable.  Timing for passage start, end, fade in, fade out, lead in and lead out is all handled with exact sample accuracy for repeatability and predictability.

### Resampling

When the audio data is not at the working_sample_rate, it is resampled to put it at the working_sample_rate before it is passed to the Fade In/Out handler.

When the audio data is at the working_sample_rate, it is passed straight through from the Decoder to the Fade In/Out handler.

### Fade In/Out handlers

The Fade In/Out handler has several functions:

- Samples received before the passage start time are discarded, not buffered.
- When fade-in duration is > 0 then samples between the start time and the fade-in end point have the fade-in curve applied before they are buffered
- Through the passage until the fade-out start point samples are buffered exactly as they are received
- When fade-out duration is > 0 then samples between the fade-out point and the end time have the fade-out curve applied
- When the end time sample has been received by the Fade In/Out handler, the Decoder is informed that the passage is complete and no further decoding is needed.

### Buffers

The buffer of each decoder-buffer chain holds playout_ringbuffer_size stereo samples.  As the parameter name implies, these are ring buffers.

The ring buffer starts empty when its processing chain is assigned to work with a particular passage.

Whenever the buffer is empty, the mixer cannot take samples out of it.

If the mixer does attempt to take a sample from an empty buffer, the buffer returns the same value that the last successful "get next sample" call received along with a "buffer empty" status informing the mixer of the situation.

Whenever the buffer has playout_ringbuffer_headroom or fewer samples of available free space (is nearly full), the decoder is told to pause
decoding until more than playout_ringbuffer_headroom samples are available.

When the sample corresponding to the passage end time is removed from the buffer, the buffer informs the queue that passage playout has completed; the passage should now be removed from the queue.

### Mixer

The mixer implements several functions:

- every output_refill_period refills the output ring buffer for cpal to output
- implememts play and pause mode
- when in play mode:
  - takes samples from the "now playing" passage buffer 
  - when in a lead-out / lead-in crossfade, also takes samples from the "playing next" passage buffer and adds them to the "now playing" sample values
  - multiplies the sample values by the master volume level
  - when "fading in after pause" also multiplies the sample values by the current fade in curve value
- when in pause mode, outputs near flatline silence
  - takes the last (stereo) sample values and repeats them with an exponential decay toward zero by multiplying the values by the pause_decay_factor at each sample sent to the output ring buffer.  This reduces the "pop" effect that can occur from an instant transition to zero.
  - each entry to pause mode starts at the last playing mode (stereo) sample values and decays through the duration of the pause until the absolute value of the current sample value is less than the pause_decay_floor, at which point the mixer simply outputs zeroes.

### Output

The mixer creates a single output stream which is fed to the output ring buffer for consumption by the cpal audio output library.

## Sample Format

All stages from the decoder output to the mixer output work with stereo f32 sample values.  This is the preferred format both for the symphonia decoder and for the cpal output handler.

----

**Document Version:** 1.0
**Created:** 2025-10-19
**Status:** NEW concept for integration into the system designs and specifications
