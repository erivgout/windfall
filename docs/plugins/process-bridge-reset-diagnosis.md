# Reset/capture ordering diagnosis and ABI4 repair

Source pin:7027569f8b466242a75faf327381e6ac44c88d28. Owner stays frozen at
6b228e1216d6e33757faa2d5970fba66f935039d;19f/0c/71/6b remain immutable. The
parent subsequently granted this exact host-only repair window. This document
retains the original diagnosis and now records the implemented contract and
executed evidence; it does not authorize production activation or close P1.

Actual Windows CI run37766014784/job113273753509 failed only the second capture
in vst3_capture_does_not_relabel_dsp_proof_after_a_reset_without_processing,
bridge.rs1386: capture timeline ownership changed. The original epoch2,
processed_generation0 and audio-health acknowledgement0 assertions stay intact.
Raw log:C:/Temp/windfall-702-windows-ci.log.

Root427 (427ed0a7a07f3cae41ab1b91133b5e317b0f3dc9) CI run37770742963 repeated the same separate-process failure, unchanged
at line1386, with26PASS/1FAIL/1auth-role in6.79s. Raw log:
C:/Temp/windfall-427-windows-ci.log; verified SHA256:
a426ee5da407973a569ed7b6f6a6c2c6e373d0f1f774266fe41f41150850c79a.

Root125 CI run37775910822 supplied a third external pre-fix separate-process
failure at the unchanged line1386,26PASS/1FAIL/1role. Raw log:
C:/Temp/windfall-125-windows-ci.log; verified SHA256:
5f788bd632b3d3d524c9adc55ea0703859ef833977cad3327812716fd1ddc237.
It is provenance, not an additional repair window or inferred post-fix CI pass.

## Compiled bounded reproduction

A NEW diagnostic crate shell compiles existing native core modules read-only,
with exact root702 bridge helper/control/adapter/auth/mapping/protocol/slots
copies. All seven Git blob hashes match; the helper prefix is
421a2a311a0d1dc6184fcd4f6c105ba573f7d395. Its appended private diagnostic forces
owner turns around a real VST3 fixture. Native execution here is in-process solely
for ordering; actual separate-process evidence is the CI failure above.

The two-case diagnostic ran twice with the original capture oracle:1PASS/1RED
in0.05/0.06s, without scheduling sleeps, retries or fixture hooks. Baseline:
actual epoch1 processing ->capture1 clears sequence ->host reset ->capture2
returns proof0. Counterexample:

1. Collect matching epoch1 COMPLETE output; audio acknowledgement is1.
2. Leave one epoch1 READY tail slot, as the2B offline pipeline permits.
3. Capture1 returns actual proof1 and clears native sequence continuity.
4. Helper claims the old slot, becoming HELPER_WRITE.
5. Host resets to epoch2, clears local acknowledgement and discards reclaimable
   old READY/DONE slots. All64 header words remain unchanged. The busy slot stays
   helper-owned.
6. Its actual native call finishes: native epoch1/Some(sequence)/processed_epoch1.
7. Capture2 reaches the unchanged conservative mismatch guard and returns the
   exact CI error. No new epoch2 DSP occurred; host acknowledgement remains0.

The helper loop polls one control packet, then claims/processes one audio slot.
Replying to capture1 therefore does not fence later old tail work. Offline
process waits for matching earlier output, not all trailing submitted blocks.
Neither the first capture nor local host reset tells the helper that epoch2 now
owns the timeline. Clearing native sequence is insufficient and is independently
undone by legitimate old processing. This is an ordering gap, not a statistical
test diagnosis. SDK success/no-drop proof remains correctly tagged epoch1.

Compiled diagnostic:
target/plugin-process-bridge-verification/debug/deps/bridge_reset_diagnostic-96322ff373018156.exe
with WINDFALL_BRIDGE_FIXTURE pointing to the owner fixture DLL. Build command was
cargo test -p windfall-plugin-host --test bridge_reset_diagnostic reset_diagnostic::
-- --test-threads=1 --nocapture. Exact diagnostic sources are archived outside
Cargo discovery in target/plugin-process-bridge-verification/diagnostics/root702-reset-source;
temporarily returning the wrapper/subdirectory to their original tests paths
rebuilds the reproduction without changing any existing source.

## Approved and implemented reset authority contract

The audio endpoint publishes the authoritative timeline epoch. A capture request
does not publish or adopt an epoch. The helper acquires that authority to decide
which timeline its owner metadata describes. DSP proof remains independently
tagged with the epoch of actual processing. This repairs the missing publication
without revoking a native call or weakening the capture ownership check.

