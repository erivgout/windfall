The piano roll halves or doubles the start of each selected note and keeps its length. Half moves a note toward the start of the pattern, rounding down to a whole tick, and a note at tick 0 stays there. Double of tick 0 stays at tick 0. Double stops where the note would reach the pattern limit.

Note lengths stay as they are. Only the start is patched; velocity and expression are preserved. Empty selections and selections whose starts would all stay unchanged do not dispatch an edit. The length commands still keep the starts.
