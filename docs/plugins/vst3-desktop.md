# VST3 desktop ownership exchange

VST3 additions remain **disabled**. The native backend's inactive-state
contract is unchanged: the processor must return to its creating owner before
state is saved or restored. This change implements and exercises that return
path without claiming that the complete desktop integration is ready.

## Implemented ownership path

`windfall-plugin-host::ownership::exchange` allocates two single-item `rtrb`
queues and an atomic request flag before handing an adapter to the engine.
The engine calls `HostedEffect::control_boundary` for every installed effect
and instrument, including stopped, bypassed and leaving units. Even an empty
callback services the exchange before plan messages are applied.

At a boundary, the audio endpoint moves its adapter into the return queue.
There is no native stop/deactivation, allocation, destruction, lock, wait,
file operation or owner-thread job submission in this operation. Unexpected
queue fullness retains the adapter locally. While ownership is absent,
instruments produce silence and effects pass through dry input. The wrapper
keeps its installed latency and tail metadata throughout the pause.

The desktop plugin owner waits up to 500 ms for each requested adapter,
releases it through the matching instance, captures inactive state, prepares
a new adapter on the same owner and publishes it into the resume queue.
The next boundary reacquires it. State is never saved by the VST3 backend
while its processor is active. A failed save still attempts reacquisition;
preparation errors or changed latency/tail leave the slot silent/bypassed
and return an error. Failed native processors are not silently reactivated.

A callback that never arrives yields an explicit capture error and cancels
the request without deactivating the outstanding processor. Cancellation
can race a boundary, so the owner also drains late returns during normal
maintenance and restores them. A second request before reacquisition can
return the already queued adapter at the next boundary. These operations
retain exactly one adapter owner throughout.

Retirement sends the whole audio endpoint to the native owner through the
existing controller-side destruction path. It drains adapters held locally,
queued for resume or returned but not yet serviced, closes the editor and
deactivates there. If the owner channel has failed, the retirement payload
is deliberately leaked; native teardown must not run on the controller or
callback thread after an owner failure. This contains wrong-thread teardown,
not a plugin crash or a hung native call.

## State and document identity

Capture retains the playback instance token. Offline factories still produce
independent instances that cannot select an editor/state owner. Speculative
preparation does not select an instance. The existing staged revision API
is retained. Live capture checks playback role, token, installed revision,
path, format, plugin ID and the original opaque-state fingerprint. It checks
revision and token again after the exchange before accepting captured bytes.

Save, backup and export snapshots now carry the native document revision
read alongside the project copy. A stale snapshot cannot capture a replacement
document's live state merely because the replacement uses the same binding.
Save and backup acquire recording exclusion for native capture. Export uses
its already-held exclusion, preserving recording-before-state lock order
without recursively locking recording. The document lock is released before
any native capture request. CLAP retains its existing active-state save path.

Returning a processor also preserves parameter points that had not reached
an audio block. The common wrapper gathers pending/main-queue parameter
events on the owner and flushes their final values after deactivation.
Their former sample offsets belong to the abandoned audio timeline; note
events are discarded. VST3 retains pending controller edits as deferred
state overrides before its native processor is destroyed. Normal callback
processing still uses its fixed buffers and bounded queue drains.

## Blockers before enabling additions

1. **Deferred native dirty-state capture.** The owner notification loop still
   tries `save_state` directly for `StateChanged`. VST3 correctly refuses
   active saves, and that notification is currently discarded. Enabling the
   format would lose native preset/dirty edits from document history. The
   replacement must enqueue a capture request for the session, retain the
   exact token/revision/binding identity, wait for recording exclusion, and
   reject it if those identities changed. It must coalesce/retry pending
   requests while recording rather than quiescing audio from the owner loop.
   This must also cover native restart/parameter-rescan ordering and updates
   left over when the returned processor is deactivated. A queued request
   must be cancelled if its control caller times out; the existing generic
   15-second owner-job timeout does not cancel work already queued/running
   and must not release recording exclusion while a later quiesce can start.
2. **Events during the pause.** The desktop audio wrapper currently has no
   adapter while capture runs. Parameter and note commands arriving during
   that interval are ignored. Engine parameter caches can then suppress
   their later resend, and sustained instrument notes are not reconstructed
   on reacquisition. Before enablement, add bounded, preallocated pending
   parameter storage and held-note reconciliation, including note-offs,
   all-notes-off, removal and transport stop. Define the intended audible
   pause policy and test it through live automation and native editor changes.
3. **Native lifecycle rejection.** The backend's deactivation API returns
   no result and does not propagate a native `setActive(false)` refusal.
   The wrapper detects failure to release its instance ownership, failed
   processors and reprepare errors, but cannot prove successful native
   deactivation for an arbitrary rejecting plugin. Propagate and test that
   failure before treating the inactive state contract as fully satisfied.

The production catalog still sets VST3 entries to `usable: false`; runtime
creation separately rejects VST3 even if a caller supplies a usable entry or
saved binding. A test-only owner capability allows deterministic fixtures to
exercise the exchange. It is absent from production builds and cannot be
set through IPC, preferences or the project format. The plugin manager shows
the scanned identity and why additions are unavailable. Scan failures keep
their original error instead of being obscured by the integration message.

## Verification scope

Tests use the repository's independent VST3 DLL fixture on Windows/MSVC.
They exercise real SDK processing/state calls through the desktop owner,
engine instrument playback, `.windfall` save/reopen, independent offline
rendering, WAV encoding/decoding, failed processors, no-callback timeout,
late return, retirement before reacquisition, stale revisions and role
separation. These are headless buffers, not audio-device or native-editor
verification. No external Surge/OB-Xf, Linux or macOS run is claimed.

The calibrated host allocator test measures zero host allocations,
reallocations or frees for callback ownership return/reacquisition and
fixture processing. Engine boundary checks cover empty and stopped callbacks.
The counter does not measure allocations inside an arbitrary plugin DLL.
Validation on 2026-10-07 (every Cargo shell sourced `scripts/msvc-env.sh`,
with `TS_RS_EXPORT_DIR=/tmp/windfall-vst3-desktop-bindings`):

- `cargo test -p windfall-plugin-host --all-features --all-targets`: 96 passed,
  zero failed/ignored. This includes the calibrated ownership allocator test
  and the pending-parameter save/restore regression.
- `cargo test -p windfall-desktop --lib -- --test-threads=4`: 165 passed,
  zero failed/ignored. After preserving live CLAP latency/tail reads in the
  final wrapper refinement, `cargo test -p windfall-desktop --lib plugin --
  --test-threads=4` passed all 12 plugin tests again.
- `cargo test -p windfall-engine --lib plugins`: 7 passed, zero failed/ignored,
  including callback allocation checks for stopped, empty, bypassed and
  departing units.
- Host all-features/all-target clippy and engine/desktop default-feature
  all-target clippy, each with `-D warnings`: pass. ASIO was not enabled.
- `cargo fmt --all -- --check` and `git diff --check`: pass.
- Desktop `pnpm install --frozen-lockfile`, `pnpm test
  src/features/plugins/plugins.test.tsx --maxWorkers=4` (5 tests),
  `pnpm typecheck`, `pnpm lint` and the changed test's Prettier check: pass.

No generated bindings or wasm, parity files, README, publication metadata,
tags, releases, pushes or pull requests are included in this feature.