### Versions and exact byte layout

The repair uses mapping ABI **4** and authenticated Hello **3**. The latter is necessary
to reject an ABI3 helper *before sending Load*, rather than merely rejecting its
mapping before a native DLL load. It keeps the exact 40-byte WFAH Hello layout:
magic at bytes 0..3, LE version 3 at 4..7, private key at 8..39. Hello 1/2,
malformed framing and wrong keys receive no Load metadata. There is no downgrade.
The private 56-byte WFAP bootstrap remains version 1, with the existing key,
session and remaining startup budget; WFCB remains version 1. No additional
CLI/environment fields, secrets, dependency, facade or state format are needed.
Eight bounded authentication candidates and the whole startup deadline,
cancellation, process-exit and reaping rules remain unchanged. This remains crash
containment, not a security sandbox or privileged-adversary defense.

Mapping ABI4 adds exactly one word; lengths and all slot offsets remain fixed:

| Namespace | Word(s) | Byte offset(s) | Meaning / writer |
| --- | --- | --- | --- |
| Mapping header | 0..25 | 0..103 | Existing immutable layout/config/identity, including negotiated latency at word 11 |
| Mapping header | 26 | 104 | Existing shared failure latch |
| Mapping header | 27 | 108 | Existing native-owner completion counter; helper only, liveness only |
| Mapping header | **28** | **112** | **TIMELINE_EPOCH: nonzero LE AtomicU32; audio endpoint only after initialization** |
| Mapping header | 29..63 | 116..255 | Reserved zero |
| Slot metadata | 0..15 | 0..63 relative to slot | Existing ownership/input identity/sequence/counts |
| Slot metadata | 16..29 | 64..119 relative to slot | Existing 14-word meter transport |
| Slot metadata | 30..31 | 120..127 relative to slot | Existing u64 input epoch |
| Slot metadata | 32..39 | 128..159 relative to slot | Existing reply identity |
| Slot metadata | 40..47 | 160..191 relative to slot | Existing reply sequence/frames/status/epoch/processed generation |
| Slot metadata | 48..52 | 192..211 relative to slot | Existing desired control start/end and native drops |
| Slot metadata | 53..63 | 212..255 relative to slot | Reserved metadata |

The mapping header is 64 words/256 bytes. Each slot is 21,456 words/85,824 bytes;
slot starts are 256, 86,080, 171,904 and 257,728 bytes. Total mapping size remains
343,552 bytes. Header word 28 is unrelated to *slot* transport word 28. No
overlap, Rust-layout publication, split-word epoch publication or relocation is
introduced. Windows64 little-endian, lock-free atomic32/64 capability and
AtomicU32 size/alignment 4 remain required; OS mapping base alignment is checked.
LocalWords test backing has the same AtomicU32 alignment. A slot u64 epoch has a
zero high word and must be in 1..=u32::MAX in ABI4; control epoch remains a checked
u64 field with the same bounded domain. Native state limits remain unchanged.

Config::header/from_header and Region::initialize/attach require ABI4 and initial
TIMELINE_EPOCH=1. Initialization zeroes/pre-touches all storage and writes the
existing header off realtime, then Release-stores epoch 1 before either endpoint
is installed. Attach first Acquires epoch 1 and checks the entire exact initial
header, identity, lengths, platform and reserved words before loading native code.
There is no runtime re-attach of a reset map. Dynamic epoch reads do not reparse
the startup-only header or confuse mutable latency/completion words with layout.
Hello3 rejects old helpers before Load; the exact ABI4 attach check independently
rejects ABI1/2/3 maps before native loading, including a peer falsely claiming a
new Hello.

### Host publication and overflow

One Audio endpoint is the sole reset publisher for a map. Construction requires
fresh epoch 1. reset_timeline has this order:

1. Check current mapped epoch equals the local epoch; precompute checked next
   epoch and next u64 sequence without changing either. The next epoch must fit
   nonzero u32. Sequence numbers are never reused, including partial blocks.
2. Invalidate local completed_proof, acknowledged generation and the shared
   Signals.processed_generation; clear existing adapter/delayed buffers and
   availability using their preallocated storage. Preserve held-note ownership,
   current parameter intent and desired generation. Adopt the checked local
   epoch/sequence and initialize the next local input block. No block is yet READY.
3. Discard only reclaimable expired READY/DONE slots with the existing four
   single-attempt CAS operations. HELPER_WRITE remains untouched.
