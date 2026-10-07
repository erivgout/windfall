# Parameter controls

The controls every built-in instrument and effect editor is made of. An editor hands in a settings object and a way to change one setting. Nothing here knows about the project store, so the rack inspector and the mixer's effect editors use the same code.

```tsx
import {
  effectDescriptor,
  ParamControl,
  ParamGroup,
  ParamRow,
  useParamBinding,
} from "@/features/params"
```

## Example

```tsx
const descriptor = effectDescriptor("compressor")

function CompressorEditor({
  track,
  slot,
}: {
  track: TrackId
  slot: EffectSlot
}) {
  const bind = useParamBinding({
    descriptor,
    params: slot.params.type === "compressor" ? slot.params : null,
    setParam: (param, value, gesture) =>
      dispatch(
        { type: "setEffectParam", track, effect: slot.id, param, value },
        gesture
      ),
  })
  return (
    <ParamGroup title="Compressor">
      <ParamRow columns={4}>
        <ParamControl {...bind("thresholdDb")} />
        <ParamControl {...bind("ratio")} />
        <ParamControl {...bind("attackMs")} />
        <ParamControl {...bind("releaseMs")} />
      </ParamRow>
      <ParamControl {...bind("detector")} layout="inline" />
    </ParamGroup>
  )
}
```

That gives each control its name, its value with the unit, a status-bar hint, double-click to the default, typed entry, and one undo step per drag. With no layout of your own, `<GenericParamEditor descriptor={descriptor} params={params} setParam={setParam} />` lays out every setting.

Run `pnpm dev` and open `/?view=params` to see every descriptor laid out that way, in both themes. `features/channel-rack/synth/synth-editor.tsx` is a purpose-built layout to copy from.

## Descriptors

Read from `src/bindings/descriptors.json`, which `scripts/gen-bindings.sh` writes from the Rust tables.

```ts
type ParamDescriptor<Params extends object = object> = {
  readonly name: string
  readonly params: readonly ParamInfo[]   // in command index order
  readonly defaults: Params               // includes `type`, ready for set…Params
}

effectDescriptor<K extends EffectKind>(kind: K): ParamDescriptor<EffectParamsOf<K>>
instrumentDescriptor<K extends InstrumentKind>(kind: K): ParamDescriptor<InstrumentParamsOf<K>>
paramIndex(descriptor: ParamDescriptor, id: string): number   // throws on an unknown id
paramInfo(descriptor: ParamDescriptor, id: string): ParamInfo // throws on an unknown id
EFFECT_KINDS: EffectKind[]
INSTRUMENT_KINDS: InstrumentKind[]
```

The file writes the core's 32-bit floats out at full length (0.699999988079071). A project writes the same floats the short way (0.7), and read as doubles the two are not equal. So the numbers of a descriptor are brought to the project's spelling when the file is loaded: `info.default`, `info.min`, `info.max` and everything in `defaults` equal what a project stores, and `defaults` loaded with `set…Params` reads back unchanged. `shortFloat(value: number): number` is that conversion. Write your own preset values with few digits (0.85, not `Math.fround(0.85)`) and they read back unchanged too.

An id is the dotted path into the settings object: `"thresholdDb"`, `"lowShelf.gainDb"`, `"oscillators.0.level"`. `paramIndex` is the number `setEffectParam` and `setInstrumentParam` take as `param`.

## Reading and writing a settings object

```ts
readParam(params: object, info: ParamInfo): number
writeParam<Params extends object>(params: Params, info: ParamInfo, value: number): Params
clampParam(info: ParamInfo, value: number): number
```

Values are always the numbers the commands take: a toggle is 0 or 1, a choice is the index of the option, an integer is whole. `readParam` throws when the object has nothing at the id (it belongs to another kind). `writeParam` returns a new object, clamps the value like the core does, and shares everything it did not touch with the original.

## Text

```ts
formatParam(info: ParamInfo, value: number): string
parseParam(info: ParamInfo, text: string): number | null   // inside the range, or null
```

