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

The existing product seams changed are the row's step-area rendering,
the rack's transient view state/cleanup and its scoped action registration.
Channel button, mute/solo/mix/routing,
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

Initial `ea2f36dd` checkpoint run: **7 files, 187 tests passed**, two workers:

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

## R1 standards response (incremental checkpoint after immutable ea2f36dd)

R1 spec review found no actionable defect and independently ran 80 actual-WASM
tests. That result belongs to the reviewer, not this follow-up's execution.
Both standards P2 findings are addressed in the next source-only checkpoint;
the parent owns a new full-context R2 review on both axes and parity acceptance.

**Registry:** `ARCHITECTURE.md` requires “Every user action is a named entry in
the action registry” and “Every label (menus, palette, tooltips, context menus)
shows the key an action has in its own scope.” The three row-view choices now
have canonical `channelRack` actions, with titles, checked/enabled state,
disabled reasons and default shortcuts Ctrl+Alt+1/2/3 (Mod uses Cmd on macOS).
The guarded `channelRack.openPianoRoll` entry delegates to `view.pianoRoll`;
`standsFor` reveals that existing action's configured key (Alt+3, or F7 in the
FL preset). There is no duplicate shortcut map or menu-title table.
`ActionMenuItem` renders the row dropdown. Choice/replacement invalidation
joins the registry's existing project/transport/selection subscriptions.

**Thumbnail context menu:** the same standard requires menus “on every thing
with actions of its own” and says “the innermost one opens.” `ContextActions`
now wraps the actual thumbnail. Its target-bound inline entries derive all
titles, shortcut hints, checked state and disabled reasons from registered
actions, then execute those registry ids. This uses the existing inline-entry
contract for the thing clicked; it does not add action metadata. Right-click
retains ordinary rack pointer/focus row selection, opens only this inner menu,
and performs no primary navigation, transport or history write. Context
commands execute after close, so opening the existing piano session keeps its
grid focus.

**Target/focus lifetime:** `note-preview-target.ts` binds only selection and
lifetime to canonical metadata. Every captured command validates generation,
current pattern and channel before selection and execution, including deferred
context commands. Disabled reasons distinguish a changed project/pattern and
deleted targets. Dropdown capture validates before `ActionMenuItem` runs its
canonical id. Palette/keymap commands use the current selected lane. One frame
after a view command restores focus into the replacement step/thumbnail control;
it rechecks the captured document/lane and current channel selection. This
prevents queued focus from selecting a successor document's reused id or
overriding a later user selection. It is unrelated to realtime playback.

**Optional Primitive Obsession concern:** addressed within the owned scope.
Callers pass typed `NotePreviewLane` identity. Key encoding is private to
`rack-store.ts`, and each stored choice retains typed lane identity; deletion
cleanup no longer decodes strings. All of it remains transient and excluded
from persisted preferences/projects.

Only the previously added Show steps lookup in the approved legacy test changes
to `menuitemcheckbox` with an anchored title match, reflecting the shared
component's checked role and shortcut-inclusive accessible name. Every original
dot, accessible-label and lit-step assertion and every other old-test line is
unchanged. New tests check actual registry/palette discovery, configured keys in
both presets, scoped key execution, checked-state invalidation, click/Enter
context execution, exact mounted piano focus, stale menus and deferred focus
after pattern switch/New/Open/deletion/selection changes. All document operations
continue to use the assigned actual Rust WASM and mirror.

Fresh T3 inspection on the follow-up source at port 1549 confirmed one inner
context menu with the canonical keys and current-view tick. Thumbnail SVG and
ordinary step rows retained exactly matching left/width coordinates. Fresh-load
Ctrl+Alt+2/3 restored focus to step 1/the thumbnail; context-menu Enter opened
pattern 1/channel 30 and focused Note grid, with project/history identity and
dirty state unchanged. The initial hot reload retained old registered function
closures; this focus check was repeated after a full reload with the current
registered run functions. A saved, inspected screenshot of the final menu is
`C:/Users/ewhee/.t3/userdata/browser-artifacts/browser-screenshot-127-0-0-1-muz8fscm-20f768e4.png`.

Fresh follow-up execution: the same seven focused files passed **201/201 tests**
with `--maxWorkers=2 --no-cache --configLoader=runner`, including 14 additional
registry/context/focus cases. An intermediate focus-timing test recursively
replayed background realtime frames and exhausted its worker; its timing control
now holds only the focus callback and leaves realtime frames running normally.
One intermediate run reached the legacy synchronous menu lookup before menu
mount; its approved lookup was retained and passed in the final full run. There
are no retries, skips or weakened original assertions in the source. The exact
incremental SHA and final static/artifact checks are reported in the handoff.
The earlier four-scale browser/playhead measurements above
remain initial checkpoint evidence; they are not represented as a new physical
Windows/native-window run. Geometry and all native artifacts are unchanged.
