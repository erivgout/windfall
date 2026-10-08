# Native runtime repairs

These changes address the three persistence, bundle validation and VST3 release findings against `7476b074ffaff593e9a9f392665d1a1e8067a1c9`. The isolated implementation branch includes parent integration `73149138c6158b0dc5e73604cdf19abaed801006` through merge `307837a6`; archive and export paths use those integrated implementations. Production VST3 availability remains enabled with independent path, format, plugin-ID and instrument/effect-role checks.

## Capture ordering

Parameter admission is not proof of native application. A document edit can also precede the callback that consumes its plan. Every writable binding parameter therefore has preallocated atomic metadata for the latest callback value, its generation, the committed document value/generation, and the generations observed and applied by audio. Control publishes committed values before queuing a plan. Callback publication does one bounded read of document metadata; it never waits for that writer. Only a nonempty successful native process with no outstanding rejected value acknowledges delivery. Owner snapshots can retry reads off audio. The control registry mutex is never accessed by audio or held around a native call.

The ordering is:

1. An outstanding committed value wins over older native readback until it has been observed and applied, including when no callback has yet observed the edit.
2. Later callback automation wins once the committed value has been observed and processed. Save does not continually force native state back to the stored document value.
3. VST3 capture reads pending intents after the ownership boundary, reconciles them while inactive, and serializes afterward. Successful native deactivation records changed controller values; those newer lifecycle edits supersede older returned control points and adapter reconciliation. Regular explicit inactive parameter edits remain valid.
4. CLAP preserves active capture. Outstanding intent accompanies opaque state as project parameters. Creation loads opaque state first and then applies writable project parameters, so reopen and offline export restore the intended final value even if opaque state still predates a saturated queue. Capture does not inject duplicate active controls into the owner queue.
5. Timeout, refusal, save failure, cancellation and recovery errors do not acknowledge pending generations. A parameter snapshot conflicting with a newer published document value is rejected. Deferred dirty capture checks the session parameters again before applying, and acknowledges its dirty serial only after the document accepts the edit.

Native VST3 inactive-state requirements, exact returned ownership on refusal, cancellation/join lifetimes, recording exclusion and playback/offline token/revision/binding guards remain enforced. Callback storage does not grow. The existing engine note reconstruction, hardware ownership/epoch handling, CLAP reserved releases and ordered inactive flush chunks are retained.

## Scanned binary identity

`paths::plugin_file_identity` uses the same `vst3_binary` resolver as native loading. Its identity contains the canonical resolved binary path, size and exact modification time. Missing nested binaries cannot fall back to stamping a directory. The scanner stores that identity with its result, cache freshness compares it, runtime approval copies the stored identity, and creation compares it to current identity before loading. Approval does not refresh an old scan's stamp to a changed file. Cache schema 2 intentionally rescans schema 1 entries.

The real fixture is packaged as `Bundle.vst3/Contents/x86_64-win/Bundle.vst3`. Appending a valid PE overlay changes only the inner file: the outer directory size and modification time remain equal. The old runtime approval rejects it; an actual scanner rescan permits it; removing the inner file rejects it. Host coverage also saves/loads the catalog, verifies that the old scanned identity remains unchanged until refresh, and removes missing binaries from the catalog. Single-file fixtures still exercise production identity and role guards.

This is metadata validation, not a content hash or a protection against concurrent replacement after the check.

## VST3 translated event bound

The input API admits up to 1024 ordinary events plus 129 reserved immediate releases. A panic can expand into releases for 16 channels × 128 keys. At most 2048 initially held keys need release once; every admitted ordinary note-on can contribute its own event and one later panic release; every explicit reserved release can contribute one event. Repeated panics cannot release a held key again without an intervening note-on. Reset's initial releases consume the same initial-held allowance. Thus

```text
2048 + 2 * EVENT_CAPACITY + IMMEDIATE_RELEASE_CAPACITY = 4225
```

is a conservative preallocated native event capacity. CLAP dialect-specific capacities and admission limits are unchanged.

The VST3 instrument fixture now tracks held channel/key pairs and stays audible if any survive a release storm. The regression warms all 2048 pairs, queues an ordinary global panic followed by 1023 distinct note-ons, then an adapter panic. Its 4094 native events fit the new bound. It checks zero dropped events, silence after that block and another panic, and zero host allocator/reallocator/free calls during the panic and recovery callbacks.

