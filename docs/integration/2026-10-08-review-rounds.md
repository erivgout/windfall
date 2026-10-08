# Fixed-source review rounds, 2026-10-08

These reports keep standards and specification findings separate. They are
independent reviews of source not yet integrated into root. Owners received
all findings and their full prior briefs; the full project remains active.

# Timeline R3

Fixed source: `59c8f7f3adc0c552fca972b515e5219ddaf24d06`, parent
`e0809d0e1933e650667f379dbaf4687196d30865`, in
`gpt/t3-timeline-regions`.

## Standards

**Standards-only R3: three documented-standard findings.**

Reviewed fixed `59c8f7f3adc0c552fca972b515e5219ddaf24d06`, directly atop `e0809d0e1933e650667f379dbaf4687196d30865`.

1. **Hydration changes do not notify the action registry.** `apps/desktop/src/features/playlist/timeline-actions.ts:124` now reads `hydrated` for Clear’s enabled state, but `apps/desktop/src/features/playlist/actions.ts:508` watches only tool, selection and active. Hydration returning no region changes none of those values, leaving open registry consumers stale. The `docs/ARCHITECTURE.md:437` requires notification. Include `hydrated` in the selector.

2. **The Timeline dropdown duplicates registered actions.** `apps/desktop/src/features/playlist/timeline-controls.tsx:373` supplies separate titles, disabled predicates and handlers for Play/Loop/Zoom/Export/Clear. This bypasses the `docs/ARCHITECTURE.md:430`, omitting registry shortcuts and disabled reasons. Render these static entries through `ActionMenuItem`. Inherited from first delivery.

3. **Timeline entity IDs omit required newtypes.** `crates/windfall-project/src/timeline.rs:36` and its marker type use `pub id: u32`; update/remove commands likewise accept raw integers. The `docs/ARCHITECTURE.md:67` requires `u32` newtypes. Use serde-transparent identity types, preserving numeric JSON. Inherited from first delivery.

No additional recording/State, reserved-seam or callback-standard breach found. No separate baseline-smell findings.

Independent verification was fixed-blob source inspection; no tests, builds, edits or delegation. The owner’s 248 UI/77 native-engine passes and strict checks remain provenance. Worktree is clean at the fixed hash. Combined-root/artifact integration and physical/external-plugin/non-Windows verification remain gates; staged follow-ups remain open.

## Spec

**Spec verdict: changes requested — two P2 findings.**

Reviewed `e0809d0e1933e650667f379dbaf4687196d30865` → `59c8f7f3adc0c552fca972b515e5219ddaf24d06`.

1. **Chained pending Plays lose the live cancellation target.** `apps/desktop/src/features/playlist/timeline-store.ts:44` prefers the successor’s unpublished `guard` over its inherited `cancel`. Start Play A, start Play B while A’s Play is pending, hold B’s region publication, let A commit, then Clear before B commits. Clear targets B although native still owns A. The actual fixed-function harness ended with `playing: true`, owner request 1, watermark 3, and both native region and UI selection null. This violates the approved requirement that cancellation “must reconcile any transport it commits during invalidation.” Retain cancellation authority across uncommitted successors. Native consequences are source-traced.

2. **Song meters leak into pattern-mode plugin transport.** `crates/windfall-engine/src/processor.rs:712` consults the song meter map without checking playback mode. With scalar 4/4 and a song 7/8 change at tick zero, pattern playback sends hosted plugins 7/8 while pattern labels remain 4/4. The contract says “per-pattern signatures remain scalar.” Restrict song-map lookup to song mode. **Source-traced**, including forwarding through the native plugin adapter.

Independent verification: **10 cached native timeline tests passed**, single-threaded, after dependency-path, fingerprint, timestamp and inventory checks. Other R1/R2 closures were source-traced. The owner’s 248 shared-WASM UI passes, 77 native/engine passes and strict checks remain provenance; restored legacy WASM was not tested.

