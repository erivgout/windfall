# Note length scaling

The piano roll commands **Halve selected note lengths** and **Double selected note lengths** each change the stored length of the selected notes in one undo step. Notes that are not selected are unchanged. If every selected note would keep the same length, the command does not dispatch an edit.

Half uses the whole number of ticks at or below half the current length. A 1-tick note stays 1 tick. Double stops at the maximum pattern length (`MAX_PATTERN_TICKS`).

Note starts and expression stay as they are. Only length is patched. The fixed musical lengths (16th, 8th, quarter, half, and whole) are unchanged.
