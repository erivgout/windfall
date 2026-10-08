# T8 live analyzer integration proposal

**Proposal only; no hook, IPC, Session, UI or foundation source grant is implied.**
Foundation review is pinned to
`421bf5ebf2406d660751eb8cbd670527e09b13d7`. The parent reports both R1 Standards
and Spec reviews complete with no hard/spec defects; reviewers inspected source
and primary APIs without executing tests. Foundation execution remains owner
provenance while the parent runs fresh composed checks. This document is a
separate additive
deliverable. It does not amend that checkpoint or claim its synthetic fixtures
are a live product feed. The existing math/transport contract remains in
[ANALYZER-TAPS.md](ANALYZER-TAPS.md).

Read-only observations on 2026-10-08 UTC:

- This worktree's engine/UI baseline is the pinned foundation checkpoint above.
- P1 `gpt/t3-project-preparation-p1` HEAD was
  `6d0773804199faba65f866feda21b8c4ba9a6b9a`. Its **uncommitted proposal**
  `docs/PROJECT-PREPARATION.md` had SHA-256
  `48ed0ed92563faad0e65e568ad8dfec69a737dffeeebed839427fe7285d26e8b`
  on the initial read, then
  `90bc3759ef0a594ce114894e2813c2928b73e5c6301f7ecb49a6f93775c1447d`
  after its expanded exact-answer clarification was read.
  Its snapshot/whole-state preparation/publication lease/retirement API is a
  dependency to implement and accept, not an available production API.
- M1 `gpt/t3-analysis-jobs-m1` HEAD was
  `e3cdbb3f7d04f5024350e10e5044b9ba09249890`; its active UI repair included
  `features/analysis/actions.ts` and `layout/register-actions.ts`. Those dirty
  files were inspected read-only. The proposal preserves M1's action registration
  and analysis-job lifecycle; neither its source nor another owner's changes
  were imported. No new acceptance claim is made for those drafts.

## First integration topology and authority

Offer **two distinct active stereo captures per native Session**, shared by at
most **four authenticated window subscriptions**. Each window subscription selects
one capture at a time; several drawn views in that window share it. Supported
sources initially remain `MixOutput` and `TrackPostFader(TrackId)`. Master track
post-fader is distinct from MixOutput: MixOutput includes apart voices and the
additional output gain. Track IDs are authoritative native typed IDs, never
current UI row positions. A selected EQ view may explicitly use its track's
post-fader PCM; it must label that location, rather than imply an EQ-specific
input/output tap. Effect-position taps require a separate topology grant.

Use the foundation default `N=1024,H=256,PeriodicHann,Q=32,S=2` in this first native
adapter. No UI FFT/configuration promise wider than that is offered. The native
capability response states rate support, bins/hop/history extents and capacities.
Captures are deduplicated only when project, installed plan/stream, typed source,
optional effect observation and config agree. A third distinct source is a
visible `SourceCapacity` refusal, not reassignment of another view's capture.

The two-capture limit supports an actual mixer-selected track and a mix/analyzer
view concurrently. It does not promise all mixer strips simultaneously, arbitrary
bus/plugin locations, or every EQ's independent pre/post spectrum. Those extensions
need their own finite budget and native selection checks.

Proposed native-only typed identities (names are design vocabulary, not exports):

```text
ServiceId, SubscriptionId, ViewRequestId
EngineInstanceIdentity, ProjectGeneration, PlanGeneration, StreamGeneration
CaptureRevision, CaptureTicket, SelectionGeneration, ClockEpoch, EffectOwnerGeneration
TrackId, EffectId, PatternId                     // existing domain ID types
ResolvedTrackIndex                             // private validated index wrapper
```

`CaptureTicket` binds the immutable foundation Selection to the accepted engine
instance, plan/stream generation, rate, resolved source index, optional actual
effect-owner generation, and clock-map revision. `ViewRequestId` identifies one
window's current request; it is not the capture authority. Fanout adds each
subscriber's ticket to the same accepted capture result. A source deletion,
project replacement, stream/rate change, or plan rebind requires a new capture
ticket. Old taps retain their old identity and cannot be relabeled.

