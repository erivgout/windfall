# Piano stamp cancellation and pointer-presence repairs

The first repair addressed two findings against `46de77e38dc40caf3a1493abe3454d24a4385f18`, in `a58e9e83cfc5065d76e2b48f2077f03aca662790` on integrated base `73149138c6158b0dc5e73604cdf19abaed801006`. The second repair below addresses the pending right-click and rejected-capture findings against `a58e9e83`, on integrated base `186652c6f91463a629feadd9bf2641f54f958956` in `gpt/t3-piano-scales`.

## Pending menu choices

Previously, choosing a stamp saved a local menu ref until Base UI finished closing. Blur or a tool change could cancel the editor during that interval, but the delayed callback subsequently armed the saved choice.

`Editor.deferStamp` now gives each pending choice cancellable ownership of its completion. Pending choices count as busy, so existing cancellation actions can reach them. Cancellation, lane/note/length/signature changes, project replacement and disposal invalidate that ownership; returning to the original lane or tool does not restore it. The menu also cancels its pending choice on window blur, Escape, direct tool changes and unmount. Escape reaches this pending choice even while the closing menu still owns focus. Completion checks the current document, lane and tool before arming. An invalidated callback neither arms a stamp nor focuses the grid.

Ordinary menu selection still waits for the exit transition, then arms once and focuses the grid. Cancellation creates no notes or undo entries. The editor's existing `follow`, command dispatch and created-selection recovery methods are unchanged, as are the shared project store and flow authorities.

## Preview outside the grid

Previously, pointer leave removed a stamp preview, but a global modifier event reused the retained pointer coordinates because an armed stamp counted as busy. Provisional-note counts were **3 → 0 → 3**.

Grid input now tracks whether the pointer is inside the grid, including the coordinates of captured pointer moves. Idle modifier and wheel refreshes require pointer presence. The existing held pointer frame continues to allow modifier updates during captured drags outside the grid; the last coordinates are retained. Re-entry immediately refreshes the preview. Releasing an outside gesture also clears idle hover/preview state.

## Regression evidence

Before the repair, real-menu tests reproduced re-arming after blur, tool actions/setters, Escape, a lane round trip, a lane edit followed by undo, and disposal followed by reattachment. Actual grid-input tests reproduced all three modifier failures for Alt, Shift and Ctrl. The menu tests hold Base UI's actual `Animation.finished` wait with a controlled promise, then release it after cancellation; they do not invoke `onOpenChangeComplete` directly or depend on a CSS animation duration.

The first repair added 21 tests in `stamp-lifecycle.test.tsx`. They also verify ordinary arming/focus, menu unmount, pattern-length/signature changes followed by undo, project generation cancellation, modifier down/up while outside, re-entry, captured Alt/Shift draw behavior and Ctrl copy drags outside. Real Rust WASM transactions verify created selection after held revision-gap recovery and rejection of late selection updates after lane switching, project replacement and disposal. These cases remain in the current 34-test lifecycle file.

Validation of the first repair in this worktree:

- `pnpm exec vitest run src/features/piano-roll/piano-roll.test.tsx src/features/piano-roll/stamp-lifecycle.test.tsx src/features/piano-roll/scale-stamp.test.tsx src/features/piano-roll/scales.test.ts --maxWorkers=2`: **96 tests in 4 files passed**. Two workers were allowed concurrently; no full repository suite was run.
- `pnpm exec eslint src/features/piano-roll/editor.ts src/features/piano-roll/grid-input.ts src/features/piano-roll/stamp-menu.tsx src/features/piano-roll/stamp-lifecycle.test.tsx`: passed.
- `pnpm typecheck`, `pnpm build` and `git diff --check`: passed. The production build retains the existing large-chunk advisory.
- T3 preview used its own isolated tab, `tab_h_6a86cc21-32fb-473a-bf54-e9b0999bfdbb`, at `http://localhost:5198/`. During a held real menu exit, a dispatched window blur and the registered Paint tool action each left the stamp null after completion. Ordinary selection armed Major triad and focused Note grid. Real browser pointer hover/leave and Alt/Shift/Ctrl down/up yielded **3 → 0 → 0**, with all six modifier events leaving the preview hidden; real pointer re-entry restored **3** notes. Escape then cleared the stamp and preview. The temporary animation instrumentation was removed afterward.

The first repair used the parent's committed 163 binding files (161 TypeScript and two JSON) and 1,775,678-byte simulator WASM. No Cargo process or artifact regeneration was needed. Obsolete artifacts from this worktree's earlier verification were backed up under ignored `target/piano-repair-artifact-backup/` before fast-forwarding. Browser observations are saved under ignored `target/piano-repair-qa/`.

## Second repair: right-click ownership and rejected presses

