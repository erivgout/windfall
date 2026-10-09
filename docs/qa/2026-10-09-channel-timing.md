# Channel timing acceptance — 2026-10-09

> Final execution reconciliation: the combined tree passed the complete native workspace, strict checks, fresh artifact checks and final affected frontend checks. See [completion acceptance](2026-10-09-completion.md) for exact results and promoted IDs. Earlier pending statements below describe intermediate states; explicit remaining feature gates still apply.

New independent native acceptance tests are authored for the already implemented shared `ChannelTiming` workflow. They use public APIs and were not executed by this author: the native verification worker owns compilation and runs so builds do not compete. Source product files were unchanged.

## Authored checks

- `crates/windfall-project/tests/channel_timing_qa.rs` (three tests): signed shift boundaries, swing mix/gate command edits, undo/redo, actual disk save/load, absent legacy timing defaults; invalid NaN/infinite/out-of-range controls and i32 minimum reject atomically without history changes; independent numeric expectations for half/full/zero mix, post-swing gate, positive/negative shift, onset clamping/end exclusion and unpaired trailing steps across varying pattern lengths.
- `crates/windfall-midi/tests/channel_timing_qa.rs` (one test): actual pattern export applies signed shift, half swing and gate with exact expected note ticks; original project notes remain unchanged; SMF write/read reproduces normalized exported timing; export swing opt-out retains explicit gate/shift.
- `crates/windfall-engine/tests/channel_timing_qa.rs` (one test): native production processor plays impulse notes on independently calculated exact frames at 48 kHz/120 bpm; channel mixes 0/0.5/1 and signed shifts include clamp-to-zero; three callback partitions match the same onsets and the project remains unchanged. Shared existing engine fixture helpers are imported by path; no private module or production source changes are required.

Targets communicated to the native worker:

```
cargo test -p windfall-project --test channel_timing_qa
cargo test -p windfall-midi --test channel_timing_qa
cargo test -p windfall-engine --test channel_timing_qa
```

Only these new test files were formatted directly with rustfmt. The initial workspace `cargo fmt --all -- --check` encountered shared in-progress parser errors; their owners/native worker corrected those locations. That workspace result is not a test pass and no unrelated file was changed by this author.

## Row eligibility

`win-rack-swing` and `win-rack-note-timing` are conditional completion candidates if these tests pass and the frontend worker verifies the actual per-channel controls/history dispatch on matching generated artifacts. Dedicated numeric and native integration evidence is now authored instead of inferring acceptance solely from generic command mappings. Until execution succeeds, the tests prove intended acceptance criteria only. The engine fixture proves onset timing; gate duration is checked independently by kernel and MIDI assertions, not claimed as an engine tail measurement.

Frontend follow-up: the mounted actual-WASM `channel-rack/timing.qa.test.tsx` passed its one acceptance case. It covers half swing, quarter gate, signed shifts, preservation of other fields, one history step per change, file persistence, undo/redo and reset as one undo step. Frontend lint of the new QA files also passed. Native targets remain awaiting the execution worker.
