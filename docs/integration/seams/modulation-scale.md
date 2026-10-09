# Modulation scaling

The modulation lane header offers Half and Double for the selected notes, or
every note when none are selected. Half moves the value toward center by halving
its distance from 0.5. Center stays centered, including notes without expression.
Double stops at low or high.

The other axis, fine pitch, release, and every other expression field stay as
they are. Low, Center, and High stay as they are. The entries are hidden unless
the lane is showing modulation X or modulation Y.

Changes smaller than 0.001 are omitted. Each entry is disabled when there is no
editor context or nothing to change. Clicking sends one `updateNotes` command
with expression-only patches when updates exist.
