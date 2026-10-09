# Windfall automation recording

Parity id: `wf-automation-recording`.

The transport's **Record automation** button arms recording for this session.
Its `aria-pressed` state follows `recordArmed`, which defaults to false, is
not persisted, and resets when the project is replaced. This arm is separate
from the audio Record button and its recording dialog.

`features/automation/record.ts` subscribes to committed project changes while
the transport bar is mounted. When armed and playing, a committed control
edit to a target currently driven by automation also writes its new stored
value into an existing automation clip. It does not record MIDI or
mouse-sample streams, create automations or clips, or write from the audio
callback. With the arm off, the old notice behavior remains: editing an
automated control sets the stored value used outside its clips.

The subscriber uses `editedWhileAutomated` and `drivingAutomation` to locate
the driving curve. It selects the covering clip with the earliest start at
the floored realtime song tick, with an inclusive start and exclusive end.
`toCurveTick` maps through the clip's start and offset; ticks outside
`0..MAX_SONG_TICKS` are ignored. The target's new stored value is normalized
using its automation range and clamped to `0..1`.

One `setAutomationPoints` replaces every existing point at that curve tick
with a single straight, non-held point, preserving all other points and
their order. An unchanged list sends no command. A re-entry guard prevents
the resulting project notification from recording again.

Run from `apps/desktop`:

```sh
pnpm test -- src/features/automation/record.test.tsx src/features/automation/live.test.tsx
```