| Unit           | Reads as                                                         | Also accepted when typed |
| -------------- | ---------------------------------------------------------------- | ------------------------ |
| `decibels`     | "−18.0 dB", "+3.0 dB"; no plus sign for a range that starts at 0 | "-18"                    |
| `gain`         | linear gain as dB: "−12.0 dB", "−∞ dB" at 0                      | "-inf"                   |
| `hertz`        | "0.50 Hz", "20.0 Hz", "440 Hz", "1.20 kHz", "20.0 kHz"           | "1.2k"                   |
| `milliseconds` | "0.05 ms", "2.5 ms", "250 ms", "1.25 s"                          | "1.5 s"                  |
| `seconds`      | "100 ms", "1.80 s"                                               | "1.8", "250 ms"          |
| `fraction`     | "70%"; "+20%" for a range that goes below 0                      | "70"                     |
| `semitones`    | "+7 st" for an integer, "+0.25 st" otherwise                     | "7"                      |
| `cents`        | "+5.0 ct"                                                        | "5"                      |
| `octaves`      | "+2.00 oct"                                                      | "2"                      |
| `ratio`        | "4:1", "2.5:1", "20:1", and "∞:1" at the top of the range        | "4", "inf"               |
| `pan`          | "L30", "C", "R100"                                               | "30L", "-30"             |
| `none`         | "0.71", "16"                                                     |                          |

A toggle reads "On" or "Off" and a choice reads its option's label; both parse back.

## ParamControl

```ts
type ParamControlProps = {
  info: ParamInfo
  value: number
  onValueChange?: (value: number) => void
  onGestureStart?: () => void
  onGestureEnd?: () => void
  size?: "sm" | "md" | "lg" // default "md"
  label?: ReactNode // replaces the name on screen; null hides it
  disabled?: boolean
  description?: string // one sentence for the status bar
  color?: string // knob arc or toggle light
  layout?: "stacked" | "inline" // toggles and choices; default "stacked"
  choiceStyle?: "auto" | "segmented" | "select"
  choiceIcon?: (choice: ParamChoice, index: number) => ReactNode
  scale?: ValueScaleOption // replaces the knob travel
  contextItems?: readonly ContextItem[] // entries about the setting, for its menu
  live?: LiveValueFeed // a value something else is giving the setting right now
  marker?: string // CSS color of the dot that says the setting has an automation
  className?: string
}
```

| `info.kind`        | Control                                                                                                  |
| ------------------ | -------------------------------------------------------------------------------------------------------- |
| `float`, `integer` | The kit `Knob` with the value under it. An integer steps by 1.                                           |
| `toggle`           | The kit `ToggleLed` as a light over its label, or a switch at the end of the row with `layout="inline"`. |
| `choice`           | A strip of buttons for up to 4 options (`SEGMENTED_MAX_CHOICES`), a `select` for more.                   |

Every control is named after `info.name` for assistive technology, whatever `label` shows, and carries `data-param={info.id}`. The status bar reads "Name: value" followed by `description`, or by how to use the control. Double-click a knob, or the label of a toggle or choice, to return to `info.default`; Ctrl/Cmd-click does the same. A click on a toggle or a choice is one gesture.

A right-click on a knob opens the menu every value control has: reset to default, type in a value, copy and paste (`components/value-context-menu.tsx`). A copied value carries the kind of unit it is in, which for a setting is its descriptor's `unit` (`paramUnitKind(info)`), and pastes only into a control of the same kind: a threshold into a make-up gain, not into a cutoff. `contextItems` are put before those. They are for entries about the setting itself and come with `bind(id)`. A toggle and a choice are not value controls, so they have no such menu; with `contextItems` a right-click on one opens a menu of just those entries, which is how a switch or a list of shapes gets "Create automation clip".

`live` and `marker` are the kit's (`components/audio/README.md`): a knob shows the live value with a second pointer and arc, drawn outside React, and every kind of control shows the marker as a small dot. A toggle and a choice do not show a live value.

How a knob is set up, in `format.ts`:

- `paramScale(info)`: logarithmic settings get the kit's `"log"` scale. A `gain` from 0 to 2 gets the fader taper. A time that starts at 0 and reaches a second or more is "linear" in the descriptor because a logarithm cannot start at 0; it gets a cubic curve so its first milliseconds have room. Everything else is linear.
- `paramIsBipolar(info)`: a range that is the same size either side of 0, and any `pan`, draws from the middle out and sticks to 0.

## useParamBinding

