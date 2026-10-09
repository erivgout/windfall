# Portamento duration scaling

The desktop piano roll commands **Halve selected portamento** and **Double selected portamento** each change the selected notes' portamento in one undo step. Notes that are not selected stay as they are. An empty selection sends no edit. If every selected note would keep the same duration, no edit is dispatched.

Half uses the whole number of ticks at or below half the current duration, and 1 tick stays 1 tick. Double stops at the maximum pattern length (`MAX_PATTERN_TICKS`, 245760 ticks). A missing duration is 240 ticks, including notes without an expression.

Release, fine pitch, modulation, articulation, color group, and all other expression fields stay as they are. The existing 16th, 8th, quarter, and half-note portamento presets stay. The toolbar duration for newly drawn notes stays.
