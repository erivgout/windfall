# Analysis frontend repair QA — 2026-10-09

> Final execution reconciliation: the combined tree passed the complete native workspace, strict checks, fresh artifact checks and final affected frontend checks. See [completion acceptance](2026-10-09-completion.md) for exact results and promoted IDs. Earlier pending statements below describe intermediate states; explicit remaining feature gates still apply.

## Repair scope

The interrupted repair already present in this workspace was inspected and retained:

- `Backend` extends `AnalysisBackend`, requiring analysis methods on both implementations.
- `createMockBackend` installs `unavailableAnalysis()`. Browser capability explicitly reports native analysis unavailable, supplies no models, and rejects inference, import, review and apply.
- `createTauriBackend` forwards analysis operations to native IPC commands and preserves decimal-string job, request and ticket identifiers. Local model selection uses the native file dialog.

Inspection additionally found that the native analysis command functions existed but were absent from `tauri::generate_handler!`. This was reported to the native repair worker, who registered the commands. A cross-boundary regression in `features/analysis/protocol.test.ts` derives the actual `analysis_*` calls from the frontend source and requires each command to appear in the native handler. This guards the runtime routing omission that TypeScript and mocked invoke tests alone cannot detect.

## Executed evidence

From `apps/desktop`:

1. `bun run test src/features/analysis --maxWorkers=1` — exit 0, **3 files / 17 tests passed**. Start 07:17:43, duration 102.17 s. Import evaluation accounted for 73.52 s. This run began before the new handler regression was added.
2. `bun run test src/features/analysis/protocol.test.ts --maxWorkers=1` — exit 0, **1 file / 5 tests passed**, including the new handler registration regression. Start 07:19:37, duration 1.26 s.
3. `bunx eslint src/features/analysis/protocol.test.ts` — exit 0. Prettier formatting was applied to the updated test file.

Together these runs verify all 18 current analysis tests. Panel checks include honest browser unavailability without document/history changes, action registration and keymap/palette access, recording/busy/source-edit guards, canonical decimal identifiers, late submit/review/apply responses after context replacement, retained cleanup recovery, bounded recovery reservations, review provenance, and cancellation controls. Native adapter tests check exact IPC payloads and rejection propagation.

## Verification boundary

The analysis frontend adapter and automated panel/protocol tests pass. These tests use jsdom and mocked native invocations; the handler check examines native source registration. They do **not** prove an end-to-end desktop invocation, actual model inference, device-dependent execution or packaged-app behavior. Native runtime/build evidence belongs to the separate native QA report. Analysis should only receive full end-to-end QA status when that evidence and the required desktop smoke test are available.

No source changes were reverted, no commits were created, and no CI was triggered by this repair continuation.

## Native compilation repair continuation

The desktop compilation attempt exposed an analysis apply path still using the older `Controller::prepare_project` / `publish_prepared` contract and omitting `ClipContent::Audio.output`.

`session/analysis_jobs.rs` now preserves the source clip's Mixer/Direct output route and captures the shared `ProjectPreparation` baseline with the original document/source snapshot. Native preparation remains outside recording/State guards. Final apply keeps the existing file, full pool, loading/loaded/failed, sampler request, source binding, cancellation and analysis-ticket checks, then admits the borrowed ready engine publication **before** Document mutation. Successful apply commits desired parameters, installs the ready publication, publishes its prepared sampler pool, and retires old engine owners after guards are released.

Two Session regression tests were added: Mixer/Direct output routing inheritance and rejection of an independently superseded engine plan without document/history mutation or review consumption. Both changed Rust files were formatted with Rustfmt. Native compilation/test execution is owned by the native QA worker; this section records implementation changes, not a claimed native test pass.

## Application action registration harness timeout

The clean two-worker full frontend run reported `analysis/actions.test.tsx` as `Test timed out in 5000ms` (reported duration 6,092 ms). The prior single-worker run passed the same assertions in 4,096 ms. An unchanged focused two-worker rerun also passed: `bun run test src/features/analysis/actions.test.tsx --maxWorkers=2`, exit 0, one test passed, start 08:03:51, duration 11.78 s including import/transform/setup.

This test registers all application actions and exercises the actual command palette, keymap, dropdown action, and subscription teardown. It does not mount AppShell, and it asserts behavior rather than a performance budget. Its individual timeout was set to 10 seconds to allow shared-machine/two-worker jsdom scheduling overhead. Every assertion and the actual integration path remain intact; product source and performance-budget assertions were not changed.

After the per-test timeout update, `bun run test src/features/analysis/actions.test.tsx --maxWorkers=2 --reporter=verbose` passed (exit 0, one file/one test, test 4,286 ms; start 08:04:17, total 11.51 s). `bunx eslint src/features/analysis/actions.test.tsx` passed (exit 0), and Prettier reported the updated file unchanged. These are focused checks; the frontend QA worker owns the next complete clean suite run.

## Windows DOS namespace security repair

The full desktop native run (`2026-10-09-native-tests-verified.log`) demonstrated a real failure: `analysis_substituted_drive_retarget_after_hash_cannot_install_a_changed_source` panicked with **“remapped DOS drive source was applied”**. The test successfully remapped a local DOS drive after the final source hash/identity check while all original file/directory handles remained live. File and directory share authority did not protect the effective DOS drive object.

Production `SourceFile` now retains the effective NT DOS symbolic-link object before opening parent/file components. It admits only direct `\Device\HarddiskVolumeN` targets, and explicitly refuses substituted/chained DOS mappings with `analysis:sourceNamespace`. This is a fail-closed capability boundary, analogous to the existing junction/reparse refusal; unsupported mappings are not claimed to work. Drive, parent and file authority remain owned through final apply and retire outside State guards.

The substituted-drive regression now requires explicit refusal both before capture and after retarget, with unchanged document/history, source allocation and original bytes, and zero retained jobs. A new actual Session test imports a direct physical-volume drive mapping, attempts remapping at `analysis:prepared`, requires the effective mapping and bytes to stay unchanged, verifies successful apply consumption, and verifies remapping becomes possible after authority retirement. Native test execution for this repair is pending with the native QA worker; no green security result is inferred from source inspection.
