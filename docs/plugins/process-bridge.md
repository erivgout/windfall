# Windows audio-plugin process bridge

Design proposal for N4, based on `71a0b76f`. Implementation and measured results
will be recorded below as they become available. This document does not close
the bridge or native editor parity rows.

The owner has since advanced its worktree baseline to `7f70315c` (including
`19caf872`) at the parent's direction. The parent approved the proposed 2B
schedule and host-library edit window; production runtime/manager/main and
shared fixture hooks remain reserved while native control-ordering repairs are
reviewed. The initial 512-frame default delay was a proposal; the evidence
below now measures it with B=256 native helper fixtures, separately from B=64.
The owner subsequently imported reviewed R4 `ec601edf` in local merge
`02072c81`, using the parent's applied `ea145d93` composition to preserve the
accepted capture and native point-delivery repairs. Production wiring is still
reserved separately.

## Boundary and ownership

Each prepared CLAP/VST3 instance gets its own helper process, supervisor and
shared mapping. Native modules, instances, state calls and teardown stay in
that helper, on its creating thread. The desktop keeps the existing
`HostedEffect` / `HostedInstrument` factory facade. Playback selection still
occurs from installed audio blocks; speculative and independent render helpers
cannot become state owners. Capture requests and replies include a session,
instance token, document revision and binding identity. The desktop must check
the exact binding and installed selection before and after capture.

This is crash containment, not a security sandbox. Native code can access the
user's files and other processes with their permissions. Corrupt protocol data
is rejected; isolation does not make an intentionally hostile plugin safe.

## ABI version 1

Only 64-bit little-endian Windows is initially enabled. The mapping is a fixed
size, page-aligned array of aligned 32-bit words. No Rust enum, pointer, slice,
`Vec`, `Arc`, struct padding or native object crosses the boundary. Integers,
IEEE-754 floats and event discriminants have explicit byte offsets and
little-endian encodings. Every payload word uses an atomic relaxed load/store,
including metadata, so malformed helper publications cannot create a Rust
data race. Slot state uses acquire/release operations on aligned lock-free
32-bit atomics. This is an explicitly supported platform ABI, not a claim that
Rust structs or atomics have a portable cross-process representation.

The immutable header records magic/version, mapping/header/slot sizes, four
slots, block capacity 512, event capacity 1153 (1024 ordinary + 129 reserved
releases), parameter capacity 4096, rate, selected block size, native latency,
kind, session/token/revision/binding identity and all section offsets. Validate
the exact computed layout, supported platform, capacities, overflow, alignment,
rate (8–384 kHz), block (1–512), latency (total at most one second), and identity
before any native load. Revalidate every slot length, event type/time/order,
note/channel/velocity, parameter ID/range/value and transport finite values.
Audio is planar stereo `f32`. Nonfinite input is sanitized locally. Nonfinite,
stale or corrupt output rejects the completed block and increments telemetry.

Each slot holds identity, sequence, transport, input/output audio, block-start
held-note and current-parameter snapshots, and ordered events. Snapshots allow
the helper to reconstruct current controls after skipped sequences without
replaying released notes. Ordinary overflow rejects note-ons and retains the
latest parameter for a later block; release admission has its own reserve.

Document commit, host admission and native processing are separate generations.
Submission and returning from the desktop facade never acknowledge processing.
Only a matching successfully acquired COMPLETE reply may advance the DSP
processed generation. Checked inactive capture reconciliation has a separate
generation and does not acknowledge DSP processing. A helper with
any native event/point admission failure must not acknowledge that generation;
it reports overflow and retains current desired controls for reconstruction.
Controller readback alone is not evidence that DSP processed a timed value.

This first engine facade addresses notes on channel zero and at most 128 keys;
the generic wire still validates 0–15 channels. Snapshots name the channel
explicitly. If ordinary admission saturates, note-ons stop entering. Reserved
releases coalesce only where an earlier release covers the same key/time and
no admitted note-on intervenes. There are at most 128 key releases and one
panic per block after ordinary admission stops. Earlier releases use ordinary
capacity; repeated releases/panics cannot consume an unbounded reserve. A
rejected release keeps the desired held snapshot updated, latches incomplete
control admission, and cannot be acknowledged as native processed.

Sequences/control generations/timeline epochs are monotonically increasing
64-bit counters. Before overflow they latch the instance failed; a new prepared
instance receives a fresh session and mapping rather than wrapping identifiers.
Reset/seek increments a timeline epoch, discards adapter accumulation/delayed
audio, and rejects replies from older epochs. Helpers reconstruct controls on
epoch or sequence discontinuity; output cannot resurrect a pre-seek note.