Worktree remains clean. No edits, builds, regeneration or delegation occurred. Combined-root and physical/external-plugin/non-Windows behavior remain unverified.

Standards: three documented findings; specification: two P2 findings.
All five are assigned to the same implementation owner. New identity types
must preserve numeric JSON. Registry notification/menu hooks and song-only
meter lookup have narrow authorized source windows. No finding is waived
because the owner's earlier tests passed.

# Release foundation R1

Fixed source: `671910694f4be9397e5bb63f9da730e920143f5d`, parent
`f7102e7c793c0153433edc4b7125d55d2fd0e8b7`, in
`gpt/t3-release-engineering-r1`.

## Standards

Standards review of `f7102e7c...67191069`: **0 documented-standard breaches; 2 nonblocking judgment-call smells.**

- **Possible Duplicated Code / Shotgun Surgery** — `scripts/release/cli.mjs:12`. The common options `"source", "commit", "version", "channel", "mode"` recur in three lists; command names also duplicate the `specific` map’s keys. A CLI contract change requires synchronized edits. Define common options once and derive accepted options and command names from the command map.

- **Possible Duplicated Code** — `scripts/release/source.mjs:375`. Two adjacent `git/matching-refs/tags/...` loops repeat the same normalization and `"release tag identity is already reserved"` check, differing only by the `v` prefix. Iterate over the two tag prefixes using one lookup/check body.

Workflow contract validation and syntax checks for all eight Node modules passed. Worktree remains clean. The reported 38 tests were not rerun because their fixtures invoke Cargo metadata, prohibited for this review.

## Spec

SPEC review of `f7102e7c…67191069`: **two findings; changes requested.**

- **P1 — Resource admission misses supported platform overrides.** `scripts/release/source.mjs:241` rejects only `tauri.PLATFORM.conf.json`. A future source commit can contain `tauri.windows.conf.json5` or `Tauri.windows.toml` adding model resources or helpers; `resources()` still reports `models/helpers: not-admitted` and inventories only factory content. The pinned Tauri CLI [enables these formats](https://github.com/tauri-apps/tauri/blob/30da1fd6e17de6107ecc850c95dfb16b5729f2dd/crates/tauri-cli/Cargo.toml#L59) and [merges platform configuration](https://github.com/tauri-apps/tauri/blob/30da1fd6e17de6107ecc850c95dfb16b5729f2dd/crates/tauri-cli/src/helpers/config.rs#L145). A bounded Node fixture confirmed the JSON5 override passes the resource gate. This violates “Future bridge/model resources must be an explicit validated packaging contract.” Reject every supported override format until admitted, or validate the effective merged configuration.

- **P2 — Independent verification does not bind SBOM declarations and edges to source metadata.** `scripts/release/dependencies.mjs:401` accepts arbitrary nonempty license declarations; its graph checks validate identities and node coverage, not expected edges. `scripts/release/candidate.mjs:282` relies on this validator. Starting with genuine fixture evidence, changing workspace licenses from `GPL-3.0-or-later` to `MIT` and deleting the desktop→core edge still passed validation. Updated inventory hashes/checksums would not repair the missing source binding. This falls short of evidence “from pinned Cargo/npm metadata/lockfiles” and rejection of “truncated metadata.” Recheck declarations and graph edges against authoritative metadata.

Workflow contract and actual lock readers passed; two selected tests passed. The reported 38-test suite was not rerun because it invokes Cargo. Probes used owned temporary directories and were cleaned up. The fixed worktree remains clean; no native or publication actions ran.

Standards: no documented breach and two nonblocking judgment calls;
specification: one P1 and one P2 finding. The resource-override and
source-bound SBOM repairs are assigned to the same owner, alongside pinned
upload-action provenance. Existing Tauri/version/updater metadata and
publication remain closed for this foundation. No release, upload, native
bundle, signature or installation was performed by either review.
