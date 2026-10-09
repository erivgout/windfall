# Recording and mixer independent QA review — 2026-10-09

> Final execution reconciliation: the combined tree passed the complete native workspace, strict checks, fresh artifact checks and final affected frontend checks. See [completion acceptance](2026-10-09-completion.md) for exact results and promoted IDs. Earlier pending statements below describe intermediate states; explicit remaining feature gates still apply.

This is a read-only implementation/specification and regression-coverage review of the recording/mixer feature-pass additions. It does not claim execution of the native, frontend, browser, audio-device or plugin gates owned by other QA agents. No source or parity status was changed. No GitHub Actions or CI was used.

## Result

The implementation ledger accurately describes substantial new source coverage, but passing the existing broad suites alone would not verify these additions. There are no directly identifiable behavioral tests for the new recording clock/aligner/monitor, count-in/metronome engine, waveform accumulator, mixer preset transaction, saved take groups or synchronized comp commands in the inspected native test trees. The existing mixer component tests exercise earlier single-selection, fader/pan, ordinary routing and sends; the recording dialog suite contains only the earlier channel/error and browser-unavailable cases. This is an acceptance-evidence gap, not evidence that those implementations fail.

No independently reproduced runtime defect was established in this review. Source review inspected gate lifecycle and sinc conversion, count-in scheduling/rendering, grouped source validation/crossfade geometry, preset validation/ownership remapping, external-output design, waveform interest and epoch handling, and captured mixer movement. These paths require focused new tests before they can be described as QA verified.

## Exact parity recommendations

The following implemented rows are candidates for `done` **after** fresh artifacts, full checks and the feature-specific acceptance evidence below pass. They are not approved as done by this source-only review:

- `wf-metronome`: native rendered beat positions in song/pattern mode, changed meter/tempo, accent/gain, disabled click and exclusion from offline stems; frontend toggle/menu integration.
- `win-mixer-multi-select`: Ctrl/range selection in visual dock order; grouped relative gain/pan with clamps; bulk operations; native stable-ID movement, stale capture rejection and undo/redo.
- `win-mixer-docks-layouts`: save/reload all three docks; all eight sizes/adaptive; range focus/reveal across independently virtualized docks; resize and preference restoration. FLP docking import is explicitly unfinished, but is not part of this row's stated basic UI summary.
- `win-mixer-track-eq`, `win-mixer-track-utilities`: independent native processing behavior after slots, automation/persistence/history and playback/offline agreement. FLP parameter mapping remains separately unfinished.
- `win-mixer-pdc`: signed correction validation/persistence/history and rendered impulse timing across direct/output/send/printed/external routes, including negative clamping and plan adoption. Existing automatic-PDC tests do not establish manual correction acceptance.
- `win-mixer-waveform-view`: accumulator stereo extrema, ring ordering/duration, clipping, epoch/identity reset and no-capture behavior, plus native canvas integration and virtualized interest lifecycle. The browser mock deliberately publishes no audio waveform and cannot verify this row's waveform output.
- `win-mixer-render-tracks`: selected/armed entry-point capture, Master/Current handling, removed or replaced source rejection and existing offline output validation.
- `win-mixer-track-presets`: built-in capture/apply/save/reload and one-step undo/redo; fresh effect IDs, obsolete automation removal/restoration, routing preservation, validation/file/storage bounds and stale document/destination guards. Hosted-plugin preset state cannot be marked verified from built-in tests alone.

Rows requiring additional native/device evidence before full completion:

- `core-audio-recording`: actual ADC/DAC round-trip timing at unequal rates, drift and offset signs, initial prefix preservation, stop-tail drain, queue/device failure cleanup; microphone/line and internal source attachment. Missing broader logger workflows must be tracked separately rather than inferred implemented.
- `wf-count-in`: deterministic sample-level schedule/deadline tests can verify the logic; an actual native input capture must establish that the count-in gate begins at audible playback and stop/discard restores the prior region.
- `win-mixer-audio-io`: real device port enumeration/open-count selection, shared-input multitrack routing, standalone Listen/Stop and output replacement/copy/mono sum/unavailable pair behavior. Browser simulation proves neither hardware capture nor physical output assignments.
- `win-mixer-record-arm`: native dry/post-effects/post-fader rendered capture, latency gate/tail, multitrack atomic keep and Direct printed playback alignment; hardware input arms additionally need a device smoke check.
- `wf-loop-recording`: native retained complete/final-partial passes, tempo-map partition boundaries, all/latest/only/except selection and atomic keep/history; group creation/pruning/audition and synchronized comp geometry need command tests. A native recording smoke check remains needed for the capture-to-keep workflow.
- `win-mixer-sidechain`: do not promote. The ledger explicitly says desktop bridge-provider integration remains implementation work; native detector and hosted key delivery also need dedicated acceptance.