Every issued ticket/generation/event sequence uses checked non-wrapping counters.
Native issuance validates the current authenticated window and installed project;
UI source names/generation guesses never authorize installation. Exhaustion gives
a terminal typed refusal and invalidates that service/request. There is no reset
to a reused ID. A new service must have a distinct native lifetime identity; if
its outer identity counter is exhausted, disable admission until process restart.

## Compose with P1 preparation and its future lease

Use P1's exact-candidate borrowing rule. In a future engine preparation extension,
`PreparedPublication` carries at most two `PreparedCaptureBinding` owners beside
its whole prepared PlanState, prepared message envelope and retirement capacity.
Each binding owns the prepared tap/worker/reader, resolved index/latency metadata,
immutable clock-map handles and reserved off-thread cleanup. It is constructed
and, if used, its worker is spawned **before** final guarded acceptance.

Add a checked `CaptureRevision` to P1's captured engine eligibility when source
preferences/bindings affect a candidate. A changed source request during stalled
preparation refuses/retries the stale plan candidate before any Document commit;
it cannot install a candidate that resets the user's newer source selection.
The parent/P1 must approve that extra refusal guard explicitly.

Only the producer and Copy binding metadata move into audio runtime seats.
Workers/readers/thread handles remain in an outer staged activation owner
created before any guards. Final guarded helpers borrow that owner; error,
return and unwind release lease/DocState/recording guards before its destruction.
Reserve native-service activation capacity before eligibility too. After guarded
install, hand its control endpoints to the service with the exact accepted ticket
and perform old-consumer cancellation/joins off locks. An accessor/activation
carrier for this split is an explicit P1 integration extension, not an invented
current API. No worker or reader is hidden in callback-owned PlanState. The
service keeps the new ticket pending until both acceptance and actual PCM are
observed, regardless of which ownership message reaches its owner first.

The intended sequence is:

1. Briefly capture immutable engine/project/request identity under the existing
   recording-before-DocState order and P1's preparation snapshot. Resolve only
   cheap metadata there. Allocate no queues, worker, FFT plan, scratch, clock-map
   copies or retirement storage in this phase. Release all caller guards.
2. On the preparation owner, construct the whole P1 candidate and captures,
   reserve foundation/native quotas, validate source/effect ownership and known
   latency bounds, and reserve publication/retirement capacity. Reject invalid
   generations/config/clock-map capacity visibly. Failed preparation leaves the
   installed engine/capture owners intact; drop unused candidates outside locks.
3. Reacquire the original recording/DocState and other route-specific guards,
   then obtain P1's `ProjectPublicationLease` borrowing **that same token**.
   Validate engine/plan/stream/factory/capture revisions, source identity and all
   original source/loading/request/replacement/history guards. No failure after
   musical Document mutation is allowed. No constructor, allocation, FFT, native
   owner call, IO, cancellation join or garbage collection occurs in the lease.
4. Install one **ordered prepared commit envelope**: project/PlanState and its
   associated capture bindings adopt together. Extend the reserved P1 envelope
   and retirement capacity; do not publish unrelated producer/metadata queues
   and hope their ordering matches. The lease's install remains infallible once
   issued. Until callback adoption and a fresh exact-ticket result, report
   `AwaitingAudio`; successful preparation is not live-source readiness.
5. Leave all recording/library/DocState/controller scopes. Retire displaced
   reader/worker/producer/map ownership through P1's outer `ProjectRetirement`
   carrier. Cancellation, thread join, endpoint destruction and last Arc release
   occur here, including refusal, supersession, detach and device-open failure.
   Full retirement keeps ownership and applies bounded backpressure; it does not
   drop, grow a trash map or detach a cleanup thread under guards.

