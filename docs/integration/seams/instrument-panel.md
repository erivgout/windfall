# Built-in instrument settings panels

Windfall's channel inspector selects the settings panel from the instrument's
stored `params.type`. `subtractiveSynth` keeps its existing `SynthEditor` and
`SoundMenu`. Every other built-in instrument uses `GenericInstrumentEditor`
beside the instrument section, with controls from
`instrumentDescriptor(params.type)`. Hosted plugin bindings take precedence and
continue to show `PluginControls`.

The generic panel uses `GenericParamEditor` to read each descriptor's parameter
paths and control types. It does not bind subtractive synth oscillator ids.
Edits dispatch `{ type: "setInstrumentParam", channel, param, value }`, where
`param` is the index in that instrument's descriptor. The shared parameter
binding passes the same gesture id for every change of a drag to `dispatch`,
making the drag one undo step, as in the effect fallback editor.

Automation menus, live values, and markers use the existing
`{ type: "instrumentParam", channel, param }` target through
`useParamAutomation`. The shared automation layer retains its existing checks
for which parameters can be automated.

Validation from `apps/desktop`:

```sh
pnpm test src/features/channel-rack/inspector/instrument-section.test.tsx
```

The focused tests cover the subtractive oscillator control and sound menu,
acidLine's descriptor defaults and controls, dispatch by descriptor index,
drag grouping with undo, instrument automation, and hosted plugin precedence.
The focused suite isolates the sound menu's action-id helper from rack action
registration while rendering the real synth, generic, and plugin controls.
An attempted combined run with `src/features/channel-rack/synth.test.tsx` was
blocked before tests executed by duplicate `NotePreviewLane` imports in
`src/features/channel-rack/rack-store.ts`, outside this change's scope.
Targeted ESLint and Prettier checks pass. `pnpm typecheck` remains blocked by
errors outside the changed panel files, including the rack duplicate import,
analysis backend declarations, and instrument-union assumptions in other views.

Awaiting integrated QA for the full built-in instrument catalog. This panel
exposes descriptor controls; specialized instrument layouts remain future work.
No parity completion is claimed.
