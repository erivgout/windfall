# Stage display views

Display views for `vis-fruity-big-clock` and `vis-fruity-db-meter`, labeled
**Large clock** and **Large master meter** in Windfall. Open either from the
command palette (View → Command palette). Only the open view is mounted;
closing it removes its realtime subscription.

## Integration

- `apps/desktop/src/features/stage/actions.ts` registers `view.largeClock` and
  `view.largeMasterMeter` through the existing action registry. The shell's
  `registerAllActions` owns registration and disposal.
- `StageViews` is mounted in the shell's shared `Overlays` host. Its local
  store holds only which display is open and is cleared on action disposal.
- `LargeClock` follows `PositionReadout`: `useRealtime`, the project and
  transport stores, and `formatMusicalPosition` select the song or active
  pattern's time signature and meter segments. Song time uses `songTempoMap`;
  pattern time uses the stored tempo through `ticksToSeconds`. Both texts are
  written directly to DOM refs, without React state updates per frame.
- `LargeMasterMeter` uses `meterFeed(MASTER_TRACK)` and a tall, vertical stereo
  `LevelMeter`. One subscription supplies both meter channels and the numeric
  left/right readouts. The numbers are the current frame's peak gains in dBFS
  (`gainToDb`), with silence shown as −∞. The bars keep `LevelMeter`'s existing
  release, peak hold, and clip threshold: peaks above unity latch the clip
  light until clicked. Replacing the project resets the meter and readouts.

These views do not add a new analyzer tap, spectrum, or hardware calibration.
They display existing transport and master peak telemetry; dBFS is relative
to digital full scale, not a calibrated hardware or sound-pressure reading.
They do not change the transport controls or audio processing.

## Verification

From `apps/desktop`:

```powershell
pnpm test src/features/stage/stage.test.tsx
```

The focused tests use a fake backend realtime subscription and controlled
animation frames with the real `useRealtime`, `meterFeed`, and `LevelMeter`.
They cover known tick/meter/tempo formatting, parity with the small readout
across song/pattern changes, independent stereo values, the click-to-clear
clip latch, project replacement, registry opening/closing, no React renders
per frame, and subscription/animation cleanup on unmount.