`win-mixer-tracks` is already done in parity, but its newly claimed 500-insert capacity and Current utility need refreshed acceptance before the existing done claim can be considered verified against the expanded specification. Exercise capacity/history/persistence, Current selection/processing/meters and playback/export exclusion, and a practical large-mixer performance check.

`core-mixer` remains an umbrella requiring the incomplete sidechain/plugin/device subscopes; do not close it simply because individual built-in processing candidates pass.

## Evidence inspected

- `docs/FEATURE-PASS.md` and `docs/parity/parity.json`; metronome/count-in, recording alignment/monitoring, multitrack/disk/loop/take/comp, external output, manual latency, selection/layout, preset and waveform feature documents.
- `crates/windfall-engine/src/{recording_clock,recording_timed,recording_monitor,metronome,waveform_meter}.rs`, count-in dispatch/render completion in `processor.rs`, and manual latency declaration in `state.rs`.
- `crates/windfall-project/src/{mixer_preset,take_groups,audio_comp}.rs` and native preset/move/take-group/comp lowering in `lower.rs`.
- `apps/desktop/src-tauri/src/session/{recording_takes,mixer_presets}.rs`; frontend `track-presets.tsx`, `waveform-meter.tsx`, `input-monitor-store.ts` and mixer visual-order selection helpers.
- Existing `crates/windfall-engine/tests`, `crates/windfall-project/tests`, desktop session tests, `mixer.test.tsx` and `recording-dialog.test.tsx` coverage inventory.

## Tracker reconciliation

The recording-related parity notes still say external output, standalone monitoring, printed bypass, comping and multitrack associations remain source work even though later FEATURE-PASS entries implement them. `win-mixer-pdc` still says no manual offset. These notes should be reconciled to “implementation present; acceptance pending” with links to the later documents. That is honest progress without treating implementation or a generic successful build as verified acceptance.

Full checks and refreshed artifact provenance must be appended by their execution owners. Do not copy the feature documents' older “no tests run” statements into a new completion claim once new evidence exists; record exact commands/results and the native/device scope actually exercised.

## Resumed deterministic acceptance work

The source-only review above is retained as the baseline. Ten focused native regression tests have now been added in `crates/windfall-engine/src/qa_recording_mixer.rs`, `crates/windfall-engine/tests/qa_mixer_runtime.rs`, and `crates/windfall-project/tests/qa_mixer_completion.rs`. The engine library includes the new unit file only under `cfg(test)`. These tests exercise existing implementation; no new recording or mixer feature was built.

Execution is coordinated through the native QA owner to avoid simultaneous Cargo builds and memory contention. The required focused commands are:

```powershell
cargo test -p windfall-engine --lib qa_
cargo test -p windfall-engine --test qa_mixer_runtime
cargo test -p windfall-project --test qa_mixer_completion
```

Results must be appended after execution; test creation alone is not a pass. Direct formatting of the three new files passed with `rustfmt --edition 2024 --config skip_children=true`. Initial recursive formatting discovered an already-dirty shared engine support parse defect: a stray `dock:` initializer inside `Rig::track_mut`; that defect was relayed to the native owner for repair.

The exact acceptance scopes exercised by these tests, once passing, are:

