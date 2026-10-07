# CI portability follow-up, 2026-10-07

This change starts at `157f96fd068b46adab046a6880ce27ea4b49aea1`, after the private Windows alpha. It changes CI, bindings scripts and four precise Rust lint/test-helper/assertion sites. It does not change plugin runtime behavior or regenerate the checked-in bindings/WebAssembly artifact.

## Observed failures

Inspected [Actions run 37690189142](https://github.com/erivgout/windfall/actions/runs/37690189142) with `gh run view` and downloaded completed job logs using `gh api repos/erivgout/windfall/actions/jobs/<job-id>/logs`.

- [Rust (macos-latest), job 113028686011](https://github.com/erivgout/windfall/actions/runs/37690189142/job/113028686011): `cargo clippy --workspace --all-targets -- -D warnings` failed with `associated function fixture is never used` at `plugins/mod.rs:27` and `method selected_token is never used` at `plugins/runtime.rs:225`. Both helpers had `#[cfg(test)]`; their callers are Windows-only desktop plugin tests. Both now use `#[cfg(all(test, windows))]`.
- [Rust (ubuntu-latest), job 113028686105](https://github.com/erivgout/windfall/actions/runs/37690189142/job/113028686105): the same strict Clippy command failed with `this if statement can be collapsed` at `windfall-plugin-host/src/vst3/mod.rs:125`, in Linux's `ModuleEntry` call. The condition is now one let-chain; a present entry point is still called exactly once, and refusal still returns the same error.
- [TypeScript bindings are current, job 113028685995](https://github.com/erivgout/windfall/actions/runs/37690189142/job/113028685995): generation completed, then `git diff --exit-code -- apps/desktop/src/bindings` failed only on `automation-fixtures.json`. Computed `f64` shape samples differed by last-bit platform math results, for example `0.45230952713087613` on Windows versus `0.4523095271308762` on Linux. No TypeScript or descriptor changes appeared in that job's diff.

The Rust test steps on macOS/Linux were skipped after Clippy failed. UI, mock-document freshness and parity jobs passed. Windows was still running when those completed-job logs were retrieved; that status is not a claim that Windows CI passed.

Real Linux container execution then uncovered a failure that Clippy had masked: `lifecycle_gain_offsets_transport_state_and_restart` expected `Err(NoEditor)` but received `Err(UnsupportedPlatform)`. VST3 native editor support is explicitly Windows-only. The test now expects the appropriate platform error while continuing to run the complete lifecycle/audio/state assertions on both Windows and Linux.

## Checks retained and strengthened

The Rust matrix retains all three runners, `cargo fmt --all --check`, strict workspace/all-target Clippy, and workspace tests. Ordinary Rust tests export TypeScript to a runner-temporary directory so they cannot rewrite checked-in UI files.

Bindings freshness now runs independently on Windows, macOS and Linux. `scripts/gen-bindings.sh [output-directory]` exports through a native absolute `TS_RS_EXPORT_DIR`, sources the MSVC environment on Windows, and fixes the barrel ordering to the C locale. It runs the three exporting libraries (`windfall-dsp`, `windfall-project`, `windfall-ipc`) and their referenced type exports instead of compiling the desktop shell. Generation still writes the descriptor and automation fixtures.

CI generates into a fresh temporary directory and runs `node scripts/check-bindings.mjs <generated-directory>`. This compares the complete file inventory, catching new untracked types, obsolete types, changed types/barrels/fixtures, and empty exports. Checkout CRLF is normalized. Only `automation-fixtures.json`'s computed `shapes[].values[]` may differ by at most four adjacent representable positive `f64` values (ULPs). Shape inputs, sample counts, f32 curve outputs, ranges and descriptor metadata remain exact. This allowance accounts for the observed platform math difference without discarding fixture checks or rounding other numeric data.

The checker regression suite uses the actual Windows/Linux numbers and verifies the four-ULP boundary, rejection of five-ULP drift, exact inputs/metadata, and all inventory failure modes.

## Integration ownership

When reconciling VST3 runtime work, retain the two `#[cfg(all(test, windows))]` helper gates unless the corresponding fixture callers have also been made portable. The Linux let-chain is only lint cleanup. No instance ownership, active-state capture, recording exclusion, audio callback, editor retirement or revision/token behavior was changed. Final combined binding and WebAssembly generation belongs to the parent integration task.

## Verification scope

Local verification uses Windows x64/MSVC with Rust/Cargo 1.99.0, Node 26.4.0 and the Visual Studio 2022 Build Tools selected by `scripts/msvc-env.sh`. Tests use task-specific temporary `TS_RS_EXPORT_DIR` paths. Workflow expressions are checked with actionlint 1.7.12; its external shellcheck/pyflakes integrations are disabled because those executables are not installed.

Passed locally:

- `cargo fmt --all --check`.
- `cargo clippy --workspace --all-targets -- -D warnings` on Windows.
- `cargo clippy -p windfall-plugin-host --all-targets --target x86_64-unknown-linux-gnu --target-dir target/ci-linux-clippy -- -D warnings` from Windows. This compiles/lints Linux paths; it does not execute Linux tests.
- `cargo test -p windfall-desktop --lib session::tests::plugin`: five native session/plugin tests, including recording exclusion, stale native updates, installed-instance ownership and export/state round trips.
- `cargo test -p windfall-plugin-host`: all Windows host unit/integration tests, including the real synthetic native editor, scanners, VST3 lifecycle and realtime allocation checks.
- After the portable editor-error assertion changed, its Windows lifecycle test and both strict Clippy commands were rerun successfully.
- `cargo test -p windfall-plugin-host --quiet` in the real x86_64 Linux `rust:1.99-slim` container: 92 unit/integration tests passed, including the previously failing lifecycle test, VST3 scanning, crash/hang isolation and realtime allocation checks. Windows editor tests remain correctly Windows-only. The Rust image digest was `sha256:24e632c09342c20abf8312cf4f61430a911c01ed3a5e4c02b87292b1c39c5273`.
- Fresh Windows bindings generation: all 139 files match, including the JSON fixtures.
- Windows generation invoked from outside the repository into an output path containing spaces, with a conflicting inherited `TS_RS_EXPORT_DIR`: all 139 files match, and the inherited output directory is not written.
- Fresh Linux bindings generation in a real `rust:1.99-slim` x86_64 Linux container: all 139 files match the portable comparison. The container uses read-only repository sources and task-specific output/target directories; this is not macOS or desktop/hardware verification.
- `node --test scripts/check-bindings.test.mjs`: ten regression tests passed on Windows and in Linux `node:25-alpine`; the Linux Node process also checked the actual Rust-generated Linux bindings successfully.
- `bash -n scripts/gen-bindings.sh scripts/build-sim.sh scripts/msvc-env.sh`, Node syntax checks, actionlint, and `git diff --check`.

macOS execution, hardware audio and installed third-party plugin compatibility are not verified by this change. No push, PR, tag, release or visibility change was made.

Final inspection of the original base run at `2026-10-07 22:00:37 UTC` still showed Windows job `113028686025` in progress. The three failures described above were completed and inspectable. This branch has not been submitted to hosted Actions; validating the combined integration there remains the parent's publication step.

## Integration follow-up: realtime-feed scheduling

Hosted run `37694612849` at `c37ae0b9` passed Linux Rust, UI, WASM/parity freshness and bindings freshness on Windows/macOS/Linux. Its macOS Rust job `113043374087` passed Clippy but failed the realtime-feed test: it delivered 19 frames in a window whose wall-clock duration implied approximately 37. Parallel CI can suspend this non-audio thread; the production scheduler intentionally skips a large backlog instead of emitting a burst. A minimum percentage of nominal wall-clock frames is therefore not a portable correctness assertion.

The deadline calculation is now tested with virtual timestamps for 600 frames of variable processing work, the nominal 60 Hz interval, short lateness and recovery after a long suspension. The live session test waits for three delivered frames with a bounded timeout and verifies that dropping the session ends the thread. The scheduling policy is preserved. Real app/device performance remains a measured acceptance gate, separate from these correctness tests. Hosted macOS execution of this follow-up remains pending publication.
