# Interface scaling

This implements the shared application portion of `wf-ui-scaling` (phase 1,
roadmap R1). The full row also requires scaling native third-party plugin
window content. **That part is pending with the plugin-runtime owner; this
change does not complete the parity row.** The current editor API only opens
or closes an editor (`pluginEditor(target, open)`); it has no scale argument.
No plugin-runtime, Rust, IPC, project-format, generated binding or WASM changes
are part of this implementation.

## Preference and scope

Settings → Appearance → Interface scale offers 75%, 100%, 125%, 150%, 175% and
200%, with Restore 100%. The default is 100%. The preference lives in the
existing version-1 `windfall.ui` local-storage record, outside the musical
document. Missing, nonnumeric, non-enumerated, out-of-range and malformed JSON
preferences recover to 100% on startup. Old version-1 preferences remain
compatible. Same-origin storage events rehydrate preferences in already-open
panel views without echoing a storage write.

The main entry point applies CSS `zoom` to the document root. Layout reflows in
a smaller or larger logical viewport, including panel sizes, text, SVG icons,
audio controls, canvas headers, grid, menus, tooltips, dialogs, command palette,
built-in instrument/effect editors and generic hosted-plugin parameter forms.
The same entry point covers detached panel views and the control showcases.
This is application scaling in both browser and native webview rendering; it
does not call a native webview zoom API. A webview without CSS zoom support
uses 100%, disables the scale selector and explains that limitation.

Changing this preference invokes no project command, engine configuration or
backend operation. It creates no dirty state, document revision or undo entry.
Ordinary parameter/note edits continue to use their existing document commands.

## Coordinate policy