- `wf-metronome`: synthesized click gain, stereo output, accent disable and click tail disable; live song/pattern 3/8 beat positions and accents at 120 BPM; output identical for 64- and 127-frame blocks; offline project rendering contains no runtime click. Changed-tempo/meter segments, stems specifically, and frontend toggle/menu interaction still need acceptance.
- `wf-count-in`: fractional beat deadline rounding, saturating frame arithmetic, block-independent pre-roll audio, count-in clicks while ordinary metronome is disabled, held playhead until the final beat deadline, playback transition, and stop cancellation. This does not emulate a sound-card timestamp or native input capture, so audible capture-gate timing and stop/discard region restoration remain open.
- `win-mixer-waveform-view`: requested-interest gating, partial buckets spanning calls, independent stereo extrema, finite-value sanitization and clipping, 64-point oldest-first ring retention, serial publication, and owner/epoch/rate/frame-discontinuity reset. Native canvas drawing and virtualized interest lifecycle remain open; browser mock silence is not waveform acceptance.
- `win-mixer-track-presets`: built-in capture and JSON encoding, validation, apply with fresh effect IDs, destination route/recording/dock preservation, project save/reload, one-step undo/redo, stale destination rejection, and atomic invalid-preset rejection. Obsolete automation restoration, plugin target/state remapping, storage/file bounds and stale document guards remain open.
- `win-mixer-pdc`: signed manual offset boundaries, rejection of non-finite/out-of-range values, document persistence and undo/redo. These are project transaction tests, not impulse timing tests; direct/output/send/printed/external render timing and plan adoption remain open.

These ten native tests provide inspectable evidence for the named implemented scopes. Native execution and refreshed full-check results remain required before promotion.

### Mounted waveform frontend acceptance

`apps/desktop/src/features/mixer/waveform-meter.qa.test.tsx` adds two focused mounted tests. The first mounts the real mixer toolbar and `LevelSection`, toggles the user's Levels/Waveforms control, verifies accessible pressed state and actual component replacement, and confirms guarded native interest registration and clearing. The second supplies a native-shaped waveform frame to the real `StripWaveform` canvas, asserts independent stereo extrema drawing coordinates and display clipping, verifies unchanged-serial draw suppression and missing-frame clearing, and verifies interest/subscription release on inactive state and unmount. It uses injected frame data; it does not claim the browser mock produces audio waveforms.

Executed commands and outcomes:

- `pnpm exec vitest run src/features/mixer/waveform-meter.qa.test.tsx --maxWorkers=1` — **PASS**, 1 file, 2 tests (6.87 seconds).
- `pnpm exec prettier --write src/features/mixer/waveform-meter.qa.test.tsx` — **PASS**.
- `pnpm exec eslint src/features/mixer/waveform-meter.qa.test.tsx` — **PASS**.
- Final typed fixture rerun: the native waveform epoch/serial values use their generated `bigint` types. `pnpm exec vitest run src/features/mixer/waveform-meter.qa.test.tsx --maxWorkers=1` — **PASS**, 2 tests (11.29 seconds); `pnpm typecheck` — **PASS**, exit 0.

Together with passing native accumulator/runtime tests and final frontend/native gates, `win-mixer-waveform-view` can qualify as software QA verified against its basic parity summary, “Swaps level meters for scrolling waveforms.” This row does not require physical device output. `wf-metronome` can likewise qualify for its implemented software scope after passing actual Processor scheduling/click tests and final gates. Advanced tempo/meter-segment cases remain additional regression depth; do not infer those individually executed from the fixed 120 BPM / 3/8 tests. Recording capture, manual PDC route timing, and hosted-plugin presets remain separately gated as described above.

### Recording completion migration repair

The resumed desktop compile exposed an incomplete recording migration to prepared-project publication. The finish path retained its recording mutex while joining capture and preparing native audio; loop and multitrack paths still called the older one-argument commit API. The owned recording files now transfer the take out of the mutex under a RAII finishing exclusion, prepare and retire outside recording/State guards, and reacquire short completion guards for ticket capture and atomic publication. The native owner added the matching finishing/cancellation state and single-import guard checks. Cancellation marks the in-flight completion cancelled while retaining exclusion until its native resources and temporary files have retired. Per-take installation receipts preserve published audio even if cleanup follows a post-install failure.

`apps/desktop/src-tauri/src/session/recording_completion_qa.rs` adds two deterministic regressions, each exercised for single and loop captures: cancellation paused after preparation verifies mutex release, rejection of replacement/edit admission, refusal to publish, temporary-file cleanup and RAII flag reset; successful completion verifies retained source audio, released exclusion and undo/redo access. These tests are registered by the native owner. Direct Rust formatting passed; desktop Cargo execution remains owned by native QA and must be recorded before calling these repairs verified.

