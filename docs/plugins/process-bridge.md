# Windows audio-plugin process bridge

Design proposal for N4, based on `71a0b76f`. Implementation and measured results
will be recorded below as they become available. This document does not close
the bridge or native editor parity rows.

The owner has since advanced its worktree baseline to `7f70315c` (including
`19caf872`) at the parent's direction. The parent approved the proposed 2B
schedule and host-library edit window. Shared fixture classes10+ have a narrow
approved window; production runtime/manager/main routing remains reserved
while bridge repairs are independently accepted. The initial512-frame default
delay was a proposal; the evidence
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

## ABI version 2

Only 64-bit little-endian Windows is initially enabled. The mapping is a fixed
size, page-aligned array of aligned 32-bit words. No Rust enum, pointer, slice,
`Vec`, `Arc`, struct padding or native object crosses the boundary. Integers,
IEEE-754 floats and event discriminants have explicit byte offsets and
little-endian encodings. Every payload word uses an atomic relaxed load/store,
including metadata, so malformed helper publications cannot create a Rust
data race. Slot state uses acquire/release operations on aligned lock-free
32-bit atomics. This is an explicitly supported platform ABI, not a claim that
Rust structs or atomics have a portable cross-process representation.

The startup header records magic/version, mapping/header/slot sizes, four
slots, block capacity 512, event capacity 1153 (1024 ordinary + 129 reserved
releases), parameter capacity 4096, rate, selected block size, native latency,
kind, session/token/revision/binding identity and all section offsets. Validate
the exact computed layout, supported platform, capacities, overflow, alignment,
rate (8â€“384 kHz), block (1â€“512), latency (total at most one second), and identity
before any native load. Revalidate every slot length, event type/time/order,
note/channel/velocity, parameter ID/range/value and transport finite values.
Audio is planar stereo `f32`. Nonfinite input is sanitized locally. Nonfinite,
stale or corrupt output rejects the completed block and increments telemetry.

Each slot holds identity, sequence, transport, input/output audio, block-start
held-note and current-parameter snapshots, and ordered events. Snapshots allow
the helper to reconstruct current controls after skipped sequences without
replaying released notes. Admission/drop uncertainty also persists across
contiguous blocks: the next whole held snapshot must actually be reconciled
before its generation can be acknowledged, even without a parameter change. Ordinary overflow rejects note-ons and retains the
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
the generic wire still validates 0â€“15 channels. Snapshots name the channel
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
latency. Streaming and offline scheduling differ. Waiting and success checks
observe the shared native-failure latch directly, independently of the
supervisor's later signal. Any offline error clears both supplied buffers so
partial/fallback audio cannot be mistaken for successful export output. The
staging owner must discard the complete artifact and flush the healthy delay
pipeline before publication. A failed export returns an error to the existing encoder staging owner; this module never publishes or
deletes output files, and never modifies the live plugin/document.

## Control and supervision

A bounded length-prefixed control protocol over an authenticated loopback TCP
connection is separate from mapped audio (native stdout cannot corrupt it).
Before any Load/path/state/mapping/owner metadata is sent, the supervisor
creates a 32-byte BCryptGenRandom system-preferred key. A new private child
stdin pipe carries that key plus the eight-byte expected session; neither key
nor owner metadata is placed in CLI/environment/logs. The child sends a fixed
40-byte Hello: WFAH magic (4), Hello version1 (4 LE), nonce (32). Malformed,
wrong-version/key and stalled clients receive zero metadata and are closed.
At most eight candidates are polled without blocking; a full stalled set
releases its oldest candidate for a newcomer. Candidate age is 50 ms, and the
whole startup retains deadline/cancellation/child-exit bounds. RNG, pipe or
authentication failure fails closed and the child owner reaps on unwinding.
This prevents an unrelated first loopback client from obtaining the launch
payload; it is not a security sandbox or privileged-adversary defense.
Hello version1 and control framing version1 are distinct from mapping ABI2;
none has a downgrade path.
The 24-byte little-endian frame prefix bounds metadata to 1 MiB and native
state to the existing 256 MiB host limit. Partial reads retain framing state;
there is one reader and no abandoned blocked reader thread per timeout.
Load, state capture/restore, editor refusal, and shutdown carry request IDs and
the full owner identity; state bytes use the existing checked WFPS/VST3
container. Transport and automation use only the audio path. Control IO and
native calls run away from audio. Native VST3 capture proves deactivation and
captures/reprepares on the helper owner; refusal retains the exact adapter and
returns an error. CLAP capture preserves its accepted active-state semantics.
Retain the last validated successful state; cache publication is fenced by
serialized request ID so an older caller cannot overwrite a newer capture.
A truncated/malformed/stale reply cannot replace it. Retained DSP proof is
bound to the actual processing epoch, even when VST3 capture clears sequence
continuity; inactive reconciliation never relabels old DSP proof after reset. Pending controls have finite deadlines; hung native work
ends by killing only that helper and waiting for its exit.

