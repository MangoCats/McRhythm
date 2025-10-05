# Musical Flavor

> **Related Documentation:** [Requirements](requirements.md) | [Architecture](architecture.md) | [Document Hierarchy](document_hierarchy.md)

---

**🎼 TIER 2 - DESIGN SPECIFICATION**

Defines musical flavor system and distance calculations. Derived from [requirements.md](requirements.md). See [Document Hierarchy](document_hierarchy.md).

---

Musical flavor is a quantitative definition of a passage's characteristics in many dimensions. It is derived from the AcousticBrainz high level characterization of tracks (identified by MBID) which provides values of:

- danceability, danceable: 0-1
- gender, female: 0-1
- genre_dortmund, alternative: 0-1
- genre_dortmund, blues: 0-1
- genre_dortmund, electronic: 0-1
- genre_dortmund, folkcountry: 0-1
- genre_dortmund, funksoulrnb: 0-1
- genre_dortmund, jazz: 0-1
- genre_dortmund, pop: 0-1
- genre_dortmund, raphiphop: 0-1
- genre_dortmund, rock: 0-1
- genre_electronic, ambient: 0-1
- genre_electronic, dnb: 0-1
- genre_electronic, house: 0-1
- genre_electronic, techno: 0-1
- genre_electronic, trance: 0-1
- genre_rosamerica, cla: 0-1
- genre_rosamerica, dan: 0-1
- genre_rosamerica, hip: 0-1
- genre_rosamerica, jaz: 0-1
- genre_rosamerica, pop: 0-1
- genre_rosamerica, rhy: 0-1
- genre_rosamerica, roc: 0-1
- genre_rosamerica, spe: 0-1
- genre_tzanetakis, blu: 0-1
- genre_tzanetakis, cla: 0-1
- genre_tzanetakis, cou: 0-1
- genre_tzanetakis, dis: 0-1
- genre_tzanetakis, hip: 0-1
- genre_tzanetakis, jaz: 0-1
- genre_tzanetakis, met: 0-1
- genre_tzanetakis, pop: 0-1
- genre_tzanetakis, reg: 0-1
- genre_tzanetakis, roc: 0-1
- ismir04_rhythm, ChaChaCha: 0-1
- ismir04_rhythm, Jive: 0-1
- ismir04_rhythm, Quickstep: 0-1
- ismir04_rhythm, Rumba-American: 0-1
- ismir04_rhythm, Rumba-International: 0-1
- ismir04_rhythm, Rumba-Misc: 0-1
- ismir04_rhythm, Samba: 0-1
- ismir04_rhythm, Tango: 0-1
- ismir04_rhythm, VienneseWalts: 0-1
- ismir04_rhythm, Waltz: 0-1
- mood_acoustic, acoustic: 0-1
- mood_aggressive, aggressive: 0-1
- mood_electronic, electronic: 0-1 
- and many more...

These values break down into two categories: binary classifications with two dimensions that add up to 1.0, 
and higher dimensional characterizations with more than two dimensions which all add up to 1.0.

The musical flavor of a track is defined by its multi-dimensional position defined by these AcousticBrainz
high level characterization values.  Musical flavor is used by calculating the square of the Euclidian 
distance between two tracks' positions - lower distance means the tracks are more similar, higher 
distance means they are less similar.

Square of distance is calculated first based on all of the binary classifications.

Then a square of distance is calculated for each of the higher dimensional characterizations, and all of
those squares of distances are arithmetically averaged (Σ(diff²)/N) to come up with a single higher 
dimensional distance value.

Finally, a total distance is calculated as the sum of the binary classifications square of distance plus 
the average of all higher dimensional squares of distances.  The absolute number is not important, the
relative value between comparisons is what matters.

- Each song corresponds to a single AcousticBrainz "position" / Musical Flavor.

- A passage may contain zero or more songs
  - When a passage has no songs contained in it, it has no Musical Flavor and cannot be selected for enqueing based on Musical Flavor criteria
  - When a passage has one song, that song's position is used directly as the passage's musical flavor.
  - When a passage has more than one song, each dimensional value of each song is arithmetically averaged 
    to compute the passage's "position" / Musical Flavor.
    - This average position is stored in the passage's database entry rather than re-computing it every time needed.

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

