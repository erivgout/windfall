The desktop mixer Send level menu offers Previous preset and Next preset to
step one send through the existing presets from Off toward Loud. Off stays
off when moved previous. Loud stays loud when moved next. A level between
two presets moves to the neighboring preset in the chosen direction.
Levels within 0.001 of a preset count as that preset.

Each action reads the latest send level and dispatches a setSend command with
the same from and to and only the new gain. Other sends stay as they are.