Header word27 (byte108) is a bounded u32 native-owner completion counter.
The helper publishes it with Release only after the synchronous native
process/control and idle owner turn returns; the supervisor observes Acquire.
It is liveness evidence only, never admission or DSP-generation proof. No-work
idle/stopped sessions do not arm the watchdog. Outstanding work arms a fresh
age; READY turnover and temporary empty gaps cannot reset it. Actual owner
completion may disarm that obligation; new outstanding work then starts fresh.
The helper exits before counter wrap; restart uses a new session/map. This
bounds a helper lifetime to at most u32::MAX owner turns, independently of the
64-bit audio sequence/generation limits.

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
addition enables Memory/Security/Cryptography in existing `windows-sys` **0.61.2**, whose
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

- Twenty-one bridge unit checks pass, including exact layout/identity/events,
  corrupt output, late helper-owned slot preservation, overflow snapshots,
  epoch rejection, no submission acknowledgement, continuous fallback ramps,
  counter exhaustion, transport-bound exhaustion without callback panic,
  bounded retry/cooldown/stale-attempt rejection and local-reference delay.
- Twenty process integration checks pass (one additional ignored test is an
  explicitly invoked client subprocess role): CLAP native crash, nonfinite samples and a
  helper deadline exceeded by a slow native block are terminated/reaped while
  the parent stays alive with aligned dry fallback; VST3 processing errors and
  nonfinite samples are likewise contained. Newly appended CLAP/VST3 permanent
  native process hangs are terminated; a CLAP on_main_thread permanent hang is
  killed while B=256/48k callbacks keep replacing expired READY slots.
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
  class 10 follows the unchanged original classes 0â€“9; scanner count is 18 (classes11–17 add process hang, held-key probe and fault fixtures).
  Engine graph PDC/routing/dry-wet integration remains unverified until wiring.
- Before any native DSP block, capture retains pending parameter intent without
  claiming DSP acknowledgement. CLAP stays active: its opaque state retains
  actual 0.5 while the separate companion value carries pending 0.75. VST3
  reconciles inactive state to 0.75 and reports a separate reconciliation
  generation. Both report DSP generation zero. Rejected capture metadata
  preserves the previous validated state and leaves a healthy helper alive.
- Separate offline native helpers return cancellation/deadline errors with
  both supplied buffers cleared on error. Explicit control-side termination
  confirms exit while audio retains its mapping for allocation-free fallback,
  and an independent live helper stays healthy. Exit bookkeeping reports reaped
  only after confirmed process exit, with a bounded termination deadline.
- A calibrated thread-local allocator guard records zero callback alloc,
  realloc and free calls through startup, full/late slots, parameter/note
  saturation, reset and latched failure with variable callbacks. One recorded
  run of 320 calls measured maximum 73 microseconds and average 1 microsecond;
  the final R1 run observed maximum 9 and average 1 microseconds.
  Maximum and average are separate wall-clock observations on a busy machine,
  not CPU utilization or a physical audio deadline claim.
- Strict host Clippy with **all features and all targets**, and the appended
  VST3 factory/scanner count check,
  and formatting pass. Native processed acknowledgement is conditional on
  separate event admission success, a matching successful nonempty native
  completion, and unchanged nonsaturated dropped counters taken immediately
  before/after native processing. Native input/output losses are separately
  observable and retain desired controls. R4 prerequisite source is accepted;
  production use still awaits independent bridge-repair acceptance.
- All twenty existing realtime checks pass, including native CLAP/VST3
  allocator, note-release reserve and ownership regressions. The fixture was
  built separately and the test executable run directly to keep one Cargo
  process per owner.

Pending: production factory/manager/helper mode and adopted document metadata,
semantic-native-state validation, save/undo/replacement/recording barriers, engine PDC/routing/export
and stems, manager wiring for the tested retry budget, packaged installer discovery, licensed
corpus, native editors and other OS execution. Existing state bytes are opaque:
checked container/stream validity is enforced, but this does not prove an
arbitrary plugin can restore every semantically malformed native payload.

## ABI2 byte offsets and R1 repair evidence

All integer fields below are little endian. Header length is 256 bytes, each
slot 85,824 bytes, mapping 343,552 bytes. Slot i starts at 256 + i * 85,824.
No shared Rust layout defines these offsets. Unlisted startup header words
must be zero; attach rejects ABI1 or any size/offset/identity mismatch before
native load. Latency negotiation updates byte44 before audio installation.

