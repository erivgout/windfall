# Expanded DSP runtime harness repair

> Final execution reconciliation: the combined tree passed the complete native workspace, strict checks, fresh artifact checks and final affected frontend checks. See [completion acceptance](2026-10-09-completion.md) for exact results and promoted IDs. Earlier pending statements below describe intermediate states; explicit remaining feature gates still apply.

The broad native run found two stale generic slot assumptions and one stale filter-family registry count. These are test-contract repairs; no product DSP source changed.

- `slot::any_effect_reports_what_it_holds_and_refuses_other_settings` assumed only Limiter and Distortion had latency. The explicit independent list now includes the expanded mastering, harmonics, performance, convolution and spectral kinds. Exact processor latency must also agree with serialized parameter latency, and maximum latency must bound it. Existing impulse/delay reference tests remain unchanged. The all-kind signal probe now processes 96,000 samples in 256-frame blocks so longer legitimate delay effects can emit audio.
- NoteEnvelope intentionally has a closed default gate with full modulation depth, yielding silence. The generic test explicitly verifies that default silence, then opens its gate. Both exhaustive native processing and bad-input recovery retain finite, nonzero-output assertions with the audible gate configuration. No effect is skipped.
- `filter_family_registry` now asserts the current 63-kind inventory, verifies every enum discriminant/index and unique serialized tag and parameter ID, and preserves the original first-twelve tags/discriminants and this filter family's appended slots and exact descriptors.

Owned rustfmt and diff checks pass. Native execution remains pending the native owner's focused and full runs:

```text
cargo test -p windfall-dsp --test dsp slot::
cargo test -p windfall-dsp --test filter_family_registry
```
