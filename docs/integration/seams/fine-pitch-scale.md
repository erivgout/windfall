# Fine pitch scaling

The piano-roll fine-pitch lane header offers Half and Double for the selected
notes, or every note when none are selected. Half moves the pitch toward in tune
by dividing cents by two without rounding. In tune stays in tune, including notes
without expression. Double stops at one octave up or one octave down.

Release, modulation, and portamento stay as they are, along with every other
expression field. Octave down, Semitone down, In tune, Semitone up, and Octave up
stay as they are. The entries are hidden unless the lane is showing fine pitch.

Changes smaller than 0.001 cents are omitted. Each entry is disabled when there
is no editor context or nothing to change. Clicking reads the current notes and
sends one `updateNotes` command with expression-only patches when updates exist.
