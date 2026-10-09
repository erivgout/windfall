# Piano-roll Typing mode

Windfall's piano-roll toolbar exposes **Typing**, a session-only toggle registered
as `pianoRoll.typing`. **Backquote (U+0060)** was free in both the Windfall and FL
piano-roll keymaps and toggles the mode. Typing starts off and opening another
project turns it off through `onProjectReplaced`.

This does not record notes. It only auditions the open piano-roll channel through
the existing `auditionOn` and `auditionOff` functions. It does not dispatch pattern
edits, create undo steps, or introduce a backend command.

While Typing is on and the piano roll has focus, the chromatic keyboard is:

| Key               | A   | W   | S   | E   | D   | F   | T   | G   | Y   | H   | U   | J   | K   | O   | L   | P   |
| ----------------- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Semitones above A | 0   | 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  |

A initially plays MIDI 60 (C). **Z** lowers the octave and **X** raises it,
clamping A to MIDI 24–96. Octave changes affect subsequent presses; releasing a
held key stops its original pitch. Auto-repeat does not retrigger notes or
octave changes. Tool shortcuts are consumed while Typing is on; with Typing off,
the existing shortcuts, including **Z** for Zoom, retain their behavior. Text
fields and overlays retain their keyboard interaction.

`typing-keyboard.ts` owns the mode, octave, and held physical keys. The workspace
attaches its listeners to the piano-roll root. Keyup releases the held pitch;
turning Typing off, focus loss, window blur, channel changes, project replacement,
and workspace teardown release every held note. The toolbar toggle returns focus
to the grid. Preferences do not persist Typing mode.

Run the focused test from `apps/desktop`:

```sh
pnpm test -- src/features/piano-roll/typing-keyboard.test.tsx
```