The project preset regression's undo comparison also accounts for the document allocator's intentional monotonic `next_id`: undo restores musical state while reserving IDs already allocated by the preset transaction.

Independent review caught and repaired an additional active-discard race: removing a take from the mutex before off-lock `Take::drop` could admit a new recording while the old owner stopped shared taps, monitoring and transport. Active cancellation now installs the same finishing RAII exclusion before detaching the take and clears it only after cleanup. A third deterministic regression pauses at the cleanup boundary and verifies the recording mutex is free while new recording, project replacement and engine reconfiguration remain refused; after release it verifies owned-file cleanup, exclusion reset and successful restart. Native execution remains pending with the native owner.

## Additional channel arpeggiator/polyphony acceptance

The parity summaries are `win-rack-arpeggiator`: “A real-time, non-destructive arpeggio applied to the notes a channel receives”; and `win-rack-polyphony`: “Caps the number of voices a channel plays and sets its glide time.” The implementation seam is `docs/integration/seams/channel-voice.md`. Browser simulated audio does not emulate these processors; frontend panel callback tests establish control editing, not audible arpeggio or glide.

Six new native regressions are registered automatically as integration-test files:

```powershell
cargo test -p windfall-engine --test channel_arp_polyphony_qa
cargo test -p windfall-project --test channel_arp_polyphony_qa
```

Native QA owns execution; direct `rustfmt --edition 2024 --config skip_children=true` passed for both files. Do not label these tests passed until their Cargo results are available.

- The engine target contains five actual `Processor` audio tests: Up/Down/UpDown/AsPlayed chord order with distinct audible velocity identification and unchanged source notes; live AsPlayed key ordering and release preventing further steps; zero gate silence and sustained-sample audible gate gaps; octave expansion measured by doubled playback slope on a linear sample; voice caps 1/2/3 measured in both settled audible survivor level and native voice counts; mono transfer retaining sample position; and 10 ms portamento measured from changing audible sample slope against zero-glide output. Multiple callback partitions must produce identical audio.
- The project target contains one history/persistence test covering all five modes, all four note divisions, gate 0/0.25/0.5/0.75/1, octave ranges 1–4, voice caps 1/2/16/32, mono on/off, and glide 0/10/1000/60000 ms. It verifies exact saved control values, undo/redo and unchanged source-pattern notes.
- Existing `crates/windfall-engine/src/channel_voice.rs` unit tests cover fixed scheduler order/range, mode endpoints, fractional continuity, mono decisions, limit reduction and glide coefficient semantics. Existing project `tests/channel_voice.rs` covers defaults, sanitization, command history and file save/load. Existing frontend `voice/voice-panel.test.tsx` covers Strict Mode complete draft callbacks and accumulated edits, mono control, pitch-envelope keyboard edits and settings sanitization. Final suite results are owned by their execution agents.

After these native tests and final full checks pass, the two basic software rows are completion candidates. Hosted-instrument pitch follows provider support, octave-equivalent chord metadata remains ambiguous when input tones share velocity, and the one-frame event routing path still needs performance acceptance for extreme workloads. No physical MIDI/audio device claim is inferred from the headless Processor evidence.

## Offline selected/armed mixer rendering acceptance

`win-mixer-render-tracks` summarizes “Renders the armed or selected mixer tracks to audio files offline.” `docs/MIXER-RENDER-TRACKS.md` specifies the captured selected/armed request, Master-as-mix, Current exclusion, stale document/track guards, and the existing native streamed stem/file renderer. Physical device capture is not an acceptance gate for this offline row.

The new frontend file `apps/desktop/src/features/export/mixer-render.qa.test.tsx` mounts the real toolbar and export dialog. Five tests verify exact selected/armed `ExportStems` requests, Master mix inclusion, Current checkbox/request exclusion, one-shot request consumption, unchanged saved arms, refused removed IDs, refused replacement with reused IDs, and surfaced backend admission errors. Its mocked export receiver inspects the submitted request; it does not claim to encode or synthesize audio.

- `pnpm exec vitest run src/features/export/mixer-render.qa.test.tsx --maxWorkers=1` — **PASS**, 5 tests (8.61 seconds).
- `pnpm exec prettier --write src/features/export/mixer-render.qa.test.tsx` — **PASS**.
- `pnpm exec eslint src/features/export/mixer-render.qa.test.tsx` — **PASS**.
- `pnpm typecheck` — **PASS**, exit 0.

