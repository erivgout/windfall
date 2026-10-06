# Windfall audio control kit

Knobs, faders, meters, step grids, a keyboard, a waveform and an envelope editor for React, Tailwind v4 and shadcn. Every panel of Windfall is built from these, and they are published as a shadcn registry so other audio projects can use them.

The kit is MIT-licensed (see `LICENSE`). The Windfall app itself is GPL-3.0.

```tsx
import { Fader, Knob, LevelMeter, percentUnit } from "@/components/audio"
```

All components are controlled: values come in as props and changes go out through callbacks. They import nothing but React, `cn` from `@/lib/utils`, `class-variance-authority` and each other. Each one takes `className` and passes other props to its root element.

Run `pnpm dev` and open `/?view=kit` to see every component in every state.

## Installing from the registry

`apps/desktop/registry.json` declares one item per component. Each item lists every file it needs and the CSS variables it reads, so items install on their own and need no other registry item.

The shadcn CLI reads a GitHub repository as a registry when `registry.json` sits at the repository root. That root file does not exist yet. It only has to include this one:

```json
{
  "$schema": "https://ui.shadcn.com/schema/registry.json",
  "name": "windfall",
  "homepage": "https://github.com/windfall-daw/windfall",
  "include": ["apps/desktop/registry.json"]
}
```

With it in place and the repository public:

```bash
npx shadcn add windfall-daw/windfall/knob
npx shadcn add windfall-daw/windfall/audio-kit   # everything
```

Files install to `components/audio/`. To check the registry, run `pnpm dlx shadcn@latest registry validate` and `pnpm dlx shadcn@latest build -o node_modules/.tmp/registry` from `apps/desktop`. The built JSON files can also be served from any web server and added by URL.

The token values in `registry.json` are copies of the ones in `src/index.css`. When a token changes in `src/index.css`, copy the new value into `registry.json`.

| Item | Files |
|---|---|
| `knob` | `knob.tsx` and the shared value files |
| `pan-control` | `pan-control.tsx`, `knob.tsx` and the shared value files |
| `fader` | `fader.tsx` and the shared value files |
| `number-field` | `number-field.tsx` and the shared value files |
| `level-meter` | `level-meter.tsx`, `canvas.ts`, `units.ts` |
| `waveform` | `waveform.tsx`, `canvas.ts`, `units.ts`, `use-drag-value.ts` |
| `envelope-editor` | `envelope-editor.tsx`, `units.ts` |
| `step-grid` | `step-grid.tsx`, `step-button.tsx` |
| `step-button`, `toggle-led`, `piano-keyboard` | one file each |
| `use-drag-value` | the shared value files: `use-drag-value.ts`, `value-input.tsx`, `units.ts` |
| `audio-units` | `units.ts` |
| `audio-kit` | everything, with `index.ts` |

## The interaction model

`Knob`, `PanControl`, `Fader`, `NumberField` and the `Waveform` region handles share one model, implemented once in `useDragValue`.

### Shared props (`ValueControlProps`)

| Prop | Type | Notes |
|---|---|---|
| `value` | `number` | Required. Clamped to the range for display. |
| `onValueChange` | `(value: number) => void` | Called continuously while the value changes. Never called with a value outside the range, and never with the value it already has. |
| `onGestureStart` | `() => void` | Called before the first change of a drag, key hold, wheel burst, reset or typed entry. |
| `onGestureEnd` | `() => void` | Called when that gesture is over. |
| `defaultValue` | `number` | Restored by double-click and Ctrl/Cmd-click. Without it, those do nothing. |
| `min`, `max` | `number` | Default 0 and 1. |
| `step` | `number` | Snaps values to `min + k * step`. Leave unset for a continuous control. |
| `keyStep` | `number` | With a `step`: what an arrow key or a wheel notch moves by. Shift then moves by one `step`. |
| `scale` | `"linear" \| "log" \| ValueScale` | How the value maps to travel. `ValueScale` is `{ toNormalized(value, min, max), fromNormalized(normalized, min, max) }`. `logScale` needs `min` above 0. `powerScale(exponent)` and `faderTaper` are ready-made. |
| `format` | `(value: number) => string` | The readout and `aria-valuetext`. |
| `parse` | `(text: string) => number \| null` | Turns typed text into a value. It must understand what `format` prints. The default reads the first number in the text. |
| `disabled` | `boolean` | |
| `aria-label` | `string` | Or pass a visible `label`, which labels the slider. |