For **source-only** changes, propose a sibling
`CapturePublicationLease<'a>` that borrows `&'a mut PreparedCaptureChange`, uses
P1's engine identity/capacity/retirement rules, and publishes at most two capture
adoptions through the same ordered engine message ring. It changes no musical
document/history and reconstructs no native effect rack. This is an explicit P1
interface extension to grant, not a call to today's allocating `set_plan`/`attach`.
Dropping an uninstalled lease leaves the borrowed token in its outer scope.

The callback owns concrete tap/binding values in reserved runtime capture seats
associated with its adopted state. It moves displaced producers into P1's
reserved retirement ownership, preserving existing garbage headroom. It never
drops those heap owners. A stale queued capture-change command is forwarded whole
for off-lock retirement, not destructured/dropped on audio. Generation mismatches
can only disable capture; they cannot select a different source or mutate audio.

The foundation `AudioTapSlot` remains a tested standalone ownership interface.
Its independent staging queue is **not** a transactional substitute for P1's
project-plus-binding commit. A live adapter must obtain a reviewed grant for the
prepared envelope/seat composition, instead of racing `TapInstaller::stage`
against a separate `SetPlan`. No fresh callback queue is constructed under
DocState or hidden in a Session registration function.

Detached state has no negotiated capture rate and reports `WaitingForStream`.
P1 off-lock attachment builds fresh complete capture seats and queues before its
guarded link swap. Reopen/rate changes invalidate pending candidates and publish
new stream/ticket identity. Offline N4 render/stem taps remain independent and
are neither used nor edited by this proposal.

## Exact PCM capture points and device-output preservation

At the pinned baseline, `Processor::process_block` takes `base=self.frame`, calls
`Mixer::mix`, optionally collects N4's existing stem taps, then calls
`finish_output`, then advances `self.frame`. `MAX_BLOCK` is 256, so each selected
capture can publish at most one ordinary packet per internal block. Future larger
internal blocks must partition explicitly and advance stamps; no truncation.

**TrackPostFader:** immediately after `Mixer::mix` returns, borrow only the
accepted `ResolvedTrackIndex` buffer over `[..out.len()]`, before existing stem
collection/output finishing and before the next `Mixer::clear`. That retained
buffer already contains the real routed inputs, native/built-in effects, fader
and pan. Outgoing routing reads/copies it. It has neither output gain nor device
scrub. The index was resolved against the exact prepared plan; callback validation
uses generation/index bounds, not document lookups or a UI mixer index.

**MixOutput:** after master plus apart summation and the per-frame output-gain
ramp, **before the device scrub**. Publish the unsanitized post-gain output slice
with the actual `base`, rate, source ticket and clock epoch. Capture is a copy;
failure never changes the device samples or stops audio processing.

The current `finish_output` checks input finiteness **before** multiplying gain:
its legacy result is `finite(input) ? input * gain : 0`. A naive split into gain
then `finite(product) ? product : 0` changes the finite-product-overflow case.
The narrowly proposed equivalent observation route is:

```text
1. In the gain pass, retain each input's invalid bit in a fixed 512-bit mask.
   Calculate the same ramp gain at base+offset; write input*gain to out for both
   finite and nonfinite inputs. No clamp/substitution at the observation point.
2. Publish that post-gain PCM before scrub, at most 256 stereo frames.
3. Set only the originally invalid samples to zero using that mask.
   Originally finite samples retain exactly the legacy multiplied product.
```

The 64-byte mask is prepared inline with the processor/runtime candidate outside
locks; its lifetime adds no callback heap owner. With no active MixOutput capture,
the original one-pass finisher can remain. NaN/Inf and finite multiplication
overflow are reported truthfully by the tap, including when output gain is zero.
This route preserves the baseline device results, including its overflow behavior;
changing the device's overflow scrub policy needs a separate explicit fix/grant.
No native-hook success is claimed before bitwise output-equivalence tests execute.

