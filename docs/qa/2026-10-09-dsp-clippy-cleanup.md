# DSP strict Clippy cleanup — 2026-10-09

> Final execution reconciliation: the combined tree passed the complete native workspace, strict checks, fresh artifact checks and final affected frontend checks. See [completion acceptance](2026-10-09-completion.md) for exact results and promoted IDs. Earlier pending statements below describe intermediate states; explicit remaining feature gates still apply.

Five diagnostics outside the independently owned granular-parameter file were addressed from `2026-10-09-native-clippy.log`.

- `drive/chain.rs`: replaced `chunks_exact(5)` with `as_chunks::<5>().0` over the same 15-value slice. Both visit the same three stages in order, with no remainder.
- `performance/mod.rs`: moved the SilenceGate phase condition into its match guard. Earlier phases retain the existing wildcard no-op behavior.
- `spectral/vocoder.rs`: combined the band-count change and fresh-state checks. Banks still configure only when both hold.
- `granular/engine.rs`: retained the note-on interface and narrowly allowed `too_many_arguments` on that method. Its inputs mirror instance-aware note data plus a borrowed fixed-storage sample table and compact settings; wrapping or copying the table would not improve this realtime seam.
- `effect.rs`: narrowly allowed `large_enum_variant` on `EffectParams`. The fixed-size convolver impulse remains inline and the enum remains `Copy`, preserving allocation-free realtime parameter updates and the existing public/serialized parameter model.

Both exceptions have their rationale beside the annotated declaration. No crate/module-wide allowances, parameter schema changes, or audio algorithm changes were made.

`git diff --check` passed on the five files. Cargo execution remains exclusively owned by the native QA agent; strict Clippy and DSP regression reruns must pass before claiming QA verification. Relevant existing regressions cover drive chain stage order, performance SilenceGate phase behavior, vocoder bank transitions, granular note instances, realtime allocation, and parameter serialization.
