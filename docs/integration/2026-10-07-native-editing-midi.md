# Native MIDI, editing, slicing and plugin ownership integration

This local integration combines MIDI hardware `98a914ca`, immutable audio editing `de5da276`, clip slicing `eab66af9` and bounded native plugin ownership exchange `d2bb8902`. Their merges are `7e8984ea`, `d6f5aaba`, `f060022f` and `11e1a54b`. The full-project roadmap `aee8be65` is integrated through `3e188b67`. These are development changes after the immutable `v0.1.0-alpha.1` release, not an updated installer.

## Review repairs

- The combined bindings were regenerated: 146 files, including native audio-editor and MIDI wire types. The editor now imports generated types. The pinned browser WASM was rebuilt to 1,634,799 bytes and exercises the real Rust `slice_analyze` and `slice_command` exports.
- Audio-editor and slicer Apply replies now check the existing UI document-generation signal and component lifetime before applying patches or changing selection. Deferred-reply tests replace a project with reused revisions or unmount the dialog before resolving the old result. Analysis/opening replies have equivalent stale-result guards.
- Native and browser slicing explicitly refuse tempo automation before analysis or mutation. Constant-tempo source-offset math cannot preserve slices across a tempo map. Grid/transient refusal tests retain the document/history and original native WAV bytes.
- A MIDI/sampler integration regression checks forward and ping-pong loops beyond source duration, sustain/key release, pedal-up release, retrigger and panic. Every measured callback has zero allocation/reallocation/deallocation calls.
- A macOS hosted CI timing failure is repaired with deterministic realtime-feed deadline tests and a bounded live delivery/lifetime test. See [CI portability](../ci-portability.md).

## Checks observed

Before the review repairs, the combined source passed strict workspace/all-target Clippy, the full Rust workspace suite (1,657 passed, eight intentional ignores), plugin-host all-feature tests (97 passed) and strict all-feature host Clippy. The full UI suite passed 2,129 tests across 136 files.

After the editing repairs, the three editor/slicer UI test files passed all 24 tests. UI lint and the production TypeScript/Vite build passed. Native slicing passed nine tests, MIDI hardware passed eight tests, realtime-feed tests passed three tests and the session playback suite passed 14 tests. Strict all-target Clippy for desktop/engine passed, including a follow-up desktop check after the scheduling repair. Generated WASM freshness, parity freshness and Rust formatting passed. These focused checks supplement the earlier full suites; they are not a claim that a second full suite was run.

The parent updates parity to reflect partial workflows: live hardware input/output/settings, selected-clip audio editing, detector/slicing foundations and plugin ownership. No additional row is marked done merely because a foundation exists. There are 342 rows: 59 done, 34 in progress, 247 todo and two previously justified won't-do rows.

## Piano, transport and runtime follow-up

Selected-note transformations `666e1394` are integrated through `9f716d8a`.
They share one atomic Rust command across desktop and browser: quantize with
strength/original grooves, legato/staccato, grid chop, compatible glue, strum,
time/pitch flip, transpose/key-range limiting and velocity scaling. See
[selected-note tools](../PIANO-TOOLS.md) for limits and remaining phrasing work.
The combined bindings now contain 149 files, and the pinned browser WASM is
1,722,263 bytes. Parent checks passed 12 native piano tests and 130 UI tests
covering tools, piano-roll integration and exhaustive shared-WASM commands.

CLAP repairs `320fe342` are integrated through `c2b8eb3c`. Immediate releases
and panic have reserved bounded capacity, dialect-specific native event lists
are sized for expansion, accepted parameter updates wait for available space,
and retirement flushes ordered bounded chunks. Native fixture regressions cover
saturation, retained UI key ownership, final accepted state and zero callback
allocation. Parent processing/state/realtime checks passed 43 tests, followed
by all eight desktop MIDI tests. Strict all-target Clippy for project, host,
engine and desktop and workspace formatting passed. See
[CLAP saturation](../plugins/clap-saturation.md).

The transport now has a review-and-Apply tap-tempo dialog. Shared store guards
also discard edit/history/snapshot replies from a replaced project and avoid
snapshot rollback after a newer patch. The focused transport/tempo/store suite
passed 73 tests; UI lint and production TypeScript/Vite build passed. See
[tap tempo](../TAP-TEMPO.md). These checks supplement the earlier full suites.

Editing review round two closed the original WASM and tempo-automation
findings but reproduced a new inspector lifetime regression: publication of
Apply's own replacement patch can unmount its dialog before the IPC reply and
leave selection on the removed clip. A dedicated task is fixing stable
operation ownership and adding event-before-reply inspector regressions.
Piano rows remain in progress until follow-up review; no unreviewed tool is
marked fully complete here.

## Remaining work and evidence limits

The initial CLAP findings have implementations and passing fixture regressions.
Independent follow-up review and the inspector lifetime repair still need to
finish before this local batch is pushed as verified progress.

Production VST3 additions remain disabled. Dirty-state scheduling, events during capture and fallible native deactivation are being implemented separately. MIDI note recording/controller mapping/sequenced output, playable slice mapping, advanced audio-editor tools and independent sampler stretch remain unfinished. Physical MIDI/audio hardware, installed native UI and new macOS/Linux desktop workflows have not been verified by these synthetic tests. Hosted CI evidence for the earlier sampler/portability commit is recorded separately from these unpublished feature changes.
