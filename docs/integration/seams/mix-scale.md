# Effect mix scaling

The effect mix knob's Mix submenu offers Halve and Double after Dry, 25%,
Half, 75%, and Wet. Each command sets that one effect's mix in one undo step,
using its latest mix when clicked. If the effect has disappeared or the
change is smaller than 0.001, no command is sent. The patch contains only mix.

Halve divides the mix by two without rounding. Halve of a dry effect stays
dry. Double stops at fully wet. Halve and Double are disabled when their
change would be smaller than 0.001.

The existing Half preset still sets 50%. Dry, 25%, 75%, and Wet keep their
existing values. The enable lamp stays as it is; neither command enables or
disables the effect.
