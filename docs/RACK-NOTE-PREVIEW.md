# Note preview in rack

Implementation of the bounded T0 row `win-rack-piano-roll-preview`, based on
`f9a1d2ee735531feb805df6743800b1fd692e32b`. Its contract is: “Shows a thumbnail
of a channel's piano roll notes in place of its step buttons.” This is one
UI feature in the active full Windfall plan, not completion of T0 or phases 0–7.
The parent owns independent review, composition and the parity decision.

## Behavior

The row reads its existing authoritative pattern/channel lane. Automatic view
keeps ordinary step entry when notes are single hits on the sixteenth-note grid,
at the default pitch and step duration. Off-grid timing, different pitch or
duration, and simultaneous notes select the actual note thumbnail. Samplers
and instruments use the same rule: their notes are the same document objects.
Ordinary step notes beyond a shortened pattern remain dormant and do not force
Automatic away from Steps.

Each row's **Row view** menu offers **Automatic steps or notes**, **Show steps**,
**Show notes**, and **Open in piano roll**. It stays at the visible right edge
while the pattern scrolls. Explicit Steps retains the original detail dots,
labels, step painting and erasing. Explicit Notes also works for empty and
ordinary step lanes. Choices belong to the document/pattern/channel in memory;
they are excluded from persisted preferences and from the project file.

The thumbnail represents start tick, duration and pitch with filled rectangles.
Chords share their start coordinate and occupy different pitch rows; overlapping
notes retain their coverage. Higher pitches go up. Silent notes remain visible
because they are still notes in the document. Muted/solo-silenced rows retain
the rack's dimming. It shows only the current pattern's time range, clips notes
at the range boundaries, and reports notes outside the thumbnail. Empty lanes
have a readable empty label. It does not fit time to a lane's individual extent:
the origin and width stay aligned with step rows and the ruler.

Click or Enter selects precisely that current channel and runs the existing
`view.pianoRoll` action. The existing piano panel/session supplies the selected
pattern, editor lifetime and grid focus. No note is converted, copied, removed
or dispatched. The preview validates the existing project generation, current
selected-pattern selector and channel existence before navigation. Pointer
cancel, unmount, pattern switch and project replacement prevent an old press
from opening a stale target. View menus are keyed by generation and lane, so
replacement/switch closes them. Arrow Left returns to the channel name; Up/Down
move between thumbnail and step rows. Space remains the existing transport key.
The menu returns focus to the current steps/thumbnail after a view change.

`onProjectReplaced` clears choices after New/Open, even with repeated numeric
IDs. A project-store subscription removes choices for deleted channels/patterns;
undo restoration then uses Automatic. Lane notes themselves update through the
existing structurally shared selectors after edit/undo/redo/load. Adjacent rows
do not subscribe to this lane's notes or view choice.

## Bounds and unchanged seams

`note-preview-geometry.ts` guards finite coordinates, clips time, clamps pitch
and pattern extent, and supplies a finite empty range. Up to 2048 visible notes
produce individual rectangle subpaths in one SVG path. Larger lanes combine
coverage on a fixed 512-column × 16-row grid. Difference arrays keep preparation
O(notes + 8192), with at most 4096 rectangle subpaths. Every visible note,
including the last, contributes; dense quantization is stated in the tooltip
and accessible description. This is a thumbnail, not an editing surface.
No per-note DOM elements are created.

Geometry is memoized by the target lane's note-array identity and pattern
length. Realtime playback changes only the thumbnail cursor's visibility and
CSS position through the existing playhead feed. It never rebuilds geometry or
sets React state per frame. CSS rack pitch, logical scrolling and root zoom
provide the shared horizontal coordinate system. The SVG's button has no border
inset. The ruler and thumbnail cursors use the same ticks-per-step and rack
pitch. No new pixel-to-note input conversion is introduced.

The only existing product seams changed are the row's step-area rendering and
the rack's transient view state/cleanup. Channel button, mute/solo/mix/routing,
sample drop, reorder, step commands, rack grid/ruler, piano editor/session,
scaling, Rust/project model, engine, IPC, generated types and WASM are unchanged.
No new dependencies or proprietary assets are used.

## Requirement evidence

All feature UI tests use `startRack`/the existing app harness, actual checked
Rust WASM note commands and `SimDocument` mirror. Only toast and row-render
counting are wrapped. No backend replacement or synthetic document command
implementation is used. Pure geometry tests supplement these UI tests for
coordinates rejected by the document; they do not replace UI verification.

