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

## Remaining work and evidence limits

The runtime review identified CLAP release/panic loss under a full event queue and accepted parameter truncation during inactive flush. A separate T3 implementation task owns both fixes; this integration must not be pushed as fully verified until those findings are resolved and tested.

Production VST3 additions remain disabled. Dirty-state scheduling, events during capture and fallible native deactivation are being implemented separately. MIDI note recording/controller mapping/sequenced output, playable slice mapping, advanced audio-editor tools and independent sampler stretch remain unfinished. Physical MIDI/audio hardware, installed native UI and new macOS/Linux desktop workflows have not been verified by these synthetic tests. Hosted CI evidence for the earlier sampler/portability commit is recorded separately from these unpublished feature changes.