## Reproduction and verification

The real Windows fixtures reproduced each reviewed failure before repairs:

- `cargo test -p windfall-desktop --lib runtime_repair_ -- --nocapture` failed with captured `0.5` instead of committed `0.75` before audio drained, and accepted a changed inner bundle DLL under an unchanged directory stamp. Output: `target/runtime-repairs-desktop-red.log`.
- `cargo test -p windfall-plugin-host --test realtime runtime_repair_ -- --nocapture` failed with 893 dropped native events in the multi-channel panic sequence. Its allocator assertion passed before the release assertion failed. Output: `target/runtime-repairs-host-red.log`.

The persistence regression covers capture, save, backup, ZIP save/open, file reopen and WAV export immediately after empty callbacks, before a nonempty drain. It also covers save before any callback observes the committed edit. Export output matches a subsequent settled export exactly. Playback tokens remain unchanged through capture, save, backup and export. Additional native tests cover adapter-absent controls, timeout then retry, deactivation refusal preserving accepted controls, successful recovery, stale parameter snapshot refusal, newer processed automation, and the deactivation-generated `0.375` preset including older queued points.

Verification uses Windows Git Bash with `source scripts/msvc-env.sh`, `CARGO_BUILD_JOBS=1`, `RUST_TEST_THREADS=1`, worktree-local `CARGO_TARGET_DIR=target/vst3-enable-verification`, and temporary `TS_RS_EXPORT_DIR=windfall-vst3-repairs-bindings`. Builds run serially; fixture/scanner helpers inherit the one-job limit.

Final functional checks on Windows, 2026-10-07:

| Command | Result | Worktree-local log |
|---|---|---|
| `cargo test -p windfall-plugin-host --all-features --lib --test realtime --test vst3_host --test vst3_scan --test params_state --test scanner` | 81 passed | `target/runtime-repairs-host-scoped.log` |
| `cargo test -p windfall-desktop --lib plugins::` | 24 passed | `target/runtime-repairs-desktop-final.log` |
| `cargo test -p windfall-desktop --lib plugin_recording` | 5 passed | `target/runtime-repairs-recording-final.log` |
| `cargo test -p windfall-desktop --lib plugin_update` | 1 passed | `target/runtime-repairs-stale-final.log` |
| `cargo test -p windfall-desktop --lib native_clap` | 3 passed | `target/runtime-repairs-hardware-final.log` |

These are scoped native regression suites, not a repeated full workspace run. Callback allocator counters passed with zero allocations, reallocations and frees in the measured paths.

Strict checks also passed:

- `cargo clippy -p windfall-plugin-host --all-features --all-targets -- -D warnings` (`target/runtime-repairs-host-clippy.log`).
- `cargo clippy -p windfall-desktop --lib --tests -- -D warnings` (`target/runtime-repairs-desktop-clippy.log`).
- `cargo clippy --manifest-path crates/windfall-plugin-host/test-plugins/Cargo.toml -- -D warnings` (`target/runtime-repairs-fixture-clippy.log`).
- Workspace and standalone fixture `cargo fmt --check`, and `git diff --check`.

No UI source or generated binding/WASM changes belong to this repair. Full artifact generation and full workspace parity verification remain parent-owned. Native editor windows, installed external plugins, physical audio/MIDI devices and other operating systems remain unverified. Allocator counters measure the host, not arbitrary external DLL allocation. Already-running in-process native hangs cannot safely be interrupted; joined cancellation can exceed the nominal 15-second wait.

## R4: deduplicated adoption and native parameter points

Both subsequent findings were reproduced with compiled tests before changing functional code:

- `cargo test -p windfall-desktop --lib r4_ -- --nocapture` failed in all four real engine/session combinations (CLAP/VST3, effect/instrument). Document `0.5`, processed automation `0.75`, committed document `0.75` with cache deduplication, then processed automation `0.25` produced captured `0.75`. The native getter and engine automation frame already reported `0.25`. Guarded callbacks made zero allocator calls. Log: `target/r4-desktop-roles-red.log`.
- `cargo test -p windfall-plugin-host --test realtime r4_ -- --nocapture` failed at actual native output frame 1024: `1.0` instead of `1.5`. One inactive point plus 1024 timed host points exceeded the previous native point arena. A second regression adds disjoint inactive/editor IDs and proves those values through a separate native audio output. Both allocator guards passed before the final timed-sample assertions failed. Log: `target/r4-host-sources-red.log`.

