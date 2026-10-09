# Completion acceptance — 2026-10-09

**46 newly accepted features are verified by QA and marked `done`.**
The tracker now contains **136 done, 104 in progress, 100 todo and 2 exclusions**
across 342 rows. This completes the QA reconciliation of the implemented
software candidates; it does not declare the entire project finished.

## Accepted local gates

| Gate | Result | Evidence |
| --- | --- | --- |
| Complete frontend baseline | 420 files, 5,430 tests passed | [Frontend report](2026-10-09-frontend.md) |
| Post-generation command, curve and voice checks | 116 tests passed | [Frontend report](2026-10-09-frontend.md) |
| Final affected frontend/runtime suite | 56 files, 673 tests passed on repaired simulator and final mixer sources | [Final log](2026-10-09-frontend-repaired-artifact-verified.log) |
| Final frontend typecheck, lint and production build | All exit 0; lint has no warnings/errors | [Frontend report](2026-10-09-frontend.md) |
| Full native workspace, `cargo test --workspace -j2` | Exit 0: 2,973 passed, zero failed, 27 ignored across 117 result summaries | [Final runtime log](2026-10-09-native-tests-final-gate.log) |
| Strict native workspace/all-target Clippy, warnings denied | Exit 0 | [Final strict log](2026-10-09-native-clippy-complete.log) |
| Opted-in real native plugin/capture checks | 6 host and 4 desktop cases passed using the exact fresh workspace test executables | [Host log](2026-10-09-native-host-opted.log), [Capture log](2026-10-09-native-capture-opted.log) |
| Fresh generated binding comparison | 367 bindings and fixtures match, exit 0 | [Final artifact log](2026-10-09-native-bindings-complete.log) |
| Simulator generation and source freshness | Passed; final 3,748,949-byte artifact accepted by the affected frontend suite | [Native report](2026-10-09-native.md) |
| Live browser | Group labels, three docks, unequal scrollbars, resize alignment, real hot replacement, command execution and undo accepted | [Browser report](2026-10-09-browser.md) |
| Tracker generation and `node scripts/parity.mjs --check` | Passed; 342 rows and current PARITY.md | [Generated tracker](../parity/PARITY.md) |

Ten of the normal run's 27 ignored cases were executed separately and passed.
The remaining seventeen are explicitly enumerated in the [native report](2026-10-09-native.md):
opt-in CPU/resource/quality measurements, external codec checks, listening
artifacts and a subprocess authentication role exercised by its parent test.
They are not silently counted as independent normal passes.

The complete frontend baseline used SHA-256
`16540f31f499a6e804eb68986bd70b4839478e8b6d311db92ada7a3a9b81d205`.
The MIDI-capacity repair produced the final simulator, SHA-256
`b379d6bc82d6b3d9882adbcc4b543f3393a09722623ea8cd12c3599e961aaa1c`,
source-input hash
`fa1067cf531abbbc512e0bfb292ccc5c4e4b250bcda1141abc1e147f67cc0989`.
The full 420-file suite was not repeated on the new artifact: the final
56-file supplement covers its changed import/document/runtime behavior and
the final mixer changes. All final static checks were repeated and passed.

This pass tests the shared dirty working tree based on
`be782b3c86058006c96c3374cc86f757b88c91a2`. The implementation and repairs
are uncommitted; that base commit alone is not the tested source snapshot.
Checks ran locally on Windows with the documented lean profiles and native
fixture/helper paths. No GitHub CI or Actions were used.

## Exact accepted feature IDs

Each row below is now `done` and has dated QA scope/evidence in
[parity.json](../parity/parity.json). Related features with wider unfinished
contracts remain separate.

