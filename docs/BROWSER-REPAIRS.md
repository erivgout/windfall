# Browser-library repair evidence

Original base: `4b0d509db734e6b0a8e7ec5312f04867f8bdb4d5`. Branch: `gpt/t3-browser-repairs`. Implementation worktree: `C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-browser-repairs`.

This repair addresses the three findings reviewed at `433234e0947160d630e888ebc3470525a4c8e41f`. The recursive index, shared query language, favorites/tags and existing audition/import workflows remain in place. Parent project-store/flow, piano, artifact and parity work is excluded.

The second repair round starts from parent `bde3fd77dacdd9c1d4c92cf3a77e5c4d1d1eab55`, fast-forwarded in this same worktree. That parent already contains first repair `d90db39e4cf3aa8615145a9b496f790f08efd6be`. The evidence below retains the first round, then records the three additional findings reviewed at the parent commit. No parent-owned store/flow or sampler preparation files are changed by this round.

## Reproductions before the corresponding fixes

`pnpm exec vitest run src/features/browser/repairs.test.tsx` from `apps/desktop` initially failed all nine tests. Six held `libraryFile` across the real `newProject`/`openProjectPath` flows and real mock/WASM document events, then answered the lookup using the backend's current library generation. Rack/playlist imports changed the replacement document and replacement imported into a channel with a reused ID. Three rendered the browser with `librarySearch` held, selected/auditioned a tree file, refreshed, then imported without reselection; every destination changed the project. Backend methods were delayed, not replaced by fabricated document mutations or manual generation announcements.

`cargo test -p windfall-desktop --lib checked_import_refuses_reusing_a_loaded_older_file_version_at_every_destination -- --nocapture` failed before the native fix with `rack accepted new audio but retained 480 old frames`. The test wrote a real temporary WAV, imported 480 frames, rewrote it to 960 frames, refreshed/reselected, and confirmed native facts/preview accepted the newer file before the project imported it while retaining its older pool buffer. The isolated initial build took 18m26s; the deterministic test itself took 0.20s. The same regression passes after the fix and covers all three destinations.

## Repairs

Browser actions capture `getProjectGeneration()` before lookup and retain it through every await. The playlist helper additionally checks after mixer-routing lookup and after its import reply, before patch application/selection. Old replies cannot select or reveal objects in a replacement project.

Every non-folder selection created by `selectionOf` captures an immutable root path and library epoch independently of search results. Request objects capture that selection before a timer/await. Refresh/root changes advance the epoch; query and metadata revisions do not. Token lookup verifies the captured epoch/path/root before accepting a reply, and only attaches it when the exact captured selection is still current. Audition, facts, all three import actions and selected-file dragging use the pinned token. Search replies also check the epoch immediately, including the interval before React cleans up the old effect.

Loaded-source reuse compares the native cache's file-version key, including OS file identity, length and modification time. Weak audio identity records keep provenance for live project sources after the LRU evicts their decoded-cache entry. Concurrent separate decodes of the same version still compare equal. Unknown or changed versions refuse the import before document dispatch; no audible buffer is replaced outside undo. Unchanged files retain one sample. Native import lock order remains recording exclusion, library generation guard, State; version comparison only inspects in-memory metadata.

## Focused verification

UI command: `pnpm exec vitest run src/features/browser src/lib/ipc/library.test.ts src/features/playlist/audio/audio-clips.test.ts src/features/playlist/audio/audio-ui.test.tsx` — 11 files, 196 tests passed. This includes initial pending-search refresh regressions, all destinations across New/Open and own-root removal/re-addition, delayed token attachment, delayed import replies, playlist routing lookup, drag invalidation and the established browser/audio workflows.

`pnpm exec tsc -b` and ESLint on all changed TypeScript modules passed. Prettier checked the changed TypeScript modules; final formatting is checked before commit.

Native commands, run sequentially from the worktree with the resource environment below:

- `cargo test -p windfall-desktop --lib session::tests::library -- --nocapture` — 12 passed. Tests cover changed-version refusal at rack/playlist/replacement, exact refusal history/pool preservation, post-save/reopen current-source reuse and undo, deduplication after cache eviction at every destination, and real native New/Open barriers at every destination with reused channel IDs. Existing recording/root-removal/audition/metadata tests remain green.
- `cargo test -p windfall-desktop --lib samples::tests -- --nocapture` — 6 passed. This includes same-size/same-timestamp OS file replacement, surviving provenance after cache eviction, unknown-source refusal and weak-record cleanup.
- `cargo test -p windfall-core --lib` — 6 passed, including clone/empty-buffer identity and proof that identities do not retain audio.
- `cargo test -p windfall-desktop --lib library::tests -- --nocapture` — 7 passed, preserving recursive search, metadata, bounds, cancellation/supersession, stale roots/files and Windows junction-cycle behavior.
- `cargo clippy -p windfall-desktop -p windfall-core --lib --tests -- -D warnings` — passed with no warnings (6m31s).

All 31 focused native tests passed. `rustfmt --edition 2024 --check` on the four changed Rust files and `git diff --check` passed. The TypeScript lint invocation was `pnpm exec eslint src/features/browser/commands.ts src/features/browser/file-token.ts src/features/browser/library-store.ts src/features/browser/store.ts src/features/browser/preview.ts src/features/browser/tree-view.tsx src/features/browser/library-controls.tsx src/features/browser/repairs.test.tsx src/features/playlist/audio/ops.ts`; `pnpm exec prettier --check` on those same files passed.

## Second-round reproductions and repairs

