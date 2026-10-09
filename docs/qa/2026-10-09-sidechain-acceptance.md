# Sidechain integrated acceptance — 2026-10-09

> Final execution reconciliation: the combined tree passed the complete native workspace, strict checks, fresh artifact checks and final affected frontend checks. See [completion acceptance](2026-10-09-completion.md) for exact results and promoted IDs. Earlier pending statements below describe intermediate states; explicit remaining feature gates still apply.

The new `crates/windfall-engine/tests/mixer_sidechain_acceptance_qa.rs` target closes a missing public-processor test seam identified by the completion audit. It renders actual native `Processor` output from saved project routes rather than inspecting route flags or DSP effects alone.

Five cases cover:

- Silent detector-only copies into a normal effect chain, with an independent audible send to the same destination as the positive control.
- External peak and RMS compressor reduction versus internal detection, muted detector sources, required upstream sources under destination solo, and callback partition equivalence.
- Routing command, mixed-graph cycle refusal, serialization/reload, undo and redo, each checked against processor PCM.
- Different key arrival at successive rack slots around a preceding Stereo Matrix delay, with a subtraction probe and an audible positive control; blocks of 1, 64 and 127 frames.
- Saved auxiliary selection passed to the provider, replacement through a factory revision, an inactive saved input producing silence, and selection undo without replacing the provider owner.

The hosted provider is a deterministic test implementation of the public `HostedEffect` interface. It does **not** instantiate CLAP or VST3 or service the desktop native-owner exchange, so it must not be cited as proof of concrete native bus negotiation or bridge integration.

The native QA owner runs `cargo test -p windfall-engine --test mixer_sidechain_acceptance_qa`; no parallel Cargo execution was started here. Formatting and focused diff checks passed. Test execution remains pending until the native owner supplies a result.

`win-mixer-sidechain` remains partial. The historical source packet's broad claim that the desktop bridge provider is missing is stale: `apps/desktop/src-tauri/src/plugins/runtime.rs` already launches that provider, discovers authentic auxiliary inputs, and its direct native audio facade reapplies selections during reconciliation and forwards key slices.

A precise current bridge gap was found by source inspection: `plugins/bridge.rs` implements `HostedEffect` without overriding `set_sidechain_input` or `process_sidechain`, and its `process` method calls the adapter's ordinary realtime/offline APIs. The default trait methods therefore discard the engine's selection and detector slice. The underlying bridge adapter already exposes key-aware realtime/offline methods. This is reported to the parent agent for a separately owned product fix; the QA-only test file does not change it.

Other full-row gates are missing verification, not assumed missing implementation: concrete native multi-aux/mono/wider-layout mapping, native owner exchange, version-six key transport through the desktop provider, dynamic PDC/history transitions and the mounted routing UI. They cannot be inferred from the new engine fixture. Concrete native follow-up can use the existing `windfall-plugin-host/tests/common::SIDECHAIN` fixture (`org.windfall.test.sidechain`) and the real-helper launch pattern in `tests/bridge.rs`, with desktop-facade wiring in the acceptance path.