4. Publish the authoritative epoch **last**, with one strong compare_exchange
   from checked previous to checked next, success AcqRel / failure Acquire.
   This is the reset linearization point. It includes Release publication and
   detects an unexpected writer without a CAS retry loop. Subsequent new-epoch
   READY publication occurs only after this operation succeeds.

The helper's Acquire READY claim also orders the earlier header publication;
therefore a valid new READY block cannot precede its epoch authority. There is
no callback wait, clock, allocation/free, lock, IO, process/native call, Arc
clone/drop or unmap. The added reset work is one bounded load/CAS plus existing
bounded buffer work. Nothing increments desired control generation or supplies
an admission/processing acknowledgement.

Zero, regression, unexpected local/header disagreement, failed reset CAS,
sequence exhaustion or an attempted reset past u32::MAX fails closed: clear
current local proof/availability and Release-latch Signals.failed plus the
existing mapped failure flag. Keep the last published authority; do not publish
zero, truncate, wrap, silently clamp or acknowledge the requested reset. The
existing supervisor reaps off realtime; a manual restart gets a fresh map/session
and epoch 1. A helper can observe skipped epochs when resets happen faster than
its turns; any greater bounded epoch is valid, while a lower acquired authority
than its last observation is corruption. No counter addition changes liveness.

### Helper ownership, processing and idle

Native.epoch becomes the last acquired authoritative epoch. Only a private
adopt_timeline operation updates it; Native::process must stop unconditionally
assigning it from an input block. Adoption updates continuity to None and marks
held-note reconciliation necessary. It does not set processed_epoch,
processed_generation, output proof, parameter readback or owner-completion.

Acquire/adopt authority at the start of each control/audio/idle owner turn and
after native processing, control capture/recovery and idle return. For audio,
after take_input owns HELPER_WRITE, acquire the authority immediately before
admitting that owner turn:

* Input epoch equals acquired authority: the native turn is permitted. Reset
  publication after this check may occur while that turn is in flight; it cannot
  revoke native ownership. Finish on the creating owner, retaining input epoch
  on all output and any actual DSP proof. On return acquire/adopt the latest
  authority before recording current continuity or handling another request.
* Input epoch is older: retire the already-owned input with a helper-only
  Release publication HELPER_WRITE -> EMPTY, after the helper's final payload
  access. Do no native work and publish no DONE or proof. This adds no output
  status. The host cannot perform this retirement on behalf of the helper.
* Zero, out-of-domain or future input epoch: malformed protocol; latch failure
  and use the existing failed completion/termination path. Other identity,
  binding, sequence, transport and event checks remain intact.

For an allowed old turn that finishes after reset, matching successful nonempty
native processing with complete admission and unchanged no-drop evidence may
still record processed_epoch=that old input epoch. It cannot write the header,
roll Native.epoch backward, or label that evidence with the new epoch. Only if
the post-turn acquired authority still matches the input can sequence continuity
be retained. Otherwise continuity stays invalidated for reconstruction by the
next current snapshot. The original output is tagged old and host collection
rejects/discards it. Old DONE cannot restore host proof after reset.

The existing owner completion word 27 advances only after the actual synchronous
native owner/idle turn returns, as before. Reset adoption or stale-slot retirement
alone does not advance it. Watchdog age cannot be erased by READY turnover or
reset/temporary empty slots. An idle/main-thread hang still requires bounded
termination; an actually returned idle is liveness evidence, never DSP evidence.

### Capture authority and cache publication

Helper Native::capture first acquires/adopts mapped authority and requires exact
requested epoch equality plus the existing identity/generation/table checks.
Stale and future requests get a recoverable ownership refusal *before* native
state work; the request itself cannot change authority. Valid same-current-epoch
capture may proceed even when the last native work was older. Its DSP field is
processed_generation only when processed_epoch equals the requested epoch;
otherwise it is zero. Inactive reconciliation remains a separate field.

After CLAP idle/save/readback, or VST3 deactivate/reconcile/save/mandatory
reactivation (including recovery after refusal), re-acquire authority. A changed
epoch rejects the capture, adopts the actual current timeline metadata and does
not send Captured. Native recovery is never skipped by this check. Native bytes
and pending committed intent keep their accepted separation; no proof is inferred
from queue admission, reset, adoption, inactive reconciliation, capture or liveness.

Control::capture also needs a Region clone in its private Inner, acquired off
realtime during launch. Check authority before enqueueing, after receiving and
validating the response, and again under the existing last_state mutex before
publication. On mismatch return the conservative ownership error without
replacing cached state. Preserve request-ID monotonic publication under reversed
caller completions. The final Acquire under this mutex is the successful host
capture linearization point: if reset occurs afterward, the result remains
explicitly stamped with its old epoch; no old proof becomes current. Existing
caller/document generation postchecks remain necessary. A previously valid cache
may remain as the last known state with its old epoch; it is not relabeled.