Requested callback ordering: handle accepted plan/capture envelopes, apply any
clock-discontinuity reset, render/mix, capture selected track(s), apply gain,
capture mix, device scrub, advance/publish transport. No capture allocation,
destruction, lock/wait/IO/FFT/transcendental or native-control call is introduced.
Existing N4 plugin boundaries, T1 transport/bar helpers and PDC adoption order
remain owner-controlled.

## Nonconsuming actual gain-reduction observation

The current `GainReductionMeter::take_db()` swaps/reset its atomic maximum, and
`Controller::frame()` is its existing consumer. Calling either for an analyzer
would steal observations. Adding a plain peek of that held maximum would still
give the existing reader's time horizon, rather than this audio block's reduction.
Neither is the proposed block observation.

Request an E1/E2-owned optional **processor-local Copy observation** of the
already calculated block maximum (e.g. compressor's `deepest`), forwarded through
AnyEffect/EffectSlot/EffectUnit without touching the existing atomic consumer.
No new logarithm is computed by capture. Proposed typed observation:

```text
ActualEffectObservation {
  effect: EffectId,
  owner_generation: EffectOwnerGeneration,
  first_frame: u64, end_frame: u64,
  kind: InternalProcessedBlockMaximum,
  reduction_db: finite 0..=120,
}
Option<ActualEffectObservation>       // absent is different from valid zero
```

Bind at most one optional effect target per capture, verified as the active
owner in the selected track/master chain. Bypass/no processed block/missing
observation/invalid metadata gives an explicit absent reason. Native-plugin GR
is absent until N4 exposes an actual supported observation; this proposal changes
no bridge ABI. Internal effect-stage reduction is not the net attenuation of a
wet/dry mix, whole chain, master plus apart voices, or post-fader amplitude.

Existing foundation per-window GR can receive a constant block observation only
with its block horizon disclosed; it is not a per-sample trace. A later scrolling
GR implementation must retain at most 128 typed block observations with exact
frame ranges, overwrite/drop counts and source/owner continuity, and break the
trace on gaps/replacement. Foundation's current window maximum alone does not
complete the limiter's scrolling-history acceptance.

## Audio frames, musical ticks and latency status

Wire clocks retain native u64 frame precision as canonical **decimal strings**
(at most 20 digits); generations, counts and sequence IDs use the same encoding.
Signed PDC frame labels use canonical signed decimal strings. JSON/JS numbers
must not round absolute frames above 2^53. Frame position is absolute within its
explicit StreamGeneration/ClockEpoch, not wall time or musical tick.

Proposed clock envelope:

```text
AudioClock { stream, epoch, firstFrame, endFrame, sampleRate }
MusicalClock = Known { mode, pattern: Option<PatternId>, playing,
                      firstTick, endTick, tempoMeterRevision, clockRevision }
             | Unknown { reason }
PdcLatency = Known { sourceFrames } | Transition { referenceFrames }
           | MixedPaths { masterReferenceFrames } | Unknown { reason }
DeviceLatency = KnownEstimate { frames, observationRevision }
              | Unknown { reason }
```

Known musical positions come from an actual T1-owned sequencer/tempo/meter
snapshot at the tapped frame, not the UI's latest RealtimeFrame or frame/rate/BPM
arithmetic. The baseline sequencer Clock is affine in counted ticks, but song
position also uses pass/loop state and Plan.unwarp. A packet's first/end tick
interpolation cannot supply arbitrary interior FFT-window boundaries under a
tempo map. Bars/beats use T1's checked meter segments/helpers; this proposal does
not expand T1's currently limited host-event/test repair grant.

For a later T1 grant, copy a bounded clock descriptor at the **start** of the
actual sequencer span before advance changes pass/play state. It contains raw
anchor frame/tick/frames-per-counted-tick, pass origin, mode/pattern/play/loop
state, immutable prepared tempo/meter map revision and exact span frame limits.
Use at most eight inline span descriptors per 256-frame publication; worker
does all map inversion/label conversion. Cap retained map segments at 4096 per
map, prepared off locks; over-capacity maps yield `MusicalClock::Unknown(MapLimit)`
rather than truncated bar evidence. Discontinuities split/reset the PCM segment;
if span storage overflows, refuse that whole capture packet, count
`clockSpanDroppedFrames` and invalidate continuity. Playback still processes the
entire block. No new callback transcendentals or allocated span list is allowed.
This extra metadata/storage requires a new reviewed analyzer-context grant;
the frozen foundation Stamp currently does not contain it. Charge all retained
descriptor/map storage in the revised prepared layout under the same process
32 MiB quota. Copy only the bounded tempo/meter metadata off locks; do not retain
a whole obsolete Plan/project asset graph merely for analyzer labels. Until it lands, report
musical clock unknown and offer only honest frame/time labels.

Track PDC references the accepted state's source-specific `behind[index]`.
Stable master reference is state.latency, derived from actual negotiated DSP/native
latency, including N4's bridge pipeline. It is not a guessed plugin setting.
Use `Transition` through adoption/history priming until the owner proves stable
alignment. MixOutput may contain apart tails bypassing the current master path;
report `MixedPaths` when a single lag does not describe every audible contributor.
Do not subtract a master reference and label the result exact heard time for all
mixed paths. Foundation's scalar pdc_frames is a supplied reference label; the
native wrapper must carry the stronger status before promising presentation time.

Device status/configured buffer size alone does not establish actual device
latency. `KnownEstimate` requires the native device owner's timestamp/measurement
source and revision; otherwise it is unknown. PDC and supplied device estimates
derive signed frame/time labels off thread. Neither label is a physical speaker
clock. If truthful latency metadata is outside the foundation's supported
one-second/rate bounds, refuse capture with `LatencyUnsupported`; never clamp it
or silently call it zero.

## Proposed native payload and bounded subscriptions

Add a separate live-analyzer DTO/channel family rather than FFT data to every
legacy RealtimeFrame. This keeps `Controller::frame()`'s single consuming meter
reader unchanged. Proposed operations, all unimplemented:

```text
analyzer_capability() -> NativeAnalyzerCapability
analyzer_select(subscription, request, source, optionalEffect) -> SelectionStatus
analyzer_subscribe(window, request, channel) -> SubscriptionStatus
analyzer_ack(subscription, request, captureTicket, sequence)
analyzer_unsubscribe(subscription, request)
analyzer_status(subscription, request) -> bounded current status
```

Capability states native availability, stream readiness, exact config/topology,
quota use and unsupported reasons. Subscribe/select accepts only authenticated
native windows. Status is `Preparing`, `AwaitingAudio`, `Live`, `WaitingForStream`,
`Stalled`, `Retiring`, `Unavailable`, or `Refused(reason)` with exact tickets.
`Live` requires a fresh accepted PCM snapshot. No mock/sim zero frame establishes
native availability; a browser backend may return `Unavailable(DesktopRequired)`.

Each result has schema version, exact service/subscription/view/capture tickets,
project/plan/stream/selection/clock generations, checked event sequence, [first,end)
clocks and latency status, current drop/invalid/reset counters, calibration and
config, channel validity/clip/levels, stereo statistics, 513 standard spectrum
bins, at most eight logical spectrogram slices, 128 envelope bins and 128 vectors.
Optional future GR history is at most 128 observations. Raw `2N` PCM remains
native; this first payload advertises min/max waveform, not lossless oscilloscope
samples. A faithful oscilloscope raw-data mode is a separate bounded payload/view
gate, not an inferred waveform from envelope centers.

Proposed exported names are `AnalyzerCapability`, `AnalyzerSelectionRequest`,
`AnalyzerSelectionStatus`, `AnalyzerEvent`, `AnalyzerAck` and
`AnalyzerUnsubscribe`. `AnalyzerEvent` is a tagged `Status` or `Frame` envelope.
`AnalyzerFrame` groups `AnalyzerIdentity`, `AudioClock`, `MusicalClock`,
`PdcLatency`, `DeviceLatency`, `AnalyzerCounters`, two `AnalyzerChannelReading`
values, `AnalyzerStereoReading`, calibrated `AnalyzerSpectrumBin` values and
the bounded waveform/spectral/vector/optional-GR summaries above. The request
carries its expected native project/stream generation; the response carries the
server-issued accepted capture ticket. Bounds apply at both encode and decode.
These are new DTO proposals for the owning IPC grant, not hand-edited generated
TypeScript or additions to M1's analysis-job DTOs.

Finite scalars are JSON numbers; silence, invalid channel/bin/phase and unavailable
statistics are explicit tagged states/null plus validity. Preserve invalid counts
and waveform-bin missing min/max. Do not stringify NaN as healthy zero, skip DC or
Nyquist, invent a finite dB floor, or infer spectrum/GR from filter settings.
Source captions come from current native metadata and are checked again at UI
delivery; captions never replace typed identity. Cap copied caption/reason text
at 256 UTF-8 bytes. A longer name is explicitly omitted with `NameOverCapacity`
and the typed track ID shown; it is not silently truncated or cloned unboundedly.
Untrusted input must pass byte/identifier bounds before entering retained service
storage; framework request parsing is not claimed covered by that storage quota.

Proposed additional limits, all exposed by capability/status:

| Resource | Bound | Refusal/drop lifecycle |
| --- | --- | --- |
| Active captures / pending preparations / retirement backlog | 2 / 2 / 2 per Session | Busy/capacity refusal; no extra candidate until old retirement drains |
| Foundation retained taps | Existing process max 8, 32 MiB charged | Existing quota errors preserve installed path; no quota release at cancellation alone |
| Native window subscriptions | 4 | Fifth returns SubscriptionCapacity; closing removes only its exact ticket |
| Control requests | 8 bounded pending commands, maximum 1024 encoded bytes each | Queue/size refusal, exact candidate retained off locks; no unbounded job map |
| Result encoding | 512 KiB per event, prepared bounded writer | Whole result refused/count PayloadLimit; no arbitrary spectrum/history truncation |
| Per-window delivery | 1 unacknowledged event + 1 pending event | Replace/coalesce pending event with downstreamSkipped counter and continuity metadata |
| Controlled native wire buffers | 9 prepared 512 KiB buffers, process-wide charged budget 8 MiB including service overhead | WireByteLimit before subscribe/select; actual requested layout checked before commit |
| Emit frequency | At most 30 events/sec per window; status shares the same credit | No catch-up burst or unbounded status backlog |
| Unacknowledged timeout | 2 seconds | Stalled, invalidate window request, release subscription outside locks; no repeated sends |
| Audio freshness | Snapshot older than 500 ms off-thread receipt age while stream is expected active | Mark Stalled/clear healthy display; require a new exact-ticket epoch before Live recovery |
| UI spectral history | 32 slices, each 513 stereo power bins | Discard oldest with explicit age/drop/gap metadata; no interpolated missing slices |
| UI retained numeric buffers | Prepared/charged at most 512 KiB per subscribed window | Client refusal; retain no unbounded JSON-result array |

These budgets are additional to the foundation and need measured layout/encoder
checks in implementation. The nine wire buffers cover two per window plus one
shared encoder; account candidate/service objects in the 8 MiB admission charge.
The framework's own channel copies/JS object/RSS bookkeeping are not an asserted
8 MiB process-memory ceiling. Credit-gated sends ensure at most one outstanding
analyzer event per window enters that transport. Audit/measure any additional
framework buffering; do not label a naked Channel.send call bounded delivery.

The service owns/drives bounded subscription state off DocState. Stage/drain only
cheap ownership/metadata under any small service coordination lock; encoding,
sending, cancellation and joins occur after guards. It must not reuse today's
Session realtime subscriber-send-under-lock loop for heavy analyzer payloads.
Native status/progress events and result events share acknowledgement credit.
Ack must match subscription, request, capture ticket and sequence; late acks
cannot revive old ownership or clear a successor's pending event.

Each delivered payload reports source input drops, stale queued frames, analysis
snapshot drops, serialization refusals, downstream coalesced events, invalid PCM,
resets and observed counter saturation independently. Replacing a pending envelope
breaks the downstream display trace; never connect two delivered spectrogram/
waveform/GR points over a lost interval. Foundation counters saturate rather than
wrap; observing `u64::MAX` gives terminal `CounterExhausted` and cancellation,
not a plausible healthy frozen count. New service/event counters refuse before
overflow. Stream epoch exhaustion is similarly terminal.

Project/source replacement clears display authority immediately, invalidates old
view requests, cancels superseded preparation off locks, and uses a fresh ticket
for rebind. Losing the final subscriber stages bounded engine clear/retirement;
remaining users of a shared source keep their own valid tickets. Worker stall
already causes input overrun/reset; native/UI stall status must also invalidate
healthy held evidence. Recovery either stages a prepared new binding or uses an
explicit ordered reset/ack with a new epoch. It cannot simply redraw the old
snapshot as Live. Preparation/retirement limits can refuse recovery visibly while
keeping exact ownership retained. No synchronous retry loop under DocState.

## UI registry seam and actual views

Use new `features/analyzers/actions.ts` and the existing `lib/actions` registry
pattern: `analyzers.open`, `analyzers.selectSource`, `analyzers.close`, and distinct
view actions. Register their single disposer through the narrow future
`layout/register-actions.ts` window **alongside** M1's `registerAnalysisActions()`.
Preserve all `analysis.*` model/job/review/apply/cancel/cleanup actions and M1's
identity-checked panel disposer. Live PCM viewing does not inherit Analysis's
model-availability prerequisite or call its submit/apply endpoints.

Source selection has a labelled keyboard-operable control, honest MixOutput versus
typed track choices, pending/refusal/stall/invalid/unknown-latency status, and
focus/close/unsubscribe behavior. Capture a local project/view request before
awaiting native selection; reject mismatching replies at the final draw/store
boundary, including closed/replaced windows. The native lease remains authority.
Project replacement/source deletion clears numeric buffers and drawn evidence,
increments the checked UI request, and invalidates registry action state.

Keep high-rate data outside React state, following `lib/store/realtime.ts`'s
requestAnimationFrame pattern, with separate bounded analyzer buffers. React
state can hold low-rate source/capability/error state. Mount/unmount/share one
window subscription; disposing an old view affects only its own request. Native
data stays ticket-checked even if a view keeps a late decoded event.

Before adding a docked panel, request the exact `PanelId`/PANELS/workspace layout
window; do not silently expand existing UI saved-state migration. Distinct big
clock, dB meter, spectrum and time/spectrogram/vector modes must implement their
actual promised behavior. EQ overlay consumes explicitly selected actual PCM
separately from its parameter-derived response, with location/invalid/gap labels.
Mixer waveform uses actual selected-track min/max data at its displayed frame
range. GPU/canvas/theme work uses existing renderer seams entirely off audio.
No cosmetic panel, registry action or authored fixture earns product parity.

## Exact requested serialized grants and acceptance gates

| Next window | Concrete change requested | Owner/dependency boundary |
| --- | --- | --- |
| Analyzer context module/tests after R1 acceptance | New bounded capture ticket/context/DTO adapter tests; any Packet/Stamp context extension requires a fresh foundation grant/review | T8; keep 421bf5eb immutable |
| P1 prepared payload/lease/retirement | At most two prebuilt capture bindings, CaptureRevision guard, source-only borrowed lease, ordered message/retirement capacity | P1 controller/state/message/preparation windows; no constructor under locks |
| Processor observation | Reserved capture-seat adoption, track borrow after mix, post-gain pre-scrub observation with invalid mask and exact frame stamping | Narrow T1/P1-coordinated processor window; no unrelated transport/native-bar repair |
| Clock observation | Bounded Copy descriptors of actual sequencer spans and immutable checked map association | Separate T1 grant; current host-event/tests repair is insufficient permission |
| Actual GR observation | Nonconsuming processor-local optional block maximum with owner identity, horizon and validity | Separate E1/E2 effect/rack/DSP grant; native GR stays absent without N4 grant |
| IPC/native service | New live analyzer DTO/service/commands/backend subscribe+ack; minimal generated registration by owning parent | M1-coordinated Session/IPC/backend window, explicitly separate from current UI repair |
| Actions/view registration | New analyzers feature; one registry registration/disposer preserving M1 actions; separate panel/layout window only if needed | M1/T8 serialized UI grant |
| EQ/mixer drawing | Actual PCM spectrum overlay and selected-track waveform, accessible source selection/status | Subsequent narrowly owned EQ/mixer windows |

N4's ABI3/runtime/render/stems/factory windows remain reserved; no edits, native
capture selection or helper ABI additions are requested there in this first hook
packet. The current grant is this new document only. No root/generated files,
installs or broad source changes are part of it.

Required **future executed evidence**, on freshly composed owner checkpoints:

- Real engine PCM from distinguishable mix/track fixtures, output-gain ramps,
  apart voices, fader/pan/routing, stereo phases, invalid input and finite product
  overflow. Captured mix is unsanitized post-gain; device output remains bitwise
  equivalent for irregular partitions 1/7/64/256 and larger device buffers.
- Allocator guards across real adoption/capture/reset/full/refusal/retirement
  paths; deallocation observers and checking destructors prove all candidate/
  worker/last-handle destruction outside recording/DocState/controller locks.
- Barrier-stalled P1 construction while source requests/project edits/replacements,
  rate/stream/factory revisions change; original document/history/recording/source
  and capture authority remain exact. Test the final acceptance-to-adoption race
  with actual pending envelopes, not only version arithmetic.
- Authentic effect reduction independent of amplitude; consuming existing meter
  snapshots with and without analyzer leaves their peaks unchanged. Missing/native
  unsupported/bypassed/invalid/mixed-owner observations never become healthy zero.
- Integer frames beyond 2^53, musical seek/loop/tempo/meter changes and frame-span
  bounds, negotiated native PDC/transition/mixed tails, device latency unknown and
  measurement revision. Labels derive from actual stamps/maps, not fake clocks.
- Saturate source/candidate/retirement/subscription/control/byte/encoder/ack/UI
  capacities; stall worker and real webview, close/reload windows, delete track,
  replace project, exhaust generation/sequence/counter seams. Prove finite queue
  retention, counted gaps, stale refusal and joined shutdown with current tickets.
- Accessible source/action/view interactions, EQ live-spectrum versus response
  distinction, mixer waveform frame ranges, registry enable/disabled state and
  stale disposer protection. Run fresh native source-selection smoke tests after
  parent-owned registration/generation, not cached artifacts or mock PCM.

No Cargo, UI/native build, screenshot or hardware/listening check is needed for
this proposal/label-only follow-up; none is claimed. The only existing-source
change renames the ignored CPU case's output label to
`audio_seconds_per_elapsed_second`, with the matching foundation-doc sentence.
No algorithm, API, measurement or executed test result changes. Verification is
exact source/proposal reading, owned rustfmt and restricted diff/whitespace checks.
Full T8
big-clock/dB-meter/spectrum/Wave Candy/mixer-waveform rows, EQ live-PCM audit,
oscilloscope raw mode and limiter scrolling-GR behavior remain open until their
actual integrated/native/view checks are accepted. Headless foundation throughput
continues to mean only the already reported measurement, not a device deadline.
