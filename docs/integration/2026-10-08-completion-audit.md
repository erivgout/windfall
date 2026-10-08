# Integrated feature completion audit

Audited on 2026-10-08 against root `d2b2c6358509274879ed91660c55a11cddbd8064`,
the exact summaries in `docs/parity/parity.json`, accepted independent reviews,
composed native/shared-WASM checks and CI source `427ed0a7`.

The tracker retained eighteen implemented features as in progress after their
repairs, reviews and integration had finished. This audit closes those feature
rows. It adds no implementation, removes no feature requirement and changes no
umbrella, release, platform or hardware acceptance gate.

The roadmap's accounting rule is that a row closes when its summary's visible
behavior, relevant persistence/history, realtime safety, offline parity and
failure handling work. It separately says not to force synthetic and external
verification into the same Boolean. The remaining device, native-window and
all-platform checks below therefore remain explicit project gates, rather than
an indefinitely pending review label on every implemented software feature.

| Rows accepted                                                                                                                                                       | Delivered behavior and evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `fx-fruity-balance`, `fx-fruity-center`, `fx-fruity-mute-2`, `fx-fruity-phase-inverter`, `fx-fruity-stereo-shaper`, `fx-fruity-soft-clipper`, `fx-fruity-fast-dist` | Seven distinct processors and controls, rather than aliases. Checked commands, gestures/history, persistence, automation, latency/readiness/tails, signal/reference measurements, callback guards and live/offline/stem checks. Utility R7 is clean; composed root verification passed 112 unique native cases. Existing changing-latency automation and varying-branch transient-cancellation limits remain documented. See [UTILITY-EFFECTS.md](../UTILITY-EFFECTS.md), [UTILITY-REPAIRS.md](../UTILITY-REPAIRS.md) and the composed-utility section in [the earlier ledger](2026-10-07-native-editing-midi.md). |
| `fx-fruity-fast-lp`, `fx-fruity-free-filter`, `fx-fruity-bass-boost`                                                                                                | Actual lowpass, seven selectable filter modes and two-control bass shelf; all controls persist and automate through the generic UI. Independent transfers, stereo/partition/RT tests, live/offline/both stem modes and every serial restoration frame are checked. Root DSP 299, project 4, engine 6 and focused shared-WASM UI 70 passed. Both downstream R4 axes are clean; Spec independently ran the 4 project/6 engine cases. See [FILTER-FAMILY.md](../FILTER-FAMILY.md) and [R4 reports](2026-10-08-review-rounds.md).                                                                                      |
| `win-rack-piano-roll-preview`                                                                                                                                       | Authoritative timing/duration/pitch thumbnail in the step area; bounded dense geometry; explicit steps/notes; canonical menu, shortcut and context actions; exact guarded piano-lane navigation. Independent R2 is clean, original step assertions remain active, and root composition/async-menu compatibility tests pass. Browser checks cover zoom, scroll and focus without musical/history mutation. See [RACK-NOTE-PREVIEW.md](../RACK-NOTE-PREVIEW.md).                                                                                                                                                     |
| `win-playlist-zoom`, `win-playlist-time-markers`, `win-playlist-selection-loop`                                                                                     | Dragged-region zoom; persisted named/loop/skip/pause markers; selected playback/loop/export. Real engine navigation, canonical native generation/revision/request guards, cancellation/reload recovery, WAV content/duration, PDC/tails, linear buffered/stream/stem parity and unchanged destination/staging refusal tests pass. Source review repairs are accepted; matching shared-WASM UI and fresh full UI CI pass. See [TIMELINE-REGIONS.md](../TIMELINE-REGIONS.md) and the [review ledger](2026-10-08-review-rounds.md).                                                                                   |
| `win-browser-search`, `win-browser-tags`, `win-browser-starred`                                                                                                     | Recursive wildcard/Boolean queries, persistent normalized tags and Starred filtered results. Bounded/cancelled index, metadata refusal, late-result/root/source replacement and transactional import checks. Browser R4/sampler R3 composed review is clean. Parent composition passed 84 native and 268 UI cases, preserving exact source/version/history behavior. See [BROWSER-LIBRARY.md](../BROWSER-LIBRARY.md), [BROWSER-REPAIRS.md](../BROWSER-REPAIRS.md) and [the integration ledger](2026-10-07-native-editing-midi.md).                                                                                 |
| `win-piano-stamp`                                                                                                                                                   | One-click menu chords/scales with real preview, atomic placement/history and bounded stale-lane/project ownership. Delayed close, right-click, rejected capture, blur/Escape/tool/history repairs are integrated. Independent R4 passed 174 focused tests and browser schedules; root passed 171 stamp/history cases. See [PIANO-SCALES.md](../PIANO-SCALES.md), [PIANO-STAMP-REPAIRS.md](../PIANO-STAMP-REPAIRS.md) and [the integration ledger](2026-10-07-native-editing-midi.md).                                                                                                                              |

The regenerated tracker is **86 done, 42 in progress, 212 todo and 2 excluded**:
**88/342 accounted for, 25.7%**. Completed features alone are **25.1%**. This is
feature coverage, not an estimate of elapsed or remaining engineering time.

## Current verification and retained gaps