The extra control Region reference retains the mapping through in-flight calls.
All its allocation, mutex operations and release are control-side. Neither a
reset nor a rejected capture reclaims helper-writable storage or unmaps a mapping
still held by the callback. Process termination must still be confirmed before
control retirement; the final callback/storage references retire off realtime.

### Exact race matrix and required tests

The following matrix is implemented and executed. Private native-owner tests
force the admitted-old-turn and capture/recovery schedules with actual SDK
calls; they are mechanism evidence, not additional process containment evidence.
Private control/slot units force cache/atomic publication schedules. The unchanged
original test and appended bridge cases execute real authenticated processes.

| Forced schedule | Required result |
| --- | --- |
| Capture1; no old turn; publish reset2; capture2; no new DSP | Original oracle: epoch2, DSP proof0, host acknowledgement0 |
| Capture1; claim old epoch1 HELPER_WRITE; publish reset2; actual old native turn completes; capture2 | Same success/proof0 oracle; authority2 cannot roll back; old output rejected |
| Old READY is claimed; reset2 precedes owner-turn admission | Helper retires its own slot without native work, DONE or acknowledgement |
| Capture1 is queued before reset2 but serviced after it | Stale ownership refusal, no native capture or cache replacement |
| Future capture3 while authority2, or epoch0/out-of-range | Refusal without authority mutation or native capture |
| Resets1->2->3 while old epoch1 native turn is busy | Authority3; capture2 refuses; capture3 succeeds with proof0 without DSP |
| Reset during CLAP capture or VST3 inactive capture/recovery | Mandatory recovery, then ownership refusal; last valid cache retained |
| Reset after helper postcheck but before Control postcheck/publication | Host refusal, no cache update |
| Reset after final Control Acquire/cache check | Valid result linearizes before reset and retains its old epoch; no current proof |
| Old DONE arrives after reset and before/after later resets | No old sample/health/getter proof is collected; busy slot was never revoked |
| First new-epoch block actually processes but is admission-incomplete/no-drop unknown | Capture new epoch still has proof0; later complete matching DSP supplies current proof |
| Reset or capture only; idle turns return, or READY slots retire | No desired-generation increment or native DSP acknowledgement |
| Header zero/regression/foreign identity, failed publication, epoch u32::MAX or sequence u64::MAX | Bounded failure/fallback, no wrap; fresh-map restart required |
| Hello2 with otherwise matching key; ABI3 map with claimed Hello3 | First gets zero Load metadata; second rejected before native load; no downgrade |

Deterministic native-owner tests: reset_capture_accepts_current_authority_after_an_old_native_owner_turn;
repeated_resets_fence_old_turns_and_stale_capture_without_new_dsp;
stale_claim_retirement_is_not_native_processing_or_liveness;
reset_during_native_capture_recovers_then_refuses_and_later_processing_works;
native_authority_regression_and_future_inputs_fail_closed. The checkpoint is
after real CLAP capture/readback or before mandatory VST3 reactivation, never a
fake native success. Both formats recover, then refuse changed authority; a
subsequent incomplete block retains proof0 and a complete block supplies proof1.
The existing actual note-overflow/incomplete-new-epoch process regression also
passes unchanged.

Control tests force reset after the reply-side check and while a caller is
excluded by the cache mutex; they also preserve an explicitly old-stamped result
when reset follows successful publication, and retain reversed request-ID fencing.
Slot/adapter tests cover exact word/layout initialization, CAS disagreement,
counter boundaries/corruption, non-revoked busy slots, stale retirement, old DONE
proof rejection and finite delayed fallback. These distinguish private barriers
from executed process IO. No retry, scheduling sleep, product-test skip or weaker
oracle obtains GREEN. The five native units use the existing explicit-fixture
convention; the full --include-ignored verification executes all five plus the
existing sticky-writer unit with no nested Cargo. The integration test's single
ignored auth role is executed as a child by its ordinary passing parent tests.

### Exact implemented host-only source window and consumers

The parent granted, and this increment changes, only:

* bridge/protocol.rs: VERSION4, one header word/domain, exact initial-header and
  full offset/old-version rejection units. Slot transport and offsets stay fixed.
* bridge/slots.rs: initial Release/runtime Acquire, one bounded reset CAS,
  canonical epoch validation, helper-only stale input retirement and private
  layout/ownership/boundary units.