The second review found two remaining input paths. A right-click during menu exit reached ordinary erase/context-menu handling because cancellation checked only an armed stamp. A refused root-125 Major triad press retained the pointer frame/capture because `busy` included the idle armed stamp. Subsequent outside movement and refreshes could display a valid chord at the retained outside coordinates.

Before changing runtime source, nine new regressions failed on integrated `186652c6`; the prior 21 lifecycle tests passed. Four delayed-menu cases cover empty/occupied right-clicks in Draw and Select. The occupied Draw case deleted the saved note, changed dirty from false to true, advanced history **2 → 3**, and increased revision. The other pending cases armed the stamp after cancellation should have consumed the choice. Five rejected-press cases reproduced capture retained in the grid-input fixture and **3** provisional notes after outside movement, with separate Alt/Shift/Ctrl/wheel continuations.

`Editor.hasStampChoice` includes pending and armed choices. Right-click consumes either choice before hit testing or ordinary note actions. Grid input records that ownership before cancellation and consumes the associated context-menu event, so it cannot open a note menu or erase a note. A cancelled active stamp gesture also releases its existing capture. Ordinary Draw right-delete and Select context-menu behavior retain their existing meanings when no stamp choice owns the event.

`Editor.hasPointerGesture` distinguishes an actual gesture from `busy`. Grid input grants capture and auto-scroll only to a started gesture. Refused presses release the held frame and any capture immediately. Outside move, modifier and wheel refreshes require an active held gesture; an idle armed stamp remains available for correction and cancellation. Re-entry resumes its preview, while mouse-up from the refused press cannot place it. A later valid click places the whole pattern normally. Accepted draw/copy/stamp drags retain capture and modifier behavior outside the grid.

Validation of the second repair:

- RED: `pnpm exec vitest run src/features/piano-roll/stamp-lifecycle.test.tsx --maxWorkers=2` before the runtime fix: **9 failed, 21 passed**. All nine new failures matched the reported cancellation/capture defects.
- GREEN: `pnpm exec vitest run src/features/piano-roll/piano-roll.test.tsx src/features/piano-roll/stamp-lifecycle.test.tsx src/features/piano-roll/scale-stamp.test.tsx src/features/piano-roll/scales.test.ts --maxWorkers=2`: **109 tests in 4 files passed**. The lifecycle file now has **34** tests, including unchanged real-WASM revision-gap and late-reply cases, right-click notes/dirty/full-history/selection invariants, root-125 rejection, outside modifiers/wheel, re-entry, valid capture, and ordinary right-click controls. Concurrency was limited to two workers; no full repository suite was run.
- `pnpm exec eslint src/features/piano-roll/editor.ts src/features/piano-roll/grid-input.ts src/features/piano-roll/stamp-menu.tsx src/features/piano-roll/stamp-lifecycle.test.tsx`, TypeScript checking through `pnpm build`, production bundling, and `git diff --check`: passed. The build retains the existing large-chunk advisory.
- T3 preview used isolated tab `tab_3_a824dd12-a2c3-4a1d-bdb3-b8710da0468f` at `http://localhost:5198/`. Trusted browser right-clicks during held real `Animation.finished` waits cancelled empty and occupied choices; notes, dirty=false, full history and revision were identical before/after, and remained unchanged after completion. The occupied case kept history at **2** and preserved its note.
- A trusted browser click at root **125** observed native `hasPointerCapture=false`, no active gesture, and the expected MIDI rejection. Before its trusted pointer-up, injected leave/move, all six modifier down/up events, and a wheel continuation kept the provisional count at **0** and preserved notes/dirty/history/revision. Trusted pointer re-entry at root **120** resumed `[120, 124, 127]`. This distinguishes native capture evidence from the injected continuation events.
- A separate trusted browser drag of a valid stamp outside the grid observed capture and an active gesture throughout the outside move, with **3** provisional notes. Mouse-up released capture and placed three selected notes in one undo step. Temporary animation overrides/listeners were removed. Exact browser observations and event provenance are in ignored `target/piano-repair-qa/round2-preview.json`.

All four reported findings now have passing regressions: delayed-choice cancellation, ordinary leave/modifier suppression, pending right-click cancellation, and rejected-press capture/refresh suppression. The existing scale, rigid-group snap, atomic placement, undo/redo/persistence, and stale document/lane/reply tests remain green. The parent's store/flow authority, editor `follow`/dispatch methods, common canvas/coordinate helpers, rhythm tools and generated artifacts are unchanged by this repair.

This round used the parent's committed 166 binding files (164 TypeScript and two JSON) and 1,794,146-byte simulator WASM from `186652c6`. No Cargo process or artifact regeneration was needed.

Physical WebGPU rendering and native Tauri window focus behavior remain unverified. No renderer, Rust, rhythm-tool, shared store/flow, parity or generated-artifact changes are part of this repair.
