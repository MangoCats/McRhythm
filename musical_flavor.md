# Musical Flavor

**🎼 TIER 2 - DESIGN SPECIFICATION**

Defines musical flavor system and distance calculations. Derived from [requirements.md](requirements.md). See [Document Hierarchy](document_hierarchy.md).

> **Related Documentation:** [Architecture](architecture.md)

---

## Quantitative Definition

Musical flavor is a quantitative definition of a passage's musical characteristics in many dimensions. It is derived from the [AcousticBrainz high level](https://acousticbrainz.org/data#highlevel-data) characterization of [recording(s)](https://musicbrainz.org/doc/Recording) contained in a passage.  See: [Sample AcousticBrainz highlevel json object](sample_highlevel.json).

These characteristic values break down into two categories: 
- binary characteristics with two dimensions whose values add up to 1.0 such as: 
  - highlevel.danceability.all.danceable, not_danceable 
  - highlevel.gender.all.female, male
  - highlevel.mood_relaxed.not_relaxed, relaxed
- complex characteristics with 3 or more dimensions whose values add up to 1.0 such as:
  - highlevel.genre_electronic.all.ambient, dnb, house, techno, trance
  - highlevel.ismir04_rhythm.all.ChaChaCha, Jive, Quickstep, Rumba-American, Rumba-International, Rumba-Misc, Samba, Tango, VienneseWaltz, Waltz

## Flavor Distance
Musical flavor is primarily used to determine the "flavor distance" between two passages.  The shorter the flavor distance, the more musically similar the passages are deemed to be.

### Single Recording per Passage Calculation Example
For the case where the two passages to be evaluated each contain a single song, and therefore a single recording per passage:

- Step 1: create a list of all binary characteristics which are available for both passages, for example:
  - Passage A:
    - highlevel.danceability.all.danceable: 0.17, not_danceable 0.83 
    - highlevel.gender.all.female: 0.90, male: 0.10
    - highlevel.mood_relaxed.not_relaxed: 0.74, relaxed: 0.26
    - highlevel.xenophobia.all.not_xenophobic: 0.98, xenophobic 0.02 
  - Passage B:
    - highlevel.danceability.all.danceable: 0.93, not_danceable 0.07 
    - highlevel.gender.all.female: 0.33, male: 0.67
    - highlevel.mood_relaxed.not_relaxed: 0.78, relaxed: 0.22
  - list of binary characteristics available in both passages
    - danceability, gender, mood_relaxed
- Step 2: create vectors of FP64 (aka double) values of the first value in each characteristic in the list, for example:
  - Passage A: 0.17, 0.90, 0.74
  - Passage B: 0.93, 0.33, 0.78
- Step 3: calculate the square of the Euclidian distance between the two vectors:
  (0.17 - 0.93) * (0.17 - 0.93) + (0.90 - 0.33) * (0.90 - 0.33) + (0.74 - 0.78) * (0.74 - 0.78) = 0.9041
- Step 4: normalize by dividing by the number of characteristics in a vector: this is the binary characteristic distance
  0.9041 / 3 = 0.30136667
- Step 5: repeat steps 1 through 4 for each complex characteristic that is available for both passages, comparing all mutually available characteristics instead of just the first, for example:
  - Passage A: 
    - highlevel.genre_electronic.all.ambient: 0.11, dnb: 0.03, house: 0.47, techno: 0.02, trance: 0.37
    - highlevel.ismir04_rhythm.all.ChaChaCha: 0.02, Jive: 0.04, Quickstep: 0.08, Rumba-American: 0.16, Rumba-International: 0.32, Rumba-Misc: 0.09, Samba: 0.07, Tango: 0.05, VienneseWaltz: 0.03, Waltz: 0.14
  - Passage B: 
    - highlevel.animal_affinity.all.bird: 0.94, cat: 0.01, dog: 0.05
    - highlevel.genre_electronic.all.ambient: 0.11, dnb: 0.03, house: 0.01, techno: 0.82, trance: 0.03
    - highlevel.ismir04_rhythm.all.ChaChaCha: 0.02, Jive: 0.13, Quickstep: 0.36, Rumba: 0.23, Samba: 0.07, Tango: 0.05, Waltz: 0.14