`HostedEffect::adopt_parameters(&[PluginParameter])` is a default no-op metadata hook, inherited by instruments. `ExternalEffect::apply` and `InstrumentUnit::apply_plugin` invoke it before cache deduplication. Runtime observes a stable matching committed generation/value pair; different values from an old plan cannot observe newer intent. Adoption does not publish a desired control, enqueue a native point, acknowledge delivery, allocate, wait, or call native code. Processing acknowledgement follows a later nonempty successful native call with no outstanding rejected value and no increase in aggregate dropped events measured around that call. Thus later processed automation can supersede the adopted document edit even when the edit needed no new native control.

The synchronous local facade's native return is completion evidence. A process bridge must additionally match its helper sequence/epoch and actual successful helper completion; submission or an early facade return is insufficient. An unchanged aggregate drop count around completed processing is affirmative evidence for a conservative no-drop gate. A nonzero delta can include output/notification drops and therefore remains unknown rather than proving which parameter generation processed.

VST3 input parameter storage is allocated before callbacks for all bounded sources:

```text
256 deferred points + 4096 editor queue points + 1024 ordinary host points = 5376
```

COM queues are bounded by the smaller of declared parameter count and this input bound. Reserved instrument events cannot contain parameters. Same-ID/same-frame coalescing is optional headroom, not part of the bound. Scheduled host offsets remain intact; editor and inactive points stay at frame zero. Output storage retains its separate 1024-point bound. Host ordinary admission and CLAP capacities are unchanged.

Native input insertion returns success/refusal. Readback publishes each input queue's final value only after the actual native process succeeds, followed by newer native output values. Unexpected input refusal fails the block before native processing, reports its translated drops through the common failed-block health path, and retains final values in owner-allocated storage without changing active readback. Native process failure also retains final input intent. On owner return those retained values become inactive pending controls; successful deactivation-generated edits retain their newer ordering. Recovery allocates the normal input arena and processes retained intent. No callback storage grows or is destroyed.

Additional coverage fills all 5376 point and queue slots without coalescing, fills the entire editor queue alongside other sources, checks next-block actual native values, verifies failed-process readback, and fault-injects an unexpectedly unavailable native input queue to prove truthful drop counts and eventual processing after owner recovery. The four-role session regression checks capture, save, file reopen, offline WAV output matching reopened export, unchanged playback token, and zero callback allocations, reallocations and frees. The original three R3 paths and dirty/recording/absence/refusal guards remain in the scoped suites.

R4 verification on Windows, 2026-10-08, uses the same isolated target, temporary type-export directory and serial one-job build environment described above:

| Command | Result | Worktree-local log |
|---|---|---|
| `cargo test -p windfall-desktop --lib r4_ -- --nocapture` | 5 passed; all four real engine/session roles plus stale-plan/adoption acknowledgement | `target/r4-desktop-green.log` |
| `cargo test -p windfall-plugin-host --all-features --lib --test realtime --test vst3_host --test vst3_scan --test params_state --test scanner` | 87 passed | `target/r4-host-scoped.log` |
| `cargo test -p windfall-plugin-host --all-features --lib --test realtime r4_ -- --nocapture` | 5 passed after the final admission-check style cleanup | `target/r4-host-final.log` |
| `cargo test -p windfall-desktop --lib plugins::` | 29 passed | `target/r4-desktop-scoped.log` |
| `cargo test -p windfall-desktop --lib plugin_recording` | 5 passed | `target/r4-recording-scoped.log` |
| `cargo test -p windfall-desktop --lib plugin_update` | 1 passed | `target/r4-stale-scoped.log` |
| `cargo test -p windfall-desktop --lib native_clap` | 3 passed | `target/r4-hardware-scoped.log` |
| `cargo test -p windfall-engine --lib plugins::` | 8 passed | `target/r4-engine-scoped.log` |

Strict host all-feature/all-target, engine all-target, desktop library/tests and standalone fixture Clippy passed with `-D warnings` (`target/r4-{host,engine,desktop,fixture}-clippy.log`). Workspace/fixture formatting and `git diff --check` passed. No UI or generated artifacts changed; full combined-source and bridge verification belongs to the parent. The parent-owned scanner-test default-target fallback is outside this patch and remains intact when applying it. The external-plugin/editor/device/platform, metadata-identity and native-hang limitations above still apply.