CI [37770742963](https://github.com/erivgout/windfall/actions/runs/37770742963)
at `427ed0a7` is terminal. UI passed **172 files / 2,662 tests**, typecheck,
lint and build; Ubuntu passed workspace formatting, strict Clippy and tests.
Simulator, parity and all three binding jobs passed. This is supporting composed
software evidence, not a claim that the entire CI run passed.

Windows repeated the separate bridge post-reset capture failure: **26 passed,
one failed, one intentional subprocess-role ignore**, 6.79 seconds. The failed
second capture reports `capture timeline ownership changed`; the unchanged
epoch-2/no-new-DSP oracle is retained. Log:
`C:/Temp/windfall-427-windows-ci.log`, SHA-256
`a426ee5da407973a569ed7b6f6a6c2c6e373d0f1f774266fe41f41150850c79a`.
macOS repeats **12 passed / 8 failed** Unsupported VST3 bundle-entry cases.
App jobs were skipped. Both actual failures have implementation owners and
remain open; no platform or release row is closed by this audit.

The overall project still needs genuine off-lock native preparation/retirement
and Session publication, production bridge activation, macOS bundle hosting,
per-pattern signatures, the remaining timeline/arrangement tools, specialist
effects/instruments, analysis algorithms and app workflows, packaged installation
and update acceptance, and physical audio/native-window/all-platform verification.
Core meter, plugin-hosting, recording, release and other incomplete rows remain
unchanged. In particular, pure DSP checkpoints do not close E3/E4 or M3.

## Delivery workflow correction

The earlier process produced too many narrow reservations and repeated handoffs.
Feature owners now own complete implementation, integration and meaningful
verification in their bound branches, including necessary mechanical consumer
and test changes. Lo-fi, P1 Session publication and the macOS loader have this
scope; the reset owner has one coherent ABI4 repair scope. Immutable review pins
remain available, and actual defects are repaired while independent work
continues. Parent composition and one final Standards/Spec pair establish the
delivery; extra review rounds require a concrete unresolved finding.

Private repository access and `v0.1.0-alpha.1` are preserved. This audit creates
no release, tag, public publication, dependency upgrade or new product exclusion.

## Subsequent integrated deliveries

The parent implemented the resident Host RAM toolbar readout at `0e81f508`,
corrected its accessible accounting-scope label at `7f180337`, and accepted the
clean Standards and bounded Spec follow-up. Native Windows and remote
macOS/Linux query checks, polling/lifecycle/component checks and the actual
browser layout support `wf-toolbar-meters`. The readout explicitly excludes
webviews and plugin helpers; it is not aggregate process-tree memory.

The complete Lo-fi reduction source `d073d6e3` + `be6c9811` is composed at
`9c565d8b` + `13822c08`. Both independent review axes report zero findings.
Fresh parent checks cover all DSP targets, project persistence/history and
automation, live/offline/both stem modes and actual shared-WASM controls.
Its 16 controls and distinct quantization, rate reduction, drive, timed
replacement and resonant pre/post filtering satisfy `fx-fruity-squeeze`.
Finite activity, intentional aliasing, device/listening and platform limits
remain documented in [LOFI.md](../LOFI.md).

The tracker is now **88 done, 41 in progress, 211 todo and 2 excluded**:
**90/342 accounted for, 26.3%**; completed features alone are **25.7%**.
This adds two delivered features to the earlier eighteen-row accounting audit.
At that checkpoint, Chorus/Flanger/Phaser were still awaiting the concrete
Phaser tail-reentry repair and parent composition. The later delivery below
records their accepted integration.

The independently reviewed ABI4 reset increment is composed at `a0e71327`.
Parent local execution includes the original epoch-2/no-new-DSP capture oracle,
82 host library cases, 31 real helper-process cases and 20 realtime cases.
This repairs a concrete host failure; it does not close production routing,
off-lock preparation, native editor, packaged discovery or the overall N4 row.

The user explicitly prohibited further GitHub Actions minutes. Automatic CI
is disabled remotely; no workflow is dispatched or rerun for this batch.
Verification continues locally. The last already-completed remote run remains
separate provenance, and unexecuted platform/device gates remain open.


## Integrated modulation family

Chorus, Flanger and Phaser are now composed at `ce702032` + `b271bc43` with
Lo-fi's original index 15 retained and the new kinds appended at 16..18. Independent
Standards and the corrected Spec round have no actionable findings. Actual
parent DSP, project/history/persistence/automation, real engine live/offline/
both stems and allocation guards pass. The 15-file, 344-test UI run uses freshly
generated 189-file bindings and the matching2,021,700-byte Rust WASM, covering
all 22 controls, generic effect operations and the187-range automation inventory.
The Phaser tail reentry repair preserves its declared numerical bound.

The three basic modulation rows are done. The latest tracker is **91 done,
41 in progress, 208 todo and 2 excluded**: **93/342 accounted for (27.2%)**,
with completed features alone at **26.6%**. Specialist/vintage variants remain
separate rows. Headless signal/throughput tests do not claim device/listening
or all-platform qualification.

The monophonic pitch kernel and its reviewed CMND correction are also integrated
and17 parent numerical cases pass. The pitch editor, realtime correction/harmony
and production app registration remain open; this foundation closes no such row.
The compact capture-acknowledgement contract is composed without activating
production 19f routing, and the parent reproduced/fixed its scanner-error loss.
Actual native bookkeeping tests pass; P1 Session readiness, the parent warning
follow-up review and other production/platform gates remain separate.

No Actions minutes are used for this delivery. CI remains disabled and the
private push carries `[skip ci]`. The roadmap inventory still records its
original planning base; current status comes from the validated parity source.