| Exact IDs | Accepted behavior and evidence |
| --- | --- |
| `win-piano-paint`, `win-piano-mute`, `win-piano-slice`, `win-piano-zoom`, `win-piano-playback`, `win-piano-ghost-notes` | Real-WASM piano gestures, drum paint, retained mute, note slicing, region zoom, audition release and editable ghosts; history, cancellation and expression conservation verified. [Evidence](2026-10-09-rack-piano.md). |
| `win-rack-send-to-piano-roll`, `win-rack-channel-groups`, `win-rack-graph-editor`, `win-rack-fill-tools` | Shared canonical step/piano lane, saved named group filtering, step-property editing and rhythm-rule fill verified through mounted controls and project transactions. [Evidence](2026-10-09-workflow.md). |
| `win-rack-swing`, `win-rack-note-timing` | Saved swing mix, signed note offset and truncation verified through mounted controls and exact native/MIDI onset measurements. [Evidence](2026-10-09-channel-timing.md). |
| `wf-metronome` | Public native runtime verifies audible beat clicks; physical recording and count-in remain separate open rows. [Evidence](2026-10-09-recording-mixer.md). |
| `inst-fruity-dx10`, `inst-plucked`, `inst-sakura`, `inst-fruity-kick`, `inst-simsynth`, `inst-transistor-bass` | Mapped instrument family verified through meaningful parameter/audio differences, persistence/history and public live/offline PCM comparisons. [Evidence](2026-10-09-dsp-plugins.md). |
| `fx-fruity-delay-2`, `fx-fruity-delay-3` | Stereo/filter/modulated echo contracts verified through measured DSP/public runtime, saved parameters and mounted editor controls including modulation rate/depth. [Evidence](2026-10-09-dsp-plugins.md). |
| `wf-macros` | Verified one-click utilities: reset levels, switch every clip's stretch mode, mute empty tracks and unsolo; property/history/save conservation verified. Unused-channel selection and unused-clip purge are not claimed. [Evidence](2026-10-09-workflow.md). |
| `win-playlist-slip`, `win-playlist-slice`, `win-playlist-playback`, `win-playlist-tracks`, `win-playlist-track-groups`, `win-playlist-clip-groups`, `win-playlist-audio-clip-fades`, `win-playlist-audio-clip-properties` | Playlist gestures, named/resized/muted/soloed lanes and groups, fades/crossfades and per-instance audio properties verified with real document/history checks and complementary native PCM. Playback tool verifies seek/scrub. [Evidence](2026-10-09-playlist-workflow.md). |
| `wf-step-edit`, `wf-project-info`, `core-chord-generator` | Stopped keyboard step entry, saved project metadata/export author tags and key/mood chord insertion verified through mounted document controls and complementary native persistence/export checks. [Evidence](2026-10-09-workflow.md). |
| `win-mixer-docks-layouts`, `win-mixer-multi-select` | All eight saved layouts, three docks, grouped adjustments/routing/movement, stale refusal, keyboard ranges and independent scrolling verified; actual browser resize/scrollbar geometry accepted. [Evidence](2026-10-09-mixer-ui.md). |
| `fx-fruity-notebook-2` | Plain multi-page notebook editing, history, limits, replacement and native disk roundtrip verified. Playback-following Notebook1 remains separate. [Evidence](2026-10-09-notebook.md). |
| `win-rack-arpeggiator`, `win-rack-polyphony` | Saved non-destructive channel arpeggiation, voice caps and glide verified through mounted controls and public audible scheduling/admission tests. [Evidence](2026-10-09-recording-mixer.md). |
| `win-piano-randomize`, `win-piano-time-markers`, `core-time-signature-changes` | Chord-map note/expression randomization and independent song/pattern marker/meter maps verified through mounted controls, atomic history/save and actual native/MIDI timing/export. [Evidence](2026-10-09-piano-contracts.md). |
| `wf-typing-keyboard`, `win-browser-backups` | Computer-keyboard audition/release and dated backup listing/open workflows verified through mounted UI and complementary public native audio or actual filesystem acceptance. [Evidence](2026-10-09-completion-audit.md). |
| `win-mixer-track-eq`, `win-mixer-track-utilities`, `win-mixer-render-tracks` | Post-slot track EQ/polarity/swap/width verified with measured native PCM and mounted controls; selected/armed offline rendering verifies captured requests, real files and actual stems. [Evidence](2026-10-09-workflow.md). |

## Repairs required before acceptance

The reports retain initial failures and their final disposition. Repairs
include missing preparation/analysis/plugin wiring and command registration;
source namespace admission; recording completion/cancellation ownership;
native plugin description and selected-key forwarding; failed-device
recovery and conditional routing adoption; live-note preservation and
envelope-free one-shot release; MIDI import capacity; the generated LFO
name collision; missing delay modulation controls; readable group labels;
mixer scrollbar alignment and development hot-reload cleanup.

Stale fixtures were reconciled with current meter/marker, Current-strip,
effect-registry, latency, voice-cap and serialization contracts. Deterministic
test barriers and asynchronous menu readiness were restored. Exact audio,
history, refusal, ownership and allocation assertions were retained. See
the [evidence index](README.md) and [independent completion audit](2026-10-09-completion-audit.md)
for domain findings.

## Tracker reconciliation and remaining gates

Before these promotions, eight scope corrections and 85 moves from `todo`
to `in-progress` reconciled actual implemented subsets with the tracker.
Those moves were not QA completion claims. The final 46 promotions are
separate: five previously `todo` rows and 41 previously `in-progress`
rows are now accepted. Existing 90 done rows remain and their enabled
regression suites passed.

The 104 in-progress and 100 todo rows retain their unfinished scope. In particular:

- Physical audio/MIDI capture, routing and device timing, count-in/loop-recording,
  and wider platform/commercial-plugin acceptance remain open.
- Integrated native analyzer source selection, subscriptions and actual view
  delivery remain explicit gates for all five T8 analyzer/view rows.
- Independent alternative arrangement placements, track-scoped consolidation
  and independent automation Make Unique remain unfinished.
- EchoBank's linked eight-unit editor and FrequencyDelay's drawn sixteen-band
  editor, fallible host admission and saturating aggregate tails remain open.
  Their actual registered eight/sixteen-unit software subsets are verified.
- Sidechain's real multiple/wider auxiliary ports, saved selection across
  native-owner exchange and dynamic detector/PDC/history transitions remain
  open despite the passing engine and mono-aux CLAP/VST3 cases.
- Full hosted preset workflows, Notebook1 playback-following pages, production
  analysis inference and the other scoped instrument/controller/editor gaps
  remain documented in the domain reports.

The authoritative completion accounting is the generated tracker, not source
presence, a compiler pass, or an authored-but-unexecuted test.