Spread a unit to set `format` and `parse` together: `<Knob {...percentUnit} />`.

### Gestures

Every `onValueChange` call happens between an `onGestureStart` and an `onGestureEnd`. A gesture is one drag, one held key, one burst of wheel movement, one reset or one typed entry. Use the pair to group changes into one undo step:

```tsx
<Fader
  value={gain}
  onGestureStart={() => (gestureId = nextGestureId())}
  onValueChange={(gain) => dispatch({ setGain: gain }, gestureId)}
  onGestureEnd={() => (gestureId = undefined)}
/>
```

A press that changes nothing opens no gesture. A control that unmounts in the middle of a gesture still calls `onGestureEnd`.

### Pointer

- Drag up to raise the value and down to lower it (right and left for horizontal controls). The pointer is captured, so the drag keeps working outside the control and outside the window.
- A knob covers its range in 200 pixels (`dragRange`). A fader cap and a region handle follow the pointer exactly.
- Shift makes the drag ten times finer. Shift can be pressed or released in the middle of a drag without a jump.
- Double-click or Ctrl/Cmd-click restores `defaultValue`. With touch and pen, a double tap does.
- The wheel changes the value only when the control has focus, or after the pointer was moved onto it and no page scroll is under way. A control that scrolls under a resting pointer leaves the wheel to the page.
- Mouse, touch and pen all work. The controls set `touch-action: none`, so a touch drag does not scroll the page.

### Keyboard

| Key | Continuous control | Control with a `step` |
|---|---|---|
| Arrow up / right, arrow down / left | 1% of the travel | `keyStep`, or one step |
| Shift + arrow | 0.1% of the travel | one step |
| Page up / page down | 10% of the travel | ten key steps |
| Home / End | `min` / `max` | `min` / `max` |
| Enter | opens the text entry with the readout selected | same |
| A digit, `-`, `+`, `.` or `,` | opens the text entry starting with that character | same |

In the text entry, Enter and leaving the field commit, and Escape cancels. Typed values are clamped to the range. Keys with Ctrl, Cmd or Alt are left to the app.

### Accessibility

Each control is a `role="slider"` with `aria-valuemin`, `aria-valuemax`, `aria-valuenow`, `aria-valuetext` and `aria-orientation`, in the tab order, with a visible focus ring. Disabled controls leave the tab order and set `aria-disabled`.

### Building your own control

```tsx
const control = useDragValue({ value, onValueChange, min, max, format })
return (
  <div>
    <div {...control.sliderProps} aria-label="Depth" className="touch-none" />
    {control.editing ? <ValueInput {...control.entryProps} /> : control.text}
  </div>
)
```

`useDragValue(options)` takes `ValueControlProps` plus `orientation`, `dragRange` (pixels, or a function of the element), `fineFactor`, `trackOvershoot`, `detents`, `doubleClick` (`"reset" | "edit" | "none"`), `wheel` and `editable`. It returns `value`, `normalized` (0 to 1), `text`, `dragging`, `editing`, `sliderProps`, `entryProps`, `startEditing()`, `reset()` and `change(value)`.

The pure helpers are exported too: `clampValue`, `snapValue`, `valueToNormalized`, `normalizedToValue`, `stepValue`, `dragTravel`, `parseEntry`, `defaultFormat`.

## Units (`units.ts`)

| Function | Example |
|---|---|
| `gainToDb(gain)` | `gainToDb(1)` is 0, `gainToDb(0)` is `-Infinity` |
| `dbToGain(db)` | `dbToGain(-6)` is about 0.501, `dbToGain(-Infinity)` is 0 |
| `formatDb(db, digits = 1)` | "−6.0 dB", "+3.0 dB", "−∞ dB" at or below −120 |
| `formatGain(gain, digits = 1)` | linear gain as dB: `formatGain(2)` is "+6.0 dB" |
| `formatPan(pan)` | "L50", "C", "R100" |
| `formatPercent(value, digits = 0)` | `formatPercent(0.5)` is "50%" |
| `formatMs(ms)` | "1.5 ms", "250 ms", "1.25 s" |
| `formatSemitones(semitones, digits = 0)` | "+7 st", "−12 st" |
| `formatHz(hz)` | "440 Hz", "8.00 kHz" |
| `parseNumber`, `parseDb`, `parseGain`, `parsePercent`, `parsePan`, `parseMs`, `parseSemitones`, `parseHz` | the matching parsers; each returns `null` for unreadable text |