Before production changes, `pnpm exec vitest run src/features/browser/repairs.test.tsx --maxWorkers=2 --silent` failed five new regressions while all 19 first-round regressions passed. Two rendered `ChannelRackPanel` and fired its actual rack-scroll drop handler with a checked browser token. The real backend mutated/emitted first, its successful reply was held, then real New/Open replaced the document with one containing the same created channel ID (15). Releasing the old reply left the project intact but selected the replacement project's unrelated channel. The New case begins with the real New template and recreates the same ID through the real backend; Open reloads the saved project. The demo project's different ID allocation and a closed nested-root row were corrected in test setup before recording this RED evidence.

The other three RED cases configured `/factory/Drums/Kicks` inside `/factory`, held `librarySearch`, expanded the nested-root tree row and selected `Kick 02.wav`. The real fixture backend returned the canonical `/factory` token, matching native first-containing-root lookup. All three imports were incorrectly refused. After repair, those same rendered cases verify audition, ready waveform/facts, token attachment and rack/playlist/replacement import while search remains pending.

`cargo test -p windfall-desktop --lib import_refuses_pending -- --nocapture` ran six native regressions before the loading guard. All six failed with the destination-specific message `accepted Some(960) frames before pending reload/redo installed 480 older frames`. Three created a genuinely missing linked WAV, waited for failure, restored 480 frames and held `samples_reload()` at `samples:decoded`. Three first imported 480 frames, undid the import, evicted its decoded-cache entry with 65 real WAV decodes and held the real redo loader at the same barrier. Every case then rewrote the source to 960 frames, refreshed/reselected, confirmed native facts saw 960 frames, imported, released the old loader and measured the pool returning to 480 frames. No synthetic loading/pool mutations are used. Parent-source recompilation took 5m26s; the six RED tests ran in 0.16s.

Checked imports now refuse reuse while `State.loading` contains that sample ID, before either dispatch path mutates the document. This check and dispatch share the State lock, after recording exclusion and the library guard. It covers both explicit missing-source reload and background redo. Decoding and filesystem checks remain off State/audio locks. The outstanding load is allowed to finish normally; version mismatch refusal still applies afterward. No loaded audio is silently superseded.

Token acceptance now permits different captured/canonical roots only when both are still configured and both contain the exact same file. The selection's immutable root/epoch is retained. Six rendered regressions hold canonical lookup through refresh or nested-root removal/re-addition before each import destination; late tokens neither mutate the project nor attach to the expired selection. Seven focused token tests also reject unconfigured/sibling roots, wrong files and expired epochs. The rack's `addChannelFromFile` captures project generation before IPC and checks it before reply patch handling or selection.

## Second-round focused verification

UI command: `pnpm exec vitest run src/features/browser src/features/channel-rack/rack.test.tsx src/lib/ipc/library.test.ts src/features/playlist/audio/audio-clips.test.ts src/features/playlist/audio/audio-ui.test.tsx --maxWorkers=2` — 13 files, 256 tests passed. This preserves the existing browser, rack drop/replacement/undo and playlist workflows and includes all first/second-round UI regressions. `pnpm exec tsc -b` passed. ESLint and Prettier passed on `src/features/browser/file-token.ts`, `file-token.test.ts`, `repairs.test.tsx` and `src/features/channel-rack/channel-ops.ts`.

`cargo test -p windfall-desktop --lib session::tests::library -- --nocapture` — 18 passed, 0 failed. Recompilation after the guard took 28.53s; the tests ran in 0.24s. The six reload/redo cases prove refusal preserves the exact pre-import document snapshot, including history and playlist geometry, while the original pending 480-frame source finishes normally. Each then saves/reopens to explicitly load 960 frames, imports the unchanged current version without adding duplicate samples and undoes the edit. The 12 earlier native browser regressions also remain green.

`cargo clippy -p windfall-desktop --lib --tests -- -D warnings` — passed with no warnings in 1m42s. The existing isolated Cargo/TS export directories and single-job environment below were reused; Cargo commands ran sequentially. `rustfmt --edition 2024 --check apps/desktop/src-tauri/src/session/library.rs apps/desktop/src-tauri/src/session/tests/library.rs` and `git diff --check` passed. No cancelled or unfinished check is counted as a pass.

Native checks use `CARGO_BUILD_JOBS=1`, `CARGO_TARGET_DIR=<worktree>/target/browser-repairs` and `TS_RS_EXPORT_DIR=<worktree>/target/browser-repairs-ts`. This work's Cargo commands use one process at a time. VS 2022 BuildTools 14.44.35207 and Windows SDK 10.0.26100.0 supply the compiler, include and library paths; the newer VS install lacks desktop x64 libraries. Initial environment attempts failed at the linker before any native test ran; the final commands use the complete installed toolchain.

## Limits and integration

Headless native tests verify actual temporary files, session/document/pool behavior and deterministic decode barriers. UI tests use the existing WASM document/backend and its event ordering; they do not claim OS IPC transport or physical-device verification. No hardware playback, desktop-window interaction, other operating system verification, full-suite rerun or release build is claimed.

The browser fixture mock cannot model real disk-version replacement. That reproduction belongs to the native session/cache tests. Identity remains a filesystem stamp, not a content hash: an in-place write preserving the tracked identity, size and timestamps remains a known limitation.

Generated TypeScript/WASM artifacts, parity status and integration documentation remain the parent's responsibility. No push, PR or release is part of this work.

Second-round checks are at the integrated `bde3fd77` source plus these repairs. They do not include the parent's later sampler/plugin/archive integrations or regenerated artifacts. The parent must combine and validate that later source; this round changes only browser token handling, the rack file-import reply guard, the checked-import loading guard, focused tests and these two browser documents.