`src/lib/ui-scale.ts` is the shared contract. CSS zoom's layout/coordinate
behavior is specified in the [CSS Viewport specification](https://drafts.csswg.org/css-viewport/#zoom-property).

| Space | Meaning and conversion |
| --- | --- |
| Logical CSS pixels | Layout sizes, `clientWidth`/`clientHeight`, canvas viewport sizes, pixels per tick/row, scroll positions and control drag travel before application zoom. |
| Visual client pixels | Pointer `clientX`/`clientY`, `getBoundingClientRect()` positions/sizes and pixel-mode wheel distances. Divide relative positions and distances by `uiScaleFactor()` exactly once. |
| Backing/device pixels | Logical size multiplied by effective density: browser/native `devicePixelRatio` × application scale, subject to allocation bounds. Browser zoom/monitor DPI are already reflected in DPR and client pixels. |
| Ratio inputs | A pointer offset divided by a visual rectangle dimension already cancels application scale. Keyboard steps and line/page wheel units stay logical. |

Use `localPoint(element, event)` for border-aware local input,
`logicalDelta(distance)` for raw client deltas, and `logicalWheel(event)` for
wheel navigation. `TimeGridView.localPoint()` now returns logical pixels.
Callers keep their established line/page step sizes (playlist 16/400;
piano roll and mixer 1/1), preserving the 100% input behavior.
`createPointerFrame()` freezes both the origin and application scale at press,
preserving the established gesture contract when layout moves under a held
pointer. The musical tick/key/track/hit-test/snap algorithms remain intact.
Normalize strip/ruler/value-lane positions and scrollbar positions/lengths at
their DOM boundary as well; mixing an unscaled thumb with a scaled pointer
would change scroll extents.

`canvasResolution()` is shared by time grids, their header/value layers and
audio display canvases. It sanitizes logical dimensions, caps each edge at
8192 device pixels and each backing store at 16,777,216 pixels, including
rounding and exact observer boxes. Extremely large canvases lower density
instead of allocating an unbounded surface. All layers use the same policy
and pass their actual density to their painters/renderers. ResizeObserver
device boxes are accepted only when consistent with the effective density;
otherwise logical size × density wins, using the existing fractional-DPR
tolerance. Scale events, window resize and DPR media-query changes update
stores even without an observer delivery. Canvas2D, WebGL2 and optional
WebGPU still receive the existing device-transform contract and retain their
existing selection, restoration and fallback policy.

Floating UI/Base UI positioners report visual coordinates. Their portal gets
a fixed viewport frame with inverse root zoom; only popup content is zoomed
back to application scale. Anchor widths and available bounds are converted
to logical content units, retaining caller-specified popup widths. This avoids
double-scaling placement and keeps submenus/tooltips/selects on screen. Fixed
dialogs use logical viewport limits and scroll; the command list has its own
bounded scrolling area. Dense app headers/toolbars can scroll horizontally.
At 100%, the portal wrapper is `display: contents` and these non-default
layout overrides do not apply.

For future native editor integration, consume the application factor and
scale-change notification separately from the native window's monitor DPI.
Do not multiply both into an API that already handles monitor DPI. That owner
must implement supported-plugin scale negotiation, content resizing, failure
reporting, reopen/persistence and real native editor verification.

## Checks and evidence

Checks ran in `gpt/t3-ui-scaling`, based on
`42522322e96c04cffc831370cef762d2bf1a41d0`, after fast-forwarding the parent
artifact/document commit `186652c6f91463a629feadd9bf2641f54f958956`.
No Cargo build or local artifact regeneration was needed.

New focused tests:

- `ui-scale.test.tsx`: actual Settings selector/Restore control, persistence
  rehydration, invalid/malformed recovery, unsupported fallback, storage-event
  application, unchanged document identity and zero dispatch/configure calls.
  Actual knob/fader DOM drags reach both endpoints at 75/125/150/200%, with
  keyboard Home/End. Generic `PluginControls` number-field events edit a retained
  fixture binding through the real project command path at those four scales.
- `canvas/scaling.test.ts`: real common canvas/view/input across 75/125/150/200%
  × DPR 1/2/1.25, note hits, a frozen pointer frame, ResizeObserver logical
  resize and inconsistent device boxes, scale change without observer delivery,
  bounded square/nonsquare allocations and the audio-canvas density contract.
- `canvas/scaling-input.test.ts`: real piano grid and playlist pointer handlers,
  WASM document commands and sessions at the four scales. Note draw/hit/move,
  box/lasso selection and wheel navigation; playlist clip placement/movement;
  automation point insertion/hit/movement. Drawing contexts are test doubles;
  editor/input handlers and document commands are real.

The first scoped regression command passed **16 files / 224 tests**, covering
the three scaling files, time-grid view, pointer frame, renderer parity,
WebGL2 restoration/fallback, renderer contract, drag-value, fader, envelope,
waveform, Settings, stamp lifecycle, playlist pointer and automation UI tests.
A follow-up passed **5 files / 62 tests** for preference/density updates,
menu close behavior, hosted plugin controls and descriptor parameter controls.
The final common scaling run passed **3 files / 44 tests**, including the four
additional generic hosted editor scale cases and preservation of piano-roll
line/page wheel steps. All runs use `--maxWorkers=2`; no full suite was run.
Typecheck, ESLint, Vite production build, changed-source Prettier checks and
`git diff --check` passed. Vite reports its existing large-chunk advisory.

Commands from `apps/desktop` (the whitespace check is from the worktree root):

```powershell
pnpm test src/lib/ui-scale.test.tsx src/lib/canvas/scaling.test.ts src/lib/canvas/scaling-input.test.ts src/lib/canvas/time-grid-view.test.ts src/lib/canvas/pointer-frame.test.ts src/lib/canvas/renderer-parity.test.ts src/lib/canvas/renderer-webgl2.test.ts src/lib/canvas/renderer.test.ts src/components/audio/use-drag-value.test.tsx src/components/audio/fader.test.tsx src/components/audio/envelope-editor.test.tsx src/components/audio/waveform.test.tsx src/features/settings/settings.test.tsx src/features/piano-roll/stamp-lifecycle.test.tsx src/features/playlist/pointer.test.ts src/features/playlist/automation/automation-ui.test.tsx --maxWorkers=2
pnpm test src/lib/ui-scale.test.tsx src/lib/canvas/scaling.test.ts src/features/layout/menus-close.test.tsx src/features/plugins/plugins.test.tsx src/features/params/param-control.test.tsx --maxWorkers=2
pnpm test src/lib/ui-scale.test.tsx src/lib/canvas/scaling.test.ts src/lib/canvas/scaling-input.test.ts --maxWorkers=2
pnpm typecheck
pnpm lint
pnpm build
git diff --check
```

T3 preview used an isolated tab at `http://127.0.0.1:5211`, with status checked
before opening. Browser interactions used actual clicks, keys and scrolling;
store/session reads supplied verification only. No React state injection was
counted as user behavior. Runs occurred on 2026-10-08 UTC in
HeadlessChrome 154.0.8037.92, reporting Windows NT 10.0/Win64 and DPR 2.
This is a server preview, not a physical native-window test:

| Scenario | Observed result |
| --- | --- |
| Settings at 1280×800 | Clicked 75/125/150/200% options successfully; selected preference persisted. Reload retained 200%. |
| Settings at 800×600, 200% | Stable dialog bounds `(32,32,736,536)` visual pixels; scroll height 1192 versus logical client height 268. Scrolled to Appearance/Keyboard, clicked Restore 100%, selected 200% again, and reached Close by ordinary focus/scroll behavior. |
| Command palette | At 200%, 1920×1080 bounds `(448,112,1024,856)`; at 200%, 800×600 bounds `(32,36,736,528)` with a scrolling list. At 75%, 800×600 bounds `(208,139.5,384,321)`. All stable bounds fit the viewport; Escape closes. |
| Menus | Channel selector options selected through their real popup; Options menu at 75% fits `(273.5,25,168,96.75)`. Note context menu at 200% fits the desktop viewport and preserves the note actions. |
| Piano roll at 200% | Snare note placed at tick 2160/key 60, then selected and moved by ArrowRight/ArrowUp to tick 2400/key 61. History contains Add note and the two Move note commands. Grid backing 2656×335 for about 1328×167 visual pixels at DPR 2. |
| Piano roll at 75% | Kick note placed at tick 2400/key 60, then selected and moved to tick 2640/key 61. The initial probe landed on an existing note's end, correctly identifying the resize hit; the placement check used empty space. |
| Audio/built-in controls | Master volume Home/End reaches 0/2 at 200%. Sampler inspector scrolls to all sections; Tune Home/End reaches −48/+48, with a 72×72 visual knob at 200%. At 75%, the descriptor showcase's Balance Gain reaches 0/4 through Home/End and records the normal parameter gestures. |

Early popup checks exposed double-scaled coordinates and a portal stacking
problem; both were corrected and the actual Settings option clicks were
repeated successfully. Incorrect preview locators and before-unload prompts
from isolated test edits were corrected/resolved before continuing. Browser
drag trajectories are covered by the DOM input tests; the preview tool's
locator-only drag API was not used to claim physical pointer-drag evidence.

## Integration boundaries and pending verification

The consuming changes are deliberately narrow. Protected
`piano-roll/grid-input.ts` only imports the common conversions and normalizes
wheel/pan distances; its stamp/editor gesture behavior is not rewritten.
`editor.ts`, stamp logic and note-tool/dialog/menu/action implementations are
untouched. The other piano strips, ruler, value lane, scrollbar and canvas
layer only adopt the logical coordinate/density boundary. Playlist rulers,
scrollbars/track-header positions, EQ display, tempo drag, mixer wheel and
toast offset make the corresponding DOM-boundary conversion.

`channel-rack/rack-grid.tsx` has one overlap with the rack worker: reorder-gap
Y is normalized before division by the existing row height. Preserve that
conversion when integrating channel-operation work. Parent owns roadmap,
parity, root integration and generated artifacts; none are updated here.

Pending external checks include native Windows/WebView2 monitor transitions,
physical pointer/touch/trackpad behavior, macOS WKWebView, Linux WebKitGTK,
multiple physical detached windows and native third-party editor content
scaling. Unsupported CSS zoom is exercised as a test fallback, not on an old
physical webview. Real WebGPU hardware/device loss and a new 10k/50k performance
benchmark at each scale were not run. Existing renderer policy and mock parity
checks passing do not establish those external results. The entire
`wf-ui-scaling` parity row must remain pending until native editor integration
and its required external evidence are accepted.
