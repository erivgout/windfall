# Piano-roll Step entry

Windfall's piano-roll toolbar exposes **Step entry**, a session-only toggle
registered as `pianoRoll.stepEntry`. **Backslash (\\)** is free in the Windfall
and FL keymaps and toggles it. Enabling starts the cursor at the current
transport tick. Disabling clears the cursor; opening another project turns
Step entry off and clears it through `onProjectReplaced`.

Step entry uses Typing's chromatic letter row (A W S E D F T G Y H U J K O L P)
and current octave. A plays C; Z lowers and X raises the octave, clamped so
A stays between MIDI 24 and 96. Backquote still toggles Typing and never
inserts a note. Tool letter shortcuts are consumed while Step entry is on.

With the transport stopped, each accepted keydown auditions the open channel
and dispatches exactly one `addNotes` command containing one note at the
cursor. Its length comes from the shared snap and the pattern's meter at
the cursor; None uses one step (240 ticks). Local piano-roll snap divisions
do not override the shared choice. The cursor advances synchronously by that
length so rapid presses occupy successive positions. Auto-repeat and another
keydown of a held note do not insert again. Keyup releases the original pitch
and leaves the cursor unchanged. Notes cannot exceed the maximum pattern span.

During playback, keys only audition and the cursor does not advance.
With Step entry off, Typing retains its audition-only behavior and ordinary
tool shortcuts remain available. MIDI input is not recorded. Step entry
does not read MIDI hardware or add a backend command.

`step-entry.ts` owns the mode, cursor, and held keys. The workspace attaches
its capture listeners before Typing's to avoid duplicate auditions when both
modes are on. Text fields and overlays keep their normal keyboard behavior.
Disabling, focus loss, window blur, channel changes, project replacement,
and workspace teardown release held auditions. The mode is not persisted.

Run from `apps/desktop`:

```sh
pnpm test -- src/features/piano-roll/step-entry.test.tsx
pnpm test -- src/features/piano-roll/typing-keyboard.test.tsx
```