| Header byte offset | Field |
| --- | --- |
| 0/4 | WFBR magic / mapping version2 |
| 8/12/16/20 | mapping bytes / header words / slot words / slot count |
| 24/28/32 | max block512 / events1153 / parameters4096 |
| 36/40/44/48 | rate / block / negotiated native latency / role |
| 52/56/60 | input / output / events section word offsets |
| 64/72/80/88 | u64 session / token / revision / binding |
| 96/100 | notes / parameters section word offsets |
| 104/108 | helper failure latch / owner completion counter |

| Slot byte offset | Field |
| --- | --- |
| 0 | CAS ownership state |
| 4/12/20/28 | u64 input session / token / revision / binding |
| 36/44/48/52/56/60 | u64 sequence / frames / event count / parameter count / incomplete flags / note channel |
| 64 | transport40B: playing/numerator/denominator u32, tempo/beats/seconds f64, reserved u32 |
| 112 | u64 input epoch |
| 128/136/144/152 | u64 reply session / token / revision / binding |
| 160/168/172/176/184 | u64 reply sequence / frames / status / epoch / DSP generation |
| 192/200/208 | u64 control start / control end / u32 native drops |
| 256/4352 | planar stereo input / output; each has 512 f32 samples per channel |
| 8448 | 128 f32 held velocities on named channel0 |
| 8960 | up to4096 parameters: u32 id + f64 value (12B each) |
| 58112 | up to1153 events, six u32 words (24B each) |

Event tags 1/2 are on/off: tag,time,key,channel,f32 velocity,zero. Tag3 is
panic: tag,time,then zeros. Tag4 is parameter: tag,time,id,zero,f64 value.
Control framing remains WFCB/version1: 4B magic,4B version,8B request,
4B metadata length,4B state length, followed by checked JSON and WFPS bytes.

The six source-review counterexamples were reproduced before repair:

- Watchdog: real CLAP idle/main-thread hang at B64 and requested B256/48k
  evaded the old timer during READY turnover; bounded RED, then GREEN reap.
  Healthy no-work idle/reset and 120 variable callback turns stay alive without
  fabricated DSP acknowledgement; temporary empty obligations/counter rollover
  have deterministic unit coverage.
- Notes: 1024 admitted key60 note-ons plus rejected key69 falsely acknowledged
  the continuous next generation with native key-probe output zero (RED).
  CLAP/VST3 now actually output one for key69 before matching COMPLETE ack,
  and a later release actually returns the native marker to zero.
- Offline: native shared failure/DONE with delayed supervisor flag returned Ok
  (RED); now Err(Failed) with both buffers zero (GREEN).
- Epoch: two real VST3 captures around reset without new DSP returned old
  generation under epoch2 (RED); epoch2 now carries zero DSP proof (GREEN).
- Cache: reversed cloned-caller publication overwrote request2 with request1
  (barrier RED); monotonic request fencing preserves request2 (GREEN).
- Authentication: restoring first-client acceptance made the real unrelated
  client consume Load and reset the connection (compiled RED); private-key
  Hello gates metadata (GREEN), including eight stalled first sockets and a
  wrong Hello. Wrong intended-child key and startup cancellation fail closed
  with bounded process reap. Crypto-failure propagation is implemented;
  actual OS RNG failure is not artificially induced.

Strict all-feature/all-target Clippy, fixture formatting and scanner counts
are rerun for the source-only response commit. These tests remain headless
Windows evidence; no production desktop activation, installed-package helper
claim, licensed-corpus result, native editor parity or other-OS claim follows.

## Capture and output-fault followup

After the fixed R1 response ba61a255, new CLAP bridge IDs and VST3 classes13–17
append capture-exit, capture-hang, partial-state refusal, unsupported latency
and output-event flood. Previous IDs/classes0–12 retain their behavior.
Capture faults trigger only after actual gain0.75 DSP so gain0.5 can establish
a real last validated state first. Both formats preserve that cached state on
exit/hang/refusal; permanent capture hangs are terminated under a100ms control
deadline, and an independent live instance remains healthy. Truncated3-byte
native writes followed by explicit save failure are rejected; recovery can
subsequently capture a healthy state again.

Actual native u32::MAX latency is latched unavailable and reported as an
explicit unsupported-native-latency startup error before audio installation;
a compiled RED showed that the earlier generic EOF hid the reason. Completed
authenticated control errors are decoded before considering child exit so the
bounded startup error remains visible. Native loss-counter exhaustion or
observed rollover terminates the helper; it cannot later claim affirmative
no-drop proof from a wrapped counter.