Slot ownership is `EMPTY -> HOST_WRITE -> READY -> HELPER_WRITE -> DONE ->
HOST_READ -> EMPTY`. Host and helper use bounded single-attempt CAS, never a
callback spin loop. Payload is published with Release; the next owner acquires
before reading. A late READY slot can be reclaimed only by a successful CAS
competing with the helper's claim. A HELPER_WRITE slot is never reclaimed while
that process lives. A DONE slot is acquired before copy/rejection. Four slots
bound backlog. Helper exit must be confirmed before control-side reclamation;
there is no in-place mapping restart that can expose an old helper's writes to
a new owner. Each fresh prepared instance uses a fresh session/mapping.

## Timing and fallback

The internal pipeline block is 256 frames by default, independent of host
callbacks (including 1/7/64/480/512 frames). Input block n is collected over
`[nB, (n+1)B)` and published at `(n+1)B`. Its output is tested once at `(n+2)B`
and played over `[(n+2)B, (n+3)B)`. Thus aggregation plus one scheduling block
adds **2B**, not B, frames. Report `native_latency + 2B` through the facade so
the existing graph PDC and dry/wet compensation account for it. An impulse
test must measure this delay rather than assuming it. Startup is silent until
the delayed input is available. A healthy worker may still miss the deadline;
fixed latency provides a work window, not a realtime completion guarantee.

Callback code only copies preallocated bounded buffers, encodes checked values,
and accesses mapped atomic words and telemetry. It never performs filesystem,
pipe, process or job IO, allocation/free, locks, clock calls or waits. Missing,
late, failed or corrupt output uses dry audio delayed by the same total latency
for effects, and silence for instruments. Availability transitions apply a
bounded sample ramp preserving continuity at the transition. Held notes and
latest parameters remain bounded during failure. No native plugin is loaded
into the desktop as an isolation fallback.

Offline helpers are separate instances and owners. Offline processing may wait
for the same sequence off realtime with a deadline and cancellation; output is
still passed through the identical block adapter and aligned by the same
latency. Streaming and offline scheduling differ. A failed export returns an
error to the existing encoder staging owner; this module never publishes or
deletes output files, and never modifies the live plugin/document.

## Control and supervision

A bounded length-prefixed control protocol over one private loopback TCP
connection is separate from mapped audio (native stdout cannot corrupt it).
The 24-byte little-endian frame prefix bounds metadata to 1 MiB and native
state to the existing 256 MiB host limit. Partial reads retain framing state;
there is one reader and no abandoned blocked reader thread per timeout.
Load, state capture/restore, editor refusal, and shutdown carry request IDs and
the full owner identity; state bytes use the existing checked WFPS/VST3
container. Transport and automation use only the audio path. Control IO and
native calls run away from audio. Native VST3 capture proves deactivation and
captures/reprepares on the helper owner; refusal retains the exact adapter and
returns an error. CLAP capture preserves its accepted active-state semantics.
Retain the last validated successful state; a truncated/malformed/stale reply
cannot replace it. Pending controls have finite deadlines; hung native work
ends by killing only that helper and waiting for its exit.

The supervisor detects death, startup/control timeouts and stalled published
audio progress. All such failures latch that instance unavailable. No automatic
retry storm: retries are explicit, recreate identity and mapping through the
factory revision, and use a bounded per-binding failure budget and cooldown.
After the budget, report blocked until explicit user retry/rescan. Retirement
is control-side: stop, kill on deadline, reap, close pipes/map handles and join
workers. The audio half retains its resources until the engine's existing
control-side retirement; a supervisor must not unmap an active audio view.

The packaged desktop executable will recognize an audio-helper mode before
starting Tauri, as it already does for scanning. Test binaries may use a
standalone helper. Production discovery must use the current executable, not
Cargo target paths. Native editor requests return an explicit unsupported
error in this first delivery. Parenting, scaling, focus and editor lifecycle
remain followup work. Manager health/retry/selection require coordinated
shared wiring; persisted wrapper options are outside this patch.

## Dependencies and verification

Use existing serde/serde_json, rtrb and Windows bindings. The approved feature
addition enables Memory/Security in existing `windows-sys` **0.61.2**, whose
only locked package dependency is existing `windows-link` **0.2.1**. Both local
registry manifests declare MIT OR Apache-2.0, compatible with the GPL project.
The feature change adds no package, lockfile change, native library download or
version upgrade. Mapping pages are pre-touched on control before installation;
OS memory residency is not promised by headless tests.
Portable followup uses the same explicit byte protocol with OS shared mappings
and verified aligned lock-free atomics on each supported architecture. It
requires actual macOS/Linux execution and their native main-thread lifecycle.

Required checks include malformed layouts and identity/events; adapter impulse,
variable callbacks, parameter/note saturation, seeks/reset, fallback ramps and
allocator counters; real helper crash/hang/state-capture death, cleanup and
independent instances; healthy native CLAP/VST3 reference comparison; engine
PDC/routing/automation/export/stems; desktop save/undo/replacement/recording
barriers and manager controls. Maximum callback duration is reported separately
from average CPU, with no hardware deadline claim from headless tests.

