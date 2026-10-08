# Piano randomizer source implementation

The selected-note tool dialog, note menu and action palette expose two native
tools: Randomize / humanize and Generate random notes. Both use a stored input
seed and a bounded, platform-independent integer RNG on the project thread.
No audio callback or system-time randomness participates. Next seed increments
the seed without applying or changing the captured selection.

Randomize adds independent signed offsets to pitch, velocity, pan, start and
length. Its controls specify maximum pitch semitones, velocity/pan percentage
points, timing ticks and relative length percentage. Zero leaves a property
unchanged. Timing/length remain within the native maximum pattern span;
pitch, velocity and pan clamp to their native ranges. Note ids, articulation,
release, fine pitch, modulation and color/MIDI groups stay with each note.

Generate replaces the captured selection inside its earliest start/latest
end. Each grid cell is a seeded hit or rest according to density. Hit pitches
come from a relative 12-class chord mask, rotated by a chosen root and bounded
by inclusive MIDI keys. Chord-map presets include major/minor triads, minor
seventh and chromatic; custom comma-separated semitones can also describe a
scale. Gate controls length, including the final partial cell. Velocity comes
from an explicit minimum/maximum range; pan and complete expression/color
data are copied from seeded source prototypes. Density zero removes the
selection; a sparse seed may also produce an all-rest result.

Native validation limits both input/output to 16,384 notes and generation to
16,384 cells before document or id mutation. A chord mask with no key in the
chosen range refuses instead of manufacturing another pitch. Generated notes
receive new document ids. All calculations run through the existing captured
TransformNotes transaction, so changed source notes refuse atomically and an
apply has one undo/redo step. Unselected notes remain outside the transaction.

Provisional NoteTransform bindings include both variants. The checked-in WASM
artifact remains the older advanced-fill source; rebuild with the combined
features during the later artifact pass. No builds, tests, QA, reviews or
artifact generators ran for this source implementation.