Readouts use the real minus sign (U+2212). The parsers accept a plain hyphen, a comma as the decimal mark, and "-inf".

Units pair a formatter with its parser: `dbUnit`, `gainUnit`, `panUnit`, `percentUnit`, `msUnit`, `semitonesUnit`, `hzUnit`.

## Knob

```tsx
<Knob label="Cutoff" value={hz} onValueChange={setHz}
      min={20} max={20000} scale="log" defaultValue={8000} {...hzUnit} />
```

`ValueControlProps` plus:

| Prop | Type | Default | Notes |
|---|---|---|---|
| `size` | `"sm" \| "md" \| "lg"` | `"md"` | 24, 36 and 52 pixels. |
| `bipolar` | `boolean` | `false` | Draws the arc from the center outwards and makes the drag stick to the center. |
| `center` | `number` | middle of the range | The value a bipolar arc grows from. |
| `label` | `ReactNode` | | Shown under the knob and used as its accessible name. While dragging it shows the value. |
| `showValue` | `boolean` | `false` | Always show the value under the knob. Double-click the value to type. |
| `color` | CSS color | `--wf-control-fill` | The arc color. |
| `dragRange` | `number` | `200` | Pixels of travel for the full range. |

## PanControl

A `Knob` fixed to −1…1, bipolar, with `defaultValue` 0, `formatPan` and `parsePan`. It takes the `Knob` props except `min`, `max`, `bipolar` and `center`.

```tsx
<PanControl size="sm" value={pan} onValueChange={setPan} />
```

## Fader

With no `min`, `max` or `scale`, a fader is a gain fader: linear gain 0 to 2.0 on the dB taper, with dB marks, a dB readout, typed dB entry and 0 dB as the default. Pass a range to use it for anything else.

```tsx
<Fader value={gain} onValueChange={setGain} showValue aria-label="Kick level"
       className="h-56"
       meter={<LevelMeter ref={meterRef} taper={gainToFaderPosition} />} />
```

`ValueControlProps` plus:

| Prop | Type | Default | Notes |
|---|---|---|---|
| `orientation` | `"vertical" \| "horizontal"` | `"vertical"` | Default size is `h-44` or `w-48`. Override with `className`. |
| `ticks` | `FaderTick[] \| false` | dB marks for a gain fader | `FaderTick` is `{ value, label?, strong? }`, with `value` in the fader's own unit. Labels that would overlap are dropped; their lines stay. |
| `label` | `ReactNode` | | Shown under the fader. |
| `showValue` | `boolean` | `false` | Shows the value under the fader. Double-click it to type. |
| `meter` | `ReactNode` | | Rendered beside the travel and aligned to it. |
| `color` | CSS color | `--wf-control-fill` | |

The taper:

- `gainToFaderPosition(gain)` maps linear gain 0…2.0 to travel 0…1. Silence is 0, −48 dB is 0.07, −24 dB is 0.27, −12 dB is 0.48, −6 dB is 0.62, 0 dB is 0.78 and +6 dB is 1.
- `faderPositionToGain(position)` is its inverse.
- `faderTaper` is the pair as a `ValueScale`, for a knob that should feel like a fader: `<Knob min={0} max={2} scale={faderTaper} {...gainUnit} />`.
- `FADER_MAX_GAIN` is 2. `FADER_DB_TICKS` is the default marks: +6, 0, −6, −12, −24, −48, −∞.

Dragging anywhere on the fader moves the cap from where it is. A click on the track does not make the level jump.

## LevelMeter

A peak meter on a canvas. It is fed outside React and never re-renders for a new value.

```tsx
const meter = useRef<LevelMeterHandle>(null)
// in the realtime feed, about 60 times a second:
meter.current?.set(frame.meters[2 * track], frame.meters[2 * track + 1])

<LevelMeter ref={meter} />
```

Imperative API (`LevelMeterHandle`):

| Method | Notes |
|---|---|
| `set(left, right?)` | Linear peak values. With one argument both channels get it. Call it as often as values arrive; the loudest value since the last frame is drawn. |
| `clearClip()` | Turns the clip light off. |
| `reset()` | Drops the levels, the held peaks and the clip light. |

Or pass `subscribe`: a function that receives a listener `(left, right?) => void` on mount and returns a function that stops the feed.