```ts
type SetParam = (index: number, value: number, gesture?: number) => unknown

useParamBinding<Params extends object>(options: {
  descriptor: ParamDescriptor<Params>
  params: Params | null | undefined
  setParam: SetParam
  contextItems?: (info: ParamInfo, index: number) => readonly ContextItem[]
  live?: (info: ParamInfo, index: number) => LiveValueFeed | undefined
  marker?: (info: ParamInfo, index: number) => string | undefined
}): ParamBinder

type ParamBinder = {
  (id: string): ParamBinding           // spread onto a ParamControl
  info(id: string): ParamInfo
  value(id: string): number            // the dragged value, or the stored one
  group(key: string): ParamGroupBinding
  disabled: boolean                    // true without params
}
type ParamBinding = {
  info: ParamInfo
  value: number
  disabled: boolean
  onValueChange(value: number): void
  onGestureStart(): void
  onGestureEnd(): void
  contextItems: readonly ContextItem[]  // what `contextItems` gave for this setting
  live?: LiveValueFeed                  // what `live` gave for it
  marker?: string                       // what `marker` gave for it
}
type ParamGroupBinding = {
  set(id: string, value: number): void
  onGestureStart(): void
  onGestureEnd(): void
}
```

- One gesture id per drag: every `setParam` call between a control's gesture start and end carries the same `gesture`, so the drag is one undo step. Pass it on as the second argument of `dispatch`.
- While a control is moved it shows the value it sent, not the stored one. It lets go when the gesture is over and the last `setParam` has settled, so return the promise of your dispatch from `setParam`.
- Without `params` (`null` or `undefined`) every control is disabled and shows its default.
- An unknown id throws, on the first render.
- The handlers of an id are the same objects on every render, and `ParamControl` is memoized, so a drag renders only the control that moves.
- `contextItems(info, index)` gives the entries a setting's control shows in its right-click menu, before the ones every value control has. `index` is the number the `set…Param` commands know the setting by. It is the one place an entry about a bound setting is added for every control of an editor: an entry that makes an automation clip for the setting goes here. Return the same array for the same setting.
- `live(info, index)` and `marker(info, index)` hand each control the value automation is giving its setting and the color of its automation. `useParamAutomation` in `features/automation/live.ts` returns all three options for an effect or an instrument: `useParamBinding({ descriptor, params, setParam, ...automation })`.
- `group(key)` is for a control that moves several settings at once: everything set between its gesture start and end is one undo step, and the knobs of those settings follow along. `ParamEnvelope` uses it.

## ParamEnvelope

```tsx
<ParamEnvelope
  bind={bind}
  prefix="ampEnvelope"
  aria-label="Amp envelope"
  className="h-24"
/>
```

The kit `EnvelopeEditor` on `<prefix>.attackMs`, `.decayMs`, `.sustain` and `.releaseMs` of a binding. Ranges and defaults come from the descriptor. It takes the editor's other props (`color`, `className`, `aria-label`).

## ParamGroup and ParamRow

```tsx
<ParamGroup title="Filter" aside={<Switch />}>
  <ParamRow columns={4}>…knobs…</ParamRow>
  <ParamRow>…wraps when full…</ParamRow>
</ParamGroup>
```

`ParamGroup` is a titled box (`title: string`, `aside?: ReactNode`, plus the props of a `section`). `ParamRow` lays its children out in `columns` equal columns, or side by side with wrapping when `columns` is left out.

## GenericParamEditor

```ts
GenericParamEditor<Params extends object>(props: {
  descriptor: ParamDescriptor<Params>
  params: Params | null | undefined
  setParam: SetParam
  size?: "sm" | "md" | "lg"
  className?: string
})
```

Lays out every setting of a descriptor. `paramGroups(descriptor)` decides the groups: settings that share the path before their last segment form one (`filter.*` is "Filter", `oscillators.0.*` is "Oscillator 1", `lfos.1.*` is "LFO 2", `lowShelf.*` is "Low shelf"). Top-level settings form a group per run of them in the table, named after the word their ids share ("Unison") or "General". A descriptor with only top-level settings gets no heading. Inside a group a label drops what the heading already says: "Osc 1 level" becomes "Level", and a toggle that is named like its group ("Low cut") becomes "Enabled". `humanizePath(path)` and `shortLabels(infos, title)` are the two halves of that.
