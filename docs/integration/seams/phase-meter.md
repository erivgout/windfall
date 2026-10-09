# Phase meter

Windfall calls the panel **Phase meter** and its command-palette action
**Show phase meter** (`view.phase-meter`), following `view.spectrum`.

This displays the analyzer's existing **uncentered, normalized zero-lag
cross product**, as documented on `StereoStats.correlation`. It is not
Pearson's DC-removed correlation, not a goniometer plot, and not a calibrated
measurement. The analyzer is unchanged.

## Frame contract and publication

`RealtimeFrame.correlation` is `Option<f32>` in Rust and optional `number` in
the desktop TypeScript binding. The frame uses serde camelCase; the field
defaults to `None` when missing and is omitted from serialization when absent.
Absent means no valid correlation is available, rather than a measured zero.
Silence and nonfinite PCM have no correlation under the existing analyzer
contract. A valid measured zero is published as `Some(0.0)`.

`Controller::frame()` copies `snapshot.stereo.correlation` from the same
existing snapshot path used for spectrum publication. It accepts only finite
f64 values inside the inclusive range −1…+1, then converts to f32. Invalid
values are rejected before conversion, without clamping them into valid
evidence. No snapshot means no correlation.

Publication and validation run on the control-side frame caller, outside the
audio callback. The existing analyzer reader, worker, queue, tap, and snapshot
invalidation behavior are reused. This adds no audio-side work and changes
neither `RealtimeFrame.spectrum` nor `RealtimeFrame.spectrogram`.

## Desktop view

The horizontal meter runs from **Side · −1** on the left to **Mid · +1** on
the right, with a zero reference in the center and a two-decimal numeric
readout. Missing or invalid input hides the indicator, removes its current
numeric accessibility value, and says **No correlation available**.

The view subscribes through `useRealtime` and updates the indicator position,
numeric text, and accessibility attributes directly through DOM refs. It
does not store each frame in React state. The shared realtime store forwards
the newest correlation, clears an omitted value, and clears it on project
replacement. Closing/unmounting drops the subscription; other subscribers
keep the shared feed alive.

Minimal integration glue registers the action with the other view actions
and mounts the panel in the shell overlays. New UI strings use Windfall's
own labels.

## Validation

From Git Bash (`C:\Program Files\Git\bin\bash.exe`) at the repository root:

```sh
source scripts/msvc-env.sh
cargo test -p windfall-engine --test phase_meter -- --test-threads=1
```

The publication tests cover absence, measured zero, both endpoints, nonfinite
values, and out-of-range values. The public-frame test covers absence, +1 and
−1, seek/project/stream invalidation, silent input, and zero callback allocator
calls.

From `apps/desktop`:

```sh
pnpm test -- src/features/phase-meter/view.test.tsx src/features/phase-meter/panel.test.tsx
```

Desktop tests cover absent frames, +1, −1, measured zero, invalid values,
project replacement, no React render per frame, subscription cleanup on
unmount/close, and opening through the registered action.

Both commands above passed (two native tests and nine desktop tests).
The existing spectrum and spectrogram regressions also passed:

```sh
# Git Bash, repository root, after sourcing scripts/msvc-env.sh:
cargo test -p windfall-engine --test phase_meter --test spectrum_view --test spectrogram_view -- --test-threads=1
# apps/desktop:
pnpm test -- src/features/phase-meter/view.test.tsx src/features/phase-meter/panel.test.tsx src/features/spectrum/view.test.tsx src/features/spectrum/panel.test.tsx src/features/spectrogram/view.test.tsx src/features/spectrogram/panel.test.tsx
pnpm exec eslint src/features/phase-meter src/features/layout/app-shell.tsx src/features/layout/register-actions.ts src/lib/store/realtime.ts
pnpm exec prettier --check src/features/phase-meter
```

The regression runs passed seven native tests and 22 desktop tests; the focused
lint and formatting checks passed.

`cargo test -p windfall-engine --lib phase_meter_publication -- --test-threads=1`
is currently blocked before test execution
by existing fixture compilation errors in `project_preparation.rs`, `device.rs`,
and `render.rs`: missing channel/plugin model fields and mismatched render error
types. The nonfinite/out-of-range publication guard tests are added in
`controller.rs`, but cannot run until those unrelated fixtures compile.

The broader `pnpm typecheck` is also blocked by existing errors outside this
feature, including analysis backend methods and channel-rack/model bindings.