A5000-output-event fixture in each format proves native loss telemetry is
visible, lost/unknown generations receive no DSP acknowledgement, and desired
controls recover in actual healthy blocks after the flood is disabled. CLAP
fills owner notifications; VST3 fills its fixed SDK output note-event list.
Repeated same-offset VST3 parameter points coalesce by SDK contract, so that
would not constitute a capacity proof. Callback guards remain zero alloc/free.

The parent approved a narrow shared-host state-writer repair. LimitedWriter
now retains every refusal, uses checked size arithmetic, and refuses subsequent
writes/flush. CLAP save checks that sticky failure even if native code reports
success. A new CLAP-only bridge-ignored-stream-error ID deliberately ignores
the first refused write and returns success. Existing VST3 classes0-17 stay
unchanged, with factory/scanner count18. A32-byte native-boundary unit compiled
RED when partial bytes were accepted and GREEN after the guard; the helper's
actual256MiB cap compiled RED when the oversized reply lost the native owner.
GREEN preserves the previous cache, rejects capture, and completes healthy DSP
and capture on that same owner. The small-limit native unit is explicitly
ignored by default because it requires the separately built owned fixture; its
explicit fixture-backed invocation passes. The VST3 host already retains
sticky stream failure. These checks do not establish semantic validity of
arbitrary opaque native state or native editor behavior.

## R2 proof, startup backpressure and offline completion

DSP proof has a separate processed_epoch tag. Only the complete, matching,
nonempty successful native process with successful admission and affirmative
no-drop evidence can advance this tag with its processed generation. Current
native epoch/sequence may advance on an incomplete block without advancing
proof. Actual CLAP/VST3 regression compiled RED after epoch1 healthy DSP, reset,
then the first epoch2 note-overflow block: capture relabeled old proof under
epoch2. GREEN returns zero DSP proof, then acknowledges a later healthy epoch2
block. The earlier no-intervening-DSP VST3 capture regression remains green.

Authentication still completes before any Load disclosure. Startup Load now
writes incrementally on the authenticated nonblocking socket in at most64KiB
chunks. Every partial-write/retry checks the whole startup deadline,
cancellation and child exit; WouldBlock yields only on the control thread.
No truncation, retry of an entire already-partial frame, downgrade or local
DSP acknowledgement is allowed. The private debug test executable pauses
native Load reads500ms and only that test launch clamps SO_SNDBUF to4096; the
production socket buffer is unchanged. Existing windows-sys0.61 gained only
the approved Win32_Networking_WinSock feature, with no package/version/lock
change. Scripted old write_all compiled RED on WouldBlock; incremental writes
preserve exact framing across partial/Interrupted/WouldBlock responses and
stop on an injected deadline (GREEN). The actual supervisor::launch16MiB
valid native state completed with5017 observed WouldBlock retries in the
final run, then produced matching healthy audio and captured/restored state. Its native
inversion setting exists only in opaque state, outside the Load gain table,
so parameter adoption cannot hide a lost restore.
The old single large write_all still passed on this Windows machine even
with the clamp; this is not claimed as an executed native RED. Actual startup
cancel/deadline during backpressure refuse launch within the bounded control
budget and reclaim the mapping. Authentication's eight candidates and stale
owner/identity/framing checks remain unchanged. Not a security sandbox or
protection from a privileged adversary.

Offline scheduling rechecks cancellation/deadline/shared failure after DONE
and before final success. A private deterministic wait seam publishes DONE
simultaneously with cancellation or expiry: both compiled RED returning Ok,
then GREEN returning the precise error. The tests also consume a valid first
chunk before the failed final chunk and prove both entire128-frame supplied
buffers are cleared. Realtime process keeps no clock, wait or control IO.

Final narrow verification:24 bridge units,24 real-process integration cases
(plus one explicitly child-invoked ignored role), the fixture-backed CLAP
state unit and state-writer units pass. The calibrated320-callback run records
zero alloc/realloc/free, max5us and average1us wall time. Host lib48 cases
pass (plus the explicitly invoked ignored native state unit), all20 realtime
cases and both scanner cases pass; the scanner sees18 VST3 classes. Existing
parameter/state regressions also pass. These headless busy
Windows observations do not establish physical audio deadlines. Strict
all-feature/all-target host Clippy, host and complete fixture formatting, and
git diff --check pass for delivery. Production routing, engine
PDC/routing/export/stems, save/undo/replacement/recording integration, packaged
installer discovery, licensed Windows corpus, native editors and other OSes
remain open acceptance gates.