| Prop | Type | Default | Notes |
|---|---|---|---|
| `channels` | `1 \| 2` | `2` | |
| `orientation` | `"vertical" \| "horizontal"` | `"vertical"` | Default size is `h-32 w-2.5`. Override with `className`. |
| `minDb`, `maxDb` | `number` | `-60`, `6` | The ends of the scale. |
| `taper` | `(gain: number) => number` | | Replaces the dB scale. Pass `gainToFaderPosition` to line the meter up with a fader's marks. |
| `midDb`, `highDb` | `number` | `-12`, `0` | Where the yellow and the red zones begin. |
| `releaseDbPerSecond` | `number` | `20` | The level rises at once and falls at this rate. |
| `peakHoldMs` | `number` | `1000` | How long the peak line holds before it falls. 0 hides it. |
| `clipGain` | `number` | `1` | The clip light latches on a value above this. |
| `showClip` | `boolean` | `true` | |
| `onClipChange` | `(clipped: boolean) => void` | | |

The clip light stays on until the meter is clicked or `clearClip()` is called. The root carries `data-clipped` while it is on. The animation frame loop stops when the levels have fallen to rest and when the meter is scrolled out of view. The canvas follows the device pixel ratio and its own size.

`advanceMeter(channel, inputDb, seconds, ballistics)` and `createMeterChannel()` are the ballistics on their own.

## StepButton, StepGrid and StepGridGroup

```tsx
const rack = useRef<StepGridGroupHandle>(null)
// when the playhead moves:
rack.current?.setPlayStep(step)

<StepGridGroup ref={rack}>
  {channels.map((channel) => (
    <StepGrid key={channel.id} steps={channel.steps} color={channel.color}
              aria-label={`${channel.name} steps`}
              onToggle={(step, on) => toggleStep(channel.id, step, on)} />
  ))}
</StepGridGroup>
```

`StepGrid` props:

| Prop | Type | Default | Notes |
|---|---|---|---|
| `steps` | `readonly boolean[]` | required | One entry per step. The length sets the number of steps. |
| `onToggle` | `(step: number, on: boolean) => void` | | Called only for steps whose state changes. |
| `onGestureStart`, `onGestureEnd` | `() => void` | | One paint stroke, or one key press, is one gesture. |
| `color` | CSS color | `--wf-step-on` | The lit color of the row. |
| `groupSize` | `number` | `4` | Steps per beat. Beats alternate between the two unlit shades. |
| `size` | `"sm" \| "md" \| "lg"` | `"md"` | Row heights of 16, 24 and 32 pixels. The row fills its container's width. |
| `disabled` | `boolean` | `false` | |
| `rightClickClears` | `boolean` | `true` | Turn off to let a context menu open. |
| `stepLabel` | `(step: number) => string` | "Step 1"… | The accessible name of a step. |

Imperative API: `StepGridHandle.setPlayStep(step | null)` moves the playhead highlight of one row. `StepGridGroupHandle.setPlayStep(step | null)` moves it for every row in the group. Neither renders.

Interaction:

- A press toggles a step. Dragging on paints the opposite of the first step's state across the row, including steps a fast drag jumps over.
- A right-click or right-drag clears.
- Each row is one tab stop. Left, right, Home and End move along it; inside a `StepGridGroup`, up and down move between rows. Space and Enter toggle.

Every step is a single memoized `<button aria-pressed>` and all pointer handling sits on the row, so a toggle re-renders one button and the playhead is one attribute write. A rack of 50 rows of 64 steps holds 60 frames a second while painting with the playhead running. Keep `steps` referentially stable for rows that did not change.

`StepButton` is one step on its own: `on`, `onToggle(on)`, `alt`, `playing`, `color`, `size`, and the props of a button.

## ToggleLed and MuteSolo

```tsx
<ToggleLed pressed={armed} onPressedChange={setArmed}
           color="var(--wf-meter-high)" aria-label="Arm">R</ToggleLed>
<MuteSolo muted={muted} solo={solo} onMutedChange={setMuted} onSoloChange={setSolo} />
```

| `ToggleLed` prop | Type | Default | Notes |
|---|---|---|---|
| `pressed` | `boolean` | required | |
| `onPressedChange` | `(pressed: boolean) => void` | | |
| `variant` | `"button" \| "dot"` | `"button"` | A lettered button, or a small round light. |
| `size` | `"sm" \| "md" \| "lg"` | `"md"` | |
| `color` | CSS color | `--wf-brand` | The lit color. |