* bridge/adapter.rs: checked reset ordering/publication, local proof invalidation
  and bounded failure; callback allocator and old-output regressions.
* bridge/helper.rs: authority adoption at owner boundaries, process continuity,
  conservative capture fences and deterministic private native-turn tests.
* bridge/supervisor.rs: private Inner Region lifetime, capture/cache authority
  guards and deterministic reversed/reset publication units. No public API change.
* bridge/auth.rs: unchanged-size Hello version3, rejecting versions1/2; existing
  auth candidate/framing/private-bootstrap tests retained and extended.
* tests/bridge.rs: append process regressions; mechanically update the unrelated
  client test's intended current wrong-key Hello from2 to3 and old-version probe
  from1 to2 while retaining explicit old1 coverage. Preserve the entire original
  reset test and existing assertions/fixture behavior.
* docs/plugins/process-bridge.md plus this new diagnostic document: exact ABI4,
  Hello3, ordering, current authority versus DSP proof and evidence/limits.

Compatibility consumers are Config header parsing; Region initialization/attach,
negotiation, submit/claim/receive; Audio construction/reset/set_transport seek and
loop handling/streaming and offline scheduling; helper bootstrap/load/native
owner/control/idle; supervisor launch/capture/cache/retirement; auth candidates;
the standalone windfall-plugin-audio and debug backpressure binaries; and their
protocol/unit/process/allocator tests. Both binaries use the same helper entry
and require no source change. Future desktop helper wiring uses the same host
library; it must ship the matching helper and cannot downgrade. No desktop
runtime, engine, Controller, Session, native facade, fixture/native_thread,
MeterAnchor codec, state format, Cargo/lock or generated artifact edit is needed.
The mapping size, PDC/2B timing, owner identity, R4 native point/drop semantics and
separate processed_epoch gate are unchanged.

The incremental source-only commit directly atop frozen6b contains only these
host/doc/test hunks. It is intended to cherry-pick onto root's accepted host
prerequisite without importing19f production, discovery Describe or P1/engine
changes; parent composition must verify that independence. Existing pins remain
immutable. P1 preparation/retirement-under-guards, full meter render precheck,
production activation, installer/device/licensed corpus/editor/platform gates
remain open.

## Executed repair evidence

* New deterministic VST3 old-owner-turn test: compiled RED at the exact ownership
  error in0.03s; GREEN after repair in0.02s. Original actual process case: GREEN
  unchanged in0.11s. Historical diagnostic remains1PASS/1RED, not rewritten.
* Final host library with separately built fixture and --include-ignored:
  **76PASS/0ignored**,0.37s in the final post-audit run. The default run does not build another Cargo process;
  explicit fixture setup is mandatory for the six native fixture units.
* Bridge integration: **31PASS/1auth-child-role**,6.48s in the final post-audit run. Correct-key Hello1/2 child
  processes receive zero Load bytes. New helper rejects old maps with no Ready,
  exit70; exact Region attach remains before native loading in source. Existing
 48MiB progress/backpressure, state limit, idle/process hangs, note-only overflow,
  and deadlines/cancellation/resource reaping cases remain passing.
* Realtime integration: **20PASS**,2.37s, preserving R4 and allocator oracles.
* Existing bridge callback guard:320calls,0allocation/free, maximum4us/average1us;
  additional reset/corrupt/late-output guards also report0allocator calls.
  These headless wall times are not hardware deadline or CPU evidence.
* The first broad run caught persistent failure restarting the fallback ramp:
  the prior exact dry oracle failed. Repaired transition preserves its current
  ramp; persistent_failure_finishes_the_fallback_ramp_instead_of_restarting_it
  and the original process oracle pass without weakening assertions. One new
  corruption test setup also used a still-current epoch; corrected to a genuine
  regression. Neither is additional evidence of the original CI mechanism.
* Host all-feature/all-target strict Clippy, workspace fmt and whitespace pass.
  Original process test body/oracle is byte-identical (normalized LF) to6b.
  Host-only diff also applies cleanly to copied exact root702 source objects,
  excluding the production19f Describe additions. No root file or build changed.
  Initialization leaves epoch zero until its first Release1, avoiding an earlier
  relaxed1 that attach could acquire. This final audit change passed the full
  library/process/strict/fmt run above.

No new dependency, state format, native fixture/thread, desktop, engine or P1
source changes occur. Earlier licensed/device/editor/installer/other-platform
gaps remain explicit. Current production facade and full meter/P1 integration
remain separate frozen work.
