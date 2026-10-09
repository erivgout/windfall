# Echo bank and sixteen-band frequency delay QA

> Final execution reconciliation: the combined tree passed the complete native workspace, strict checks, fresh artifact checks and final affected frontend checks. See [completion acceptance](2026-10-09-completion.md) for exact results and promoted IDs. Earlier pending statements below describe intermediate states; explicit remaining feature gates still apply.

Scope: `fx-fruity-delay-bank` and `fx-multiband-delay` against their exact parity summaries, on the current shared dirty workspace. This report does not change the acceptance contract in `docs/DELAY-FAMILY.md`.

## Source findings

- EchoBank implements eight independently enabled delay units. Each has input and feedback filters, stereo controls and a signed send to the following unit. Native processing sums their parallel outputs while passing the serial send between units. This implements the parity summary's chainable filtered delay bank.
- FrequencyDelay implements **sixteen** bands, not the older three-band inventory description. Fifteen cumulative complementary crossovers produce sixteen frequency contributions; each has its own delay, level, pan and enabled control. Its registered descriptor table exposes all four controls for all sixteen bands.
- Both kinds are registered in `EffectKind`, parameter dispatch and `AnyEffect`. The frontend parameter-access inventory includes both, and `EffectEditor` exposes them through its generic descriptor-based editor with command dispatch and automation binding.

## New executable acceptance coverage

`crates/windfall-engine/tests/delay_bank_acceptance_qa.rs` adds two public engine integration tests:

1. EchoBank: two-unit fixture with the second unit receiving only serial input; registered next-send control changes finite, nonzero native PCM. Editing the second unit's lowpass cutoff changes the linked audio. Undo/redo restores exact PCM; every unit's routing/time/filter descriptors are present.
2. FrequencyDelay: verifies delay/level/pan/enabled descriptors for all sixteen bands, then edits one band's delay and a different band's level and pan through public project commands. Each edit changes native PCM, other band state stays intact, and undo/redo restores exact PCM.

Both tests save/reopen the real project file, compare all effect-slot parameters and native PCM, and compare fresh public `Processor` playback with offline render across 7-, 64- and 127-frame block partitions after the host latency offset. A two-frequency sample fixture exercises lower and upper frequency regions. This is signal verification, not acoustic equivalence to FL Studio.

Owned `rustfmt` and `git diff --check` pass. **Native execution is pending the native QA owner's run**; the authored assertions are not recorded as passing yet. Run:

```text
cargo test -p windfall-engine --test delay_bank_acceptance_qa
```

## Completion judgment

Keep both rows in progress while updating their notes to recognize the implemented registry, sixteen-band topology, usable generic controls and new native acceptance coverage once executed. `docs/DELAY-FAMILY.md` explicitly keeps purpose-built linked-unit and drawn sixteen-band editors in its acceptance scope and says a large generic list does not close either row. Those editors remain absent. The document also retains fallible host preparation admission and saturating engine tail aggregation as host gates; the current engine tail aggregation still uses ordinary `usize::sum` in `state.rs`. No product source was changed in this QA task.