| Requirement                                                                 | Test/source evidence                                                                                                                                 |
| --------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Real time/duration/pitch, off-grid/chords/overlaps, target lane only        | `note-preview.test.tsx`: independently calculated coordinates from commanded notes; automatic detection cases; adjacent-row isolation                |
| Everyday steps and explicit choice, no conversion/history/dirty changes     | Real Steps/Notes/Automatic menu test; unchanged project/history references; zero dispatch; original rack/step-grid suites                            |
| Exact click/Enter/menu opening and piano-grid focus                         | Three actual mounted piano-panel/session tests, second-pattern target; zero document and transport commands; T3 browser Enter check                  |
| Arrow navigation, repeats, transport Space, correct preview help            | Keyboard test with real names, step rows, thumbnail, keymap and hint store                                                                           |
| Edit/undo/redo and retained identities                                      | Commanded lane replacement, undo IDs restored, redo, real step-note identity assertions                                                              |
| Save/reopen/New and repeated IDs; stale press/menu/pattern/deletion cleanup | Actual backend file save/open/new tests; replacement generation; cancelled/old presses and menus; channel/pattern removal and undo                   |
| Mute/solo/mix/routing/reorder/sample drop                                   | Combined real-control/real-command test on a preview row retaining notes and geometry; unchanged rack suite                                          |
| Empty/silent/pitch extremes/large duration and clipping                     | Checked UI note fixtures; readable empty state; clipped final rectangle and retained outside notes                                                   |
| Negative/invalid/zero extent finite math                                    | `note-preview-geometry.test.ts`: crossing-zero clipping, invalid omission, pitch/extent clamps and finite path coordinates                           |
| Large valid lane bounded rendering                                          | Actual 3001-note UI lane, one path, last note contributes; alternating dense helper fixture ≤4096 rectangles; difference-array source                |
| No adjacent-row or per-frame note rebuild                                   | Row-render counter tests, same path element/string while actual backend playback moves cursors                                                       |
| 75/100/150/200%, logical step input/scroll/ruler/playhead                   | Four real StepGrid input and cursor-coordinate tests; actual T3 measurements at all four scales and horizontal scroll                                |
| Original detail-dot assertions preserved                                    | Only one approved existing `rack.test.tsx` test adds Automatic preview assertions and selects Show steps; every original assertion remains unchanged |
| Native/model/artifact preservation                                          | Owned-file diff; unchanged generated inventory/WASM/metadata; `check-sim` input/hash verification                                                    |

## Validation and browser evidence

Final focused run: **7 files, 187 tests passed**, two workers:

```text
node node_modules/vitest/vitest.mjs run
  src/features/channel-rack/note-preview.test.tsx
  src/features/channel-rack/note-preview-geometry.test.ts
  src/features/channel-rack/rack.test.tsx
  src/features/channel-rack/rack-render.test.tsx
  src/features/channel-rack/steps.test.ts
  src/components/audio/step-grid.test.tsx
  src/features/piano-roll/piano-roll.test.tsx --maxWorkers=2
```

Both app and node TypeScript projects passed `tsc --noEmit`; scoped ESLint,
Prettier check and `git diff --check` passed. The existing dependencies were
reused through this worktree's junction; no package installation or native build
was performed. `pnpm exec` initially refused an implicit dependency-directory
refresh; checks subsequently used the installed node entrypoints directly.
`check-bindings` was not a fresh-generation gate: it requires a generated
directory and this UI task intentionally does not generate Rust bindings.
The unchanged tracked inventory contains 174 files.

`node scripts/check-sim.mjs` reports the artifact current: **1,881,168 bytes**,
SHA-256 `9feb181f0a3d6aed64d9aabd26c61cedcf015b1437889bfa5ca7b309d18dde3a`,
input hash `26ac50ad9e44561b20ca18d811b00fc8f66e98082d9ecbe89580c07791cc250f`.
Bindings, WASM and metadata have no diff against the assigned base.

Actual T3 collaborative preview at the worktree's local Vite port 1549, using
the real shared WASM browser backend: the thumbnail, steps and ruler had exactly
matching left coordinates and widths at 75/100/150/200%. Scrolling 100 logical
pixels moved each by 75/100/150/200 visual pixels. The view menu stayed visible.
SVG content and its button matched exactly after removing the border inset.
During playback, ruler/thumbnail cursor positions differed by less than 0.008
visual pixels at each scale. Browser Enter opened pattern 1/channel 30, focused
the existing Note grid, exposed the same five notes, and left project identity,
history and dirty state unchanged. The inspected 200% rack scrolls vertically
to reach lower rows, as expected at the smaller logical viewport.

Local T3 evidence files, outside the source-only commit:

- Row view menu: `C:/Users/ewhee/.t3/userdata/browser-artifacts/browser-screenshot-127-0-0-1-muz6syrb-0a57e391.png`
- Thumbnail at 100%: `C:/Users/ewhee/.t3/userdata/browser-artifacts/browser-screenshot-127-0-0-1-muz6wkks-04c77f8b.png`
- Existing piano roll opened: `C:/Users/ewhee/.t3/userdata/browser-artifacts/browser-screenshot-127-0-0-1-muz6y5nw-4eabd32c.png`
- Thumbnail at 200%, vertically/horizontally scrolled: `C:/Users/ewhee/.t3/userdata/browser-artifacts/browser-screenshot-127-0-0-1-muz71vwy-2a050ed5.png`

This is browser/WASM and source evidence, not physical Windows native-window,
audio-device or third-party-plugin verification. Parent independent review and
integrated checks remain before parity acceptance. Native desktop smoke/focus
verification remains an external gate. The owned Vite process is stopped before
handoff. No push, PR, release, version, root README/parity or shared checkpoint
change is part of this delivery.