`MuteSolo` takes `muted`, `solo`, `onMutedChange`, `onSoloChange`, `size` and `disabled`.

## PianoKeyboard

```tsx
<PianoKeyboard lowKey={36} highKey={96} activeKeys={sounding}
               onNoteOn={(key, velocity) => backend.auditionNoteOn(channel, key, velocity)}
               onNoteOff={(key) => backend.auditionNoteOff(channel, key)} />
```

| Prop | Type | Default | Notes |
|---|---|---|---|
| `lowKey`, `highKey` | `number` | `48`, `72` | MIDI keys. |
| `orientation` | `"horizontal" \| "vertical"` | `"horizontal"` | Vertical puts the low keys at the bottom, for a piano-roll gutter. |
| `layout` | `"classic" \| "uniform"` | by orientation | "classic" gives white keys equal widths. "uniform" gives every semitone the same space, to line up with piano-roll rows. |
| `onNoteOn` | `(key: number, velocity: number) => void` | | Velocity is 0.2 to 1, louder towards the front edge of the key. |
| `onNoteOff` | `(key: number) => void` | | |
| `activeKeys` | `ReadonlySet<number> \| readonly number[]` | | Keys shown as held. |
| `showLabels` | `boolean` | `true` | Note names on the C keys. |
| `middleCOctave` | `number` | `5` | 5 names key 60 "C5", 4 names it "C4". |
| `keyboardVelocity` | `number` | `0.8` | For notes played with Space or Enter. |
| `color` | CSS color | `--wf-brand` | The highlight color. |
| `disabled` | `boolean` | `false` | |

Imperative API (`PianoKeyboardHandle`): `flash(key, durationMs = 150)` and `setLit(key, lit)` light keys without rendering, for showing what the engine plays. `releaseAll()` sends note-off for everything held.

Every note-on is followed by exactly one note-off: on pointer up, on pointer cancel, when the pointer slides off the keys, when the window loses focus and when the keyboard unmounts. Dragging across keys plays a glissando. Two pointers on one key send one note.

Each key is a `role="button"` named after its note. The keyboard is one tab stop: arrows move by a semitone, Page up and Page down by an octave, and Space or Enter plays the focused key for as long as it is held.

`noteName(key, middleCOctave)`, `isBlackKey(key)`, `layoutKeys(low, high, layout)` and `keyAtPoint(shapes, along, depth)` are exported.

## Waveform

```tsx
<Waveform peaks={info.peaks} start={sampler.start} end={sampler.end}
          onStartChange={setStart} onEndChange={setEnd} />
```

| Prop | Type | Default | Notes |
|---|---|---|---|
| `peaks` | `Float32Array \| readonly number[]` | required | Min and max pairs, one pair per bucket: min, max, min, max. This is `SampleInfo.peaks`. |
| `start`, `end` | `number` | | Region edges, 0 to 1. Each one that is set gets a draggable handle. |
| `onStartChange`, `onEndChange` | `(position: number) => void` | | |
| `onGestureStart`, `onGestureEnd` | `() => void` | | A handle drag is one gesture. |
| `minRegion` | `number` | `0.001` | The handles cannot come closer than this. |
| `formatPosition` | `(position: number) => string` | percent | The handles' `aria-valuetext`, such as a time. |
| `normalize` | `boolean` | `false` | Scale the drawing so the loudest peak fills the height. |
| `color` | CSS color | `--wf-waveform` | |
| `disabled` | `boolean` | `false` | |

Imperative API: `WaveformHandle.setPlayhead(position | null)` moves or hides the playhead without rendering.

The handles are sliders with the shared interaction model: they follow the pointer, Shift is fine, arrows step, Home and End jump, and double-click returns a handle to its end of the file. They have no wheel and no typed entry. The canvas redraws on resize, on a device pixel ratio change and on a theme change. `drawWaveform(context, peaks, width, height, color, amplitude)` is the drawing on its own.

## EnvelopeEditor

```tsx
<EnvelopeEditor {...envelope} onChange={(patch) => setEnvelope({ ...envelope, ...patch })} />
```