- animal_affinity isn't available in both passages, so only genre_electronic and ismir04_rhythm characteristic distances will be calculated
  - genre_electronic shared characteristic list: ambient, dnb, house, techno, trance.  Create vectors:
    - Passage A: 0.11, 0.03, 0.47, 0.02, 0.37
    - Passage B: 0.11, 0.03, 0.01, 0.82, 0.03
  - calculation: (0.11-0.11)*(0.11-0.11)+(0.03-0.03)*(0.03-0.03)+(0.47-0.01)*(0.47-0.01)+(0.02-0.82)*(0.02-0.82)+(0.37-0.03)*(0.37-0.03) = 0.9672
  - normalize: 0.9672 / 5 = 0.19344 is the genre_electronic characteristic distance
  - ismir04_rhythm shared characteristic list: ChaChaCha, Jive, Quickstep, Samba, Tango, Waltz.  Note that missing characteristics in either list are ignored.  Create vectors:
    - Passage A: 0.02, 0.04, 0.08, 0.07, 0.05, 0.14
    - Passage B: 0.02, 0.13, 0.36, 0.07, 0.05, 0.14
  - calculation: (0.02-0.02)*(0.02-0.02)+(0.04-0.13)*(0.04-0.13)+(0.08-0.36)*(0.08-0.36)+(0.07-0.07)*(0.07-0.07)+(0.05-0.05)*(0.05-0.05)+(0.14-0.14)*(0.14-0.14) = 0.0865
  - normalize: 0.0865 / 6 = 0.0144167 is the ismir04_rhythm characteristic distance
- Step 6: average all complex characteristic distances:
  - (0.19344 + 0.0144167) / 2 = 0.10392833 is the complex characteristic distance
- Step 7: The average of the binary and complex characteristic distances: (0.9041 + 0.10392833) / 2 = 0.504014167 is the flavor distance between passage A and passage B.

### More than one Recording per Passage Calculation Example

In a case where a passage contains more than one recording, the characteristics of each recording are combined in a weighted average to calculate the passage's 
net characteristics for flavor distance calculation.  The weight of each recording is its runtime in the passage.

Example: A passage with a run-time of 1320 seconds contains 3 recordings, plus other uncharacterized audio / silence:
- Silence: run-time 5 seconds
- Recording A: run-time 287 seconds
- Silence: run-time 2 seconds
- Recording B: run-time 372 seconds
- Unrecognized audio: run-time 350 seconds
- Recording C: run-time 304 seconds

The total run-time of characterized audio in this passage is: 287 + 372 + 304 = 963 seconds.

The weights given to the recordings' characteristics would be:
- Recording A: 287/963 = 0.298027
- Recording B: 372/963 = 0.386293
- Recording C: 304/963 = 0.315680

Any characteristics not appearing in all three recordings will not be populated in the passage's net characteristics.

Once both passages' net characteristics have been determined, musical flavor distance is calculated the same as in the single recording per passage example,
just using the passages' net characteristics to do the computation.

Implementation note: Passages' net characteristics are computed when they are initially defined, then stored in the database.  If any of a passages' recordings' characteristics
are modified, then that passage's net characteristics are re-computed.  In the case of a bulk update of many recordings' characteristics, passages' net characteristics update is
performed after the bulk update of recordings' characteristics is complete.

#### Usage of Musical Flavor
When a user defines a preferred musical flavor, they select one or more passages which contain one or more songs.

The defined musical flavor is the average position of all passages defined by the user.

When passages are being considered for enqueuing:
1. zero probability passages (in song, or artist, or work, or other cooldown criteria) are ignored
2. all passages with non-zero probability of selection have their squared distance from the
   target musical flavor computed
3. candidate passages are sorted by distance from the target, with closest passages considered first.
4. the 100 closest (or all, when less than 100 non-zero probability candidates are available) are then considered
   based on their current computed probability, the product of their base probability and any applicable cooldown
   modifiers and other probability modifiers
5. a random number between zero and the sum of all considered passages' computed probabilities is selected
6. the list of candidates is iterated and each candidate's computed probability is subtracted from the random number
   the first candidate to reduce the random number to zero or below is the chosen passage for enqueuing.

