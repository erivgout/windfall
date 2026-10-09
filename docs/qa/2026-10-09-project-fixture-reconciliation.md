# Project fixture reconciliation — 2026-10-09

> Final execution reconciliation: the combined tree passed the complete native workspace, strict checks, fresh artifact checks and final affected frontend checks. See [completion acceptance](2026-10-09-completion.md) for exact results and promoted IDs. Earlier pending statements below describe intermediate states; explicit remaining feature gates still apply.

Nine failures from the broad native runtime log were diagnosed in `windfall-project/tests/commands.rs` and `tests/file.rs`. Changes are test-only.

- The mixer now reserves a separate Current utility seat: ordinary add-track/add-channel fixtures fill `MAX_MIXER_SIGNAL_TRACKS` (500 inserts plus Master), rather than attempting to fill the larger total physical capacity. Full-mixer channel fallback still must produce only a channel routed to Master, and the completed project still passes validation. The broken-capacity test still requires validation rejection, with its current explicit diagnostic wording.
- The final EchoBank unit's next-send descriptor has identical minimum/maximum values because it has no next destination. Dispatch correctly returns its parameter label but changes nothing, so the fixture now asserts no touched sections and unchanged history for this fixed descriptor. Changed parameters retain the original exact history-label assertions.
- Exact track/instrument serialization goldens now include the added default channel voice, Current flag, manual-latency offset and compressor detector flag. Before updating, parsed old and observed new JSON were compared with only these new default fields removed; all prior values were equal.
- The legacy `DEMO_JSON` reader fixture remains unchanged. A separate explicit `CURRENT_DEMO_JSON` golden records the writer's current schema. Stable bytes, legacy load defaults and save/load equality remain required.

Focused `git diff --check` passed. The native Cargo owner was asked to rerun `cargo test -p windfall-project --test commands --test file`; passing execution evidence is required before marking this reconciliation verified.