`apps/desktop/src-tauri/src/session/mixer_render_qa.rs` adds three native file-output acceptance tests and is registered by the native owner. The selected request writes only the requested post-fader WAV, with the other source's time window silent. Saved arms produce two distinct stems and the mix in mixer order; both track-output and source-through-Master modes must conserve the simple linear mix sample by sample, preserve equal frame lengths, and leave source clips/arms unchanged. Removed IDs and Current are refused before creating output folders. Sources are deterministic stereo WAV files imported through the actual native session pipeline, and written outputs are decoded for PCM checks. Native QA owns `cargo test -p windfall-desktop --lib mixer_render_qa` and the final full workspace run; results remain pending until recorded.

After passing these native output tests and final execution gates, this row is a full basic-software completion candidate. Wider renderer modes, nonlinear/sidechain policy, formats, long streaming and cancellation already have existing native renderer/export tests; their current full-suite results must be cited rather than inferred from these new linear-source fixtures. This work changes only new test files and this QA report.

### First native execution results and recording test correction

`docs/qa/2026-10-09-native-tests-verified.log` records **PASS** for all three `session::tests::mixer_render_qa` native file-output regressions. The same run records **PASS** for the single/loop prepared-cancellation and successful-source-preservation recording regressions. The broader desktop target failed on separate plugin environment/readiness failures and one faulty assertion in the active-discard regression, so this is focused evidence within an overall failed first run, not a full native gate pass.

The active-discard failure at the original line 149 compared directory entries, not capture destruction: an unfinalized WAV intentionally owns both its reserved final `.wav` path and the writer's transactional hidden `.tmp` path. Expecting one entry while cleanup was paused was wrong. The regression now snapshots both exact owned paths, verifies they remain unchanged at the pause and disappear after cleanup, and independently counts actual capture destruction with a per-capture atomic counter: zero at the pause, one after join, still one after a subsequent recording lifecycle. No production ownership change was required for that assertion failure. The corrected regression is formatted and awaiting the native owner's rerun.

### Broad native run: channel QA fixture and playback repairs

The broad runtime run (`2026-10-09-native-tests-all-runtime.log`) found two new channel tests and four pre-existing instrument/sampler expectations failing. Final chord notes are now sorted after fixture key edits: `Rig.note` sorts before returning a mutable note, so the previous fixture actually sequenced keys 60,67,64. Audio mode/voice-limit assertions remain unchanged.

Two actual regressions were repaired: transport navigation had retired live direct owners along with sequenced owners; ordinary sampler key-up used forced eviction and faded envelope-free one-shots despite the Controller contract. Navigation now flushes sequenced/generated scheduling while retaining live direct ownership and live chord input. Ordinary key-up releases the envelope without forcing an envelope-free sample to stop; cap eviction and generated gates still force retirement. An added public Processor test verifies live sampler continuity across seek and its subsequent envelope release for four callback partitions. Existing instrument seek and one-shot sampler tests provide additional regressions.

The old synth pan assertion and architecture prose predated `NOTE-EXPRESSION.md`, which explicitly adds per-note synth pan. The test now proves unchanged left-channel PCM and silent right-channel PCM for full-left pan, alongside velocity behavior. Global 256-voice pool stress fixtures now span channels with distinct held keys so the new per-channel cap32 does not invalidate their global pool/fade assertions. These repairs await the native owner's fresh focused/full execution; no new passing claim is made here.

The final normal workspace run (`2026-10-09-native-tests-accepted.log`) passed all six public channel arpeggiator/polyphony tests, including the new live-sampler seek/key-up regression. Instrument velocity/pan and seek, sampler one-shot live key-up, and global pool overflow also passed. One global-pool fade smoothness assertion still failed: the newly distinct low keys introduce fractional sample rates, so the constant one-shot's leading boundary is cubically interpolated against zero before the sample. The fixture now uses a constant forward loop to isolate retirement gain from that source-edge transient; original voice counts, absolute PCM levels and fade-step bound are retained, with the actual step/bound printed on failure. This final test-only adjustment awaits the native owner's focused rerun.
