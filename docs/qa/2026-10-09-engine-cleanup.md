# Engine dead-code cleanup — 2026-10-09

> Final execution reconciliation: the combined tree passed the complete native workspace, strict checks, fresh artifact checks and final affected frontend checks. See [completion acceptance](2026-10-09-completion.md) for exact results and promoted IDs. Earlier pending statements below describe intermediate states; explicit remaining feature gates still apply.

The eight engine dead-code diagnostics in `2026-10-09-native-tests-resume.log` were inspected against repository-wide Rust references, including unit and integration tests. Changes are restricted to the six assigned engine files and preserve the existing dirty workspace.

- Removed `DepartureSource.effect`: departure rows already retain their effect id; consumers select generation ownership through `life`, and no consumer reads the duplicate field.
- Removed `DiskReader.rate`: the constructor still uses the supplied sample rate to compute the recording offset. Pump and tail timing use the capture gate's live output clock. The stored copy was never read.
- Removed the unused `ExternalEffect.process` forwarding wrapper. The effect rack continues to call `process_sidechain`, including the no-key case.
- Removed `EffectUnit.keeps_memory` and `Compensation.has_memory`: neither has a caller. Existing departure-source, tail, gap, and compensation-history logic remains in place.
- Kept the used `EffectUnit.process` convenience method under `cfg(test)`, since project-preparation unit tests call it. Production processing uses `process_sidechain` directly.
- Removed obsolete instrument note-start forwarding wrappers (`note_on`, `note_on_expression`, `start_note`). The processor uses `start_note_source` with expression and ownership information.
- Removed unused key-based instrument release/origin-conversion and sampler release helpers. The processor already releases owned note instances through `release_instance`; hardware origins are supplied when notes start. Existing hardware silence and origin fading stay intact.
- Removed unused rendering entry points (`InstrumentUnit.render`, `PlanState.render_instruments`). Production rendering uses `render_instrument_curves` / `render_curves` so held-note curves remain evaluated.

No lint allowances were added. No missing functionality was inferred from these fields: each removed helper has a used successor or stores a redundant value.

## Validation

`git diff --check` passed for all six assigned files (only normal Git CRLF conversion notices). Cargo was deliberately not run concurrently with the owner of the native QA gate.

The native QA owner must rerun the full engine tests and the strict workspace Clippy gate after these changes. Particularly relevant existing coverage includes project-preparation effect processing/departure and compensation tests, note-instance/channel-voice ownership tests, held-note curves, MIDI hardware release, and mixer disk recording. Until the rerun passes, this cleanup is implemented but not independently QA-verified.

## Subsequent strict Clippy pass

Ten diagnostics from `2026-10-09-native-clippy-final3.log` were addressed in the subsequently assigned `rack.rs`, `voice.rs`, `processor.rs`, and `channel_voice.rs`.

Equivalent let-chains preserve queue event selection (`at < end`), release-expression updates only when an envelope exists, strip modulation only when enabled, and final instrument curve evaluation before release. `?` replaces two `None` early returns without changing cut/steal ordering. The redundant `u64` latency cast was removed.

Three item-level `too_many_arguments` allowances retain existing internal realtime seams: instrument note data with timing/curve ownership, voice rendering with borrowed mix/plan/output and frame context, and voice start with prepared envelope ownership plus the borrowed plan/deferred-drop queue. Each has a local explanation; no module-wide allowances or scheduling/interface changes were made. Focused `git diff --check` passed. Native QA remains responsible for Clippy and engine regression reruns.

## All-target fixture repair

The next assigned compile diagnostics in `2026-10-09-native-clippy-all-targets.log` were repaired without production changes: native preparation/device/render plugin fixtures now carry default auxiliary selection metadata, preparation channels carry default voice/timing/group fields, sequencing transport patches default newly added fields, and checked streaming assertions match `RenderError::Plugin` while stem assertions retain `StemError::Plugin`.

The realtime all-effects test explicitly includes all 44 newly added effect variants in descriptor-range parameter sweeps, without a wildcard. Its fixture now distributes all 63 kinds over legal ten-slot racks: ten on the existing space track, nine on keys to reserve the existing cross-track move, and 44 over five additional tracks with sounding samplers. The previous two-track fixture would have exceeded the slot limit after enum expansion. Existing zero-allocation, audible-output and gain-reduction assertions are unchanged. Native compilation/regression reruns remain required.
