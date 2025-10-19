# Sample Rate Conversion Design

**Status:** NEW concept for integration into the system designs and specifications

## Problem Statement

When expressing time as an exact integer number of audio samples, different commonly used audio sample rates will have fractional numbers of samples. WKMP strives to achieve precise and high-quality sample-rate conversion (SRC) and to handle available audio sources as cleanly as possible.

### Common audio sample rates

Some sample rates that WKMP may be expected to play include:

- 8,000 Hz
- 11,025 Hz
- 16,000 Hz
- 22,050 Hz
- 44,100 Hz
- 48,000 Hz
- 88,200 Hz
- 96,000 Hz
- 176,400 Hz
- 192,000 Hz 

## Least Common Multiple

The least common multiple (LCM) of those common audio sample rates is: 28,224,000 Hz.  Time, expressed in 28,224,000 Hz "ticks", will always be able to exactly express an integer number of samples from all those sample rates.

## Practical considerations

28,224,000 is already large compared with a 32 bit integer, less than 76 seconds of 28,224,000 Hz ticks can be represented in a signed 32 bit integer.  However, even the ARM processors in small devices like a Raspberry Pi Zero 2 have 64 bit architectures.

Used with 64 bit integers, 28,224,000 Hz ticks represent more time than WKMP will be expected to process: over 10,000 years of time.

## Defined terms: tick, ticks

In the WKMP project, one tick represents 1/28,224,000 of a second.

Examples of usage:

- one sample of a 44.1KHz audio file is: 640 ticks long.
- one sample of a 192kHz audio file is: 147 ticks long.
- one sample of a 8kHz audio file is: 3528 ticks long.
- Midnight, January 1, 2029 (UTC): is 1861929600 seconds in Unix time, or 1861929600 * 28224000 = 52,551,101,472,000,000 ticks

### Specific usage within WKMP

Passage timing points shall be specified when these timing points
are:
- stored in the database
- passed in API calls
- used for sample number calculations

This precision of specification retains sample precise identification of time points without rounding, even when working_sample_rate is changed.

---

**Document Version:** 1.0
**Created:** 2025-10-19
**Status:** NEW concept for integration into the system designs and specifications
 