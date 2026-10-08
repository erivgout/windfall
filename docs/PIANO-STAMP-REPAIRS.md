# Piano stamp cancellation and pointer-presence repairs

These repairs address the two findings against `46de77e38dc40caf3a1493abe3454d24a4385f18`, on integrated base `73149138c6158b0dc5e73604cdf19abaed801006` in `gpt/t3-piano-scales`.

## Pending menu choices

Previously, choosing a stamp saved a local menu ref until Base UI finished closing. Blur or a tool change could cancel the editor during that interval, but the delayed callback subsequently armed the saved choice.

`Editor.deferStamp` now gives each pending choice cancellable ownership of its completion. Pending choices count as busy, so existing cancellation actions can reach them. Cancellation, lane/note/length/signature changes, project replacement and disposal invalidate that ownership; returning to the original lane or tool does not restore it. The menu also cancels its pending choice on window blur, Escape, direct tool changes and unmount. Escape reaches this pending choice even while the closing menu still owns focus. Completion checks the current document, lane and tool before arming. An invalidated callback neither arms a stamp nor focuses the grid.

Ordinary menu selection still waits for the exit transition, then arms once and focuses the grid. Cancellation creates no notes or undo entries. The editor's existing `follow`, command dispatch and created-selection recovery methods are unchanged, as are the shared project store and flow authorities.

## Preview outside the grid

Previously, pointer leave removed a stamp preview, but a global modifier event reused the retained pointer coordinates because an armed stamp counted as busy. Provisional-note counts were **3 → 0 → 3**.

Grid input now tracks whether the pointer is inside the grid, including the coordinates of captured pointer moves. Idle modifier and wheel refreshes require pointer presence. The existing held pointer frame continues to allow modifier updates during captured drags outside the grid; the last coordinates are retained. Re-entry immediately refreshes the preview. Releasing an outside gesture also clears idle hover/preview state.

## Regression evidence

Before the repair, real-menu tests reproduced re-arming after blur, tool actions/setters, Escape, a lane round trip, a lane edit followed by undo, and disposal followed by reattachment. Actual grid-input tests reproduced all three modifier failures for Alt, Shift and Ctrl. The menu tests hold Base UI's actual `Animation.finished` wait with a controlled promise, then release it after cancellation; they do not invoke `onOpenChangeComplete` directly or depend on a CSS animation duration.

The new `stamp-lifecycle.test.tsx` has 21 tests. It also verifies ordinary arming/focus, menu unmount, pattern-length/signature changes followed by undo, project generation cancellation, modifier down/up while outside, re-entry, captured Alt/Shift draw behavior and Ctrl copy drags outside. Real Rust WASM transactions verify created selection after held revision-gap recovery and rejection of late selection updates after lane switching, project replacement and disposal.

Validation in this worktree:

- `pnpm exec vitest run src/features/piano-roll/piano-roll.test.tsx src/features/piano-roll/stamp-lifecycle.test.tsx src/features/piano-roll/scale-stamp.test.tsx src/features/piano-roll/scales.test.ts --maxWorkers=2`: **96 tests in 4 files passed**. Two workers were allowed concurrently; no full repository suite was run.
- `pnpm exec eslint src/features/piano-roll/editor.ts src/features/piano-roll/grid-input.ts src/features/piano-roll/stamp-menu.tsx src/features/piano-roll/stamp-lifecycle.test.tsx`: passed.
- `pnpm typecheck`, `pnpm build` and `git diff --check`: passed. The production build retains the existing large-chunk advisory.
- T3 preview used its own isolated tab, `tab_h_6a86cc21-32fb-473a-bf54-e9b0999bfdbb`, at `http://localhost:5198/`. During a held real menu exit, a dispatched window blur and the registered Paint tool action each left the stamp null after completion. Ordinary selection armed Major triad and focused Note grid. Real browser pointer hover/leave and Alt/Shift/Ctrl down/up yielded **3 → 0 → 0**, with all six modifier events leaving the preview hidden; real pointer re-entry restored **3** notes. Escape then cleared the stamp and preview. The temporary animation instrumentation was removed afterward.

The repair uses the parent's committed 163 binding files (161 TypeScript and two JSON) and 1,775,678-byte simulator WASM. No Cargo process or artifact regeneration was needed. Obsolete artifacts from this worktree's earlier verification were backed up under ignored `target/piano-repair-artifact-backup/` before fast-forwarding. Browser observations are saved under ignored `target/piano-repair-qa/`.

Physical WebGPU rendering and native Tauri window focus behavior remain unverified. No renderer, Rust, rhythm-tool, shared store/flow, parity or generated-artifact changes are part of this repair.