| Prop | Type | Default | Notes |
|---|---|---|---|
| `attackMs`, `decayMs`, `releaseMs` | `number` | required | |
| `sustain` | `number` | required | 0 to 1. |
| `onChange` | `(patch: Partial<EnvelopeValues>) => void` | | Only the fields that changed. Always inside a gesture. |
| `onGestureStart`, `onGestureEnd` | `() => void` | | |
| `maxAttackMs`, `maxDecayMs`, `maxReleaseMs` | `number` | `5000`, `5000`, `10000` | |
| `defaults` | `Partial<EnvelopeValues>` | | Restored for a node by double-click and Ctrl/Cmd-click. |
| `color` | CSS color | `--wf-brand` | |
| `disabled` | `boolean` | `false` | |

Three nodes: attack (time), decay and sustain (time and level), release (time). Each is a `role="slider"`. Left and right change the time by 5% of its value (at least 1 ms), Shift makes that 0.5% (at least 0.1 ms), Page up and Page down 25%, and Home and End jump to 0 and the maximum. On the decay node, up and down change the sustain level by 0.01, or 0.001 with Shift.

Times are rounded to 0.1 ms and sustain to 0.001. Values never leave their ranges. The time axis picks a round length that fits the envelope and keeps it while a node is dragged, so the node stays under the pointer. The sustain stage has no length of its own and is drawn dashed.

`constrainEnvelope(values, limits)`, `envelopePatch(current, next)`, `envelopeSpanMs(values)` and `stepEnvelopeTime(value, direction, size)` are exported.

## NumberField

```tsx
<NumberField aria-label="Tempo" value={tempo} onValueChange={setTempo}
             min={10} max={522} step={0.01} splitDrag unit="BPM" />
```

`ValueControlProps` without `scale` and `format`, plus:

| Prop | Type | Default | Notes |
|---|---|---|---|
| `size` | `"sm" \| "md" \| "lg"` | `"md"` | |
| `step` | `number` | `1` | The precision of the value. |
| `decimals` | `number` | from `step` | Digits shown after the decimal point. |
| `coarseStep` | `number` | `1` | What a drag, an arrow key and a wheel notch move by. |
| `splitDrag` | `boolean` | `false` | Dragging the digits before the point moves by whole `coarseStep`s and keeps the decimals. Dragging the digits after it moves by `step`. |
| `pixelsPerStep` | `number` | `4` | Pointer travel for one step of the dragged part. |
| `unit` | `ReactNode` | | Shown after the number. |

Double-click or Enter opens typing. Shift with an arrow key moves by `step`.

## Theme tokens

The kit reads these CSS variables, which the registry items carry for both themes. Canvas components resolve them from their own element and read them again when the class, style or `data-theme` of `<html>` or `<body>` changes, or when the system color scheme does.

| Token | Read by |
|---|---|
| `--wf-brand` | toggle, keyboard highlight, waveform handles, envelope |
| `--wf-control-track`, `--wf-control-fill` | knob, fader |
| `--wf-meter-bg`, `--wf-meter-low`, `--wf-meter-mid`, `--wf-meter-high` | level meter; mute and solo use mid and low; waveform and envelope use the background |
| `--wf-step-off`, `--wf-step-off-alt`, `--wf-step-on` | step button, step grid; toggle uses the off shade |
| `--wf-playhead` | step playhead, waveform playhead |
| `--wf-grid-line`, `--wf-grid-line-strong` | envelope grid, scale marks, outlines |
| `--wf-waveform` | waveform |

They also use shadcn's `--background`, `--foreground`, `--muted-foreground`, `--border`, `--input` and `--ring`.

These optional tokens have built-in fallbacks. Define one to override the fallback:

| Token | Fallback |
|---|---|
| `--wf-knob-cap`, `--wf-knob-cap-hover` | `--foreground` mixed 14% and 22% into `--background` |
| `--wf-fader-cap`, `--wf-fader-cap-hover` | `--foreground` mixed 86% into `--background`, and `--foreground` |
| `--wf-key-white`, `--wf-key-black`, `--wf-key-line` | `oklch(0.97 0 0)`, `oklch(0.2 0 0)`, `oklch(0.55 0 0)` |
| `--wf-key-active` | `--wf-brand` |
| `--wf-mute`, `--wf-solo` | `--wf-meter-mid`, `--wf-meter-low` |

`canvas.ts` has the helpers the canvas components share: `resolveColors(element, colors)`, `subscribeTheme(listener)` and `observeCanvas(canvas, onChange)`.