Packaged installer discovery, licensed Surge/OB-Xf Windows corpus with exact
versions/formats/settings/output, native editors and other platforms remain
external verification gates. Generated bindings/descriptors/WASM and global
parity updates belong to the parent.

## Implementation evidence

Implemented in new owned host modules: explicit protocol/slot transport,
Windows mapping, preallocated callback adapter, independent render scheduling,
bounded control framing, creating-thread native helper and per-instance
supervisor. The standalone `windfall-plugin-audio` binary is exercised on
Windows/MSVC; production desktop activation remains deferred under the
parent's existing runtime/main/manager edit reservation.

Current Windows headless evidence:

- Fourteen bridge unit checks pass, including exact layout/identity/events,
  corrupt output, late helper-owned slot preservation, overflow snapshots,
  epoch rejection, no submission acknowledgement, continuous fallback ramps,
  counter exhaustion, transport-bound exhaustion without callback panic,
  bounded retry/cooldown/stale-attempt rejection and local-reference delay.
- Ten real process checks pass: CLAP native crash, nonfinite samples and a
  helper deadline exceeded by a slow native block are terminated/reaped while
  the parent stays alive with aligned dry fallback; VST3 processing errors and
  nonfinite samples are likewise contained. These tests do not yet exercise a
  permanent native hang or exit during capture.
- Healthy CLAP/VST3 native state capture and in-process restore preserve real
  values; different instances use different helper PIDs and retiring one
  leaves the other healthy. Unsupported native editor requests return an
  explicit error.
- Independent offline helpers match actual in-process deterministic fixture
  output at precision `1e-7`, using full, unsplit callback sizes
  **1/7/64/480/512** and a measured extra **128 frames for B=64**. CLAP sine
  begins at exact onset 160 (first nonzero sine sample 161), verifying real
  native 32 + bridge 128 latency and note release. The current VST3 gain fixture
  advertises 17 frames but does not implement an audio delay; its reported PDC
  latency is 145 while comparison measures only actual bridge-added 128.
  It cannot establish truthful native-latency acoustic alignment.
- With the default B=256, independent real CLAP/VST3 gain helpers place an
  impulse at exactly **512 frames** under offline scheduling. CLAP reports 512
  total; the undelayed VST3 fixture reports 529.
- Newly appended truthful delayed CLAP and VST3 effect fixtures implement a
  preallocated 37-frame stereo delay. Healthy helper audio and delayed dry
  fallback after confirmed termination both begin at **165 frames for B=64**
  and **549 for B=256**, exactly matching the adapter's latency report. VST3
  class 10 follows the unchanged original classes 0–9; scanner count is 11.
  Engine graph PDC/routing/dry-wet integration remains unverified until wiring.
- Before any native DSP block, capture retains pending parameter intent without
  claiming DSP acknowledgement. CLAP stays active: its opaque state retains
  actual 0.5 while the separate companion value carries pending 0.75. VST3
  reconciles inactive state to 0.75 and reports a separate reconciliation
  generation. Both report DSP generation zero. Rejected capture metadata
  preserves the previous validated state and leaves a healthy helper alive.
- Separate offline native helpers return cancellation/deadline errors without
  changing the caller's unconsumed buffers. Explicit control-side termination
  confirms exit while audio retains its mapping for allocation-free fallback,
  and an independent live helper stays healthy. Exit bookkeeping reports reaped
  only after confirmed process exit, with a bounded termination deadline.
- A calibrated thread-local allocator guard records zero callback alloc,
  realloc and free calls through startup, full/late slots, parameter/note
  saturation, reset and latched failure with variable callbacks. One recorded
  run of 320 calls measured maximum 73 microseconds and average 1 microsecond.
  Maximum and average are separate wall-clock observations on a busy machine,
  not CPU utilization or a physical audio deadline claim.
- Strict host Clippy with **all features and all targets**, and the appended
  VST3 factory/scanner count check,
  and formatting pass. Native processed acknowledgement is conditional on
  separate event admission success, a matching successful nonempty native
  completion, and unchanged nonsaturated dropped counters taken immediately
  before/after native processing. Native input/output losses are separately
  observable and retain desired controls; the parent is still accepting the
  VST3 point-capacity prerequisite repair before production use.
- All sixteen existing realtime checks pass, including native CLAP/VST3
  allocator, note-release reserve and ownership regressions. The fixture was
  built separately and the test executable run directly to keep one Cargo
  process per owner.

Pending: production factory/manager/helper mode and adopted document metadata,
true hang/capture-exit/malformed-state/bad-latency fixture hooks (shared hooks
reserved), save/undo/replacement/recording barriers, engine PDC/routing/export
and stems, manager wiring for the tested retry budget, packaged installer discovery, licensed
corpus, native editors and other OS execution. Existing state bytes are opaque:
checked container/stream validity is enforced, but this does not prove an
arbitrary plugin can restore every semantically malformed native payload.
