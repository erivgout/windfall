# Project and native preparation: P1 interface proposal

Status: **proposal only; production lock ownership remains open**. This file is
the first isolated deliverable, not an implementation or a new acceptance claim.
No existing source, generated bindings, artifacts, parity files or release
records were changed. No Cargo/build/test command was run for this design stage.

The isolated branch is `gpt/t3-project-preparation-p1`, fixed at
`6d0773804199faba65f866feda21b8c4ba9a6b9a`. The authoritative root was read at that
same SHA. Read-only draft observations on 2026-10-08: T1 worktree HEAD
`c5448921960ead05d0f0d4846fc627fbcf035fc6`, M1 HEAD
`e3cdbb3f7d04f5024350e10e5044b9ba09249890`, and N4 HEAD
`1fa5a16de85e90e7a8fbc123ff29ba6c413156c3` with active uncommitted bridge wiring.
The parent subsequently confirmed T1/M1 as frozen clean source checkpoints; T1
follows `94e168ae`/`a8`, M1 follows `f006`/`89`. N4's public anchor is prerequisite
only; its ABI3 source remains N4-owned. None was imported here. The parent must
give the serialized file windows before implementation.
No applicable `AGENTS.md` was found in the worktree, authoritative root or their
checked ancestor directories.

## Source finding and required outcome

The current `controller.rs:23-32` `PreparedProject` contains a `Plan` and sampler
pool. `try_prepare_project` prepares sampler banks; `compile_prepared_project`
compiles asset/routing/sequencing metadata. Neither constructs `PlanState`.
`set_prepared_project` calls `set_plan`; `set_plan` holds `Controller::State` while
`PlanState::build` calls `plugins::prepare`, builds effect/instrument racks and
constructs compensation storage. `attach` independently does the same under that
mutex, including `Processor::with_queues` allocations.

Session `edit.rs` calls these routes while holding the document: `push_project`,
`publish_with_prepared`, ordinary dispatch, native-update dispatch, native capture
commit and automation creation. `mod.rs::refresh_plugins` holds recording and
State while retrying the factory and pushing the project. Already staged sampler,
clip, editor, history, replacement and slice routes still construct native/DSP
state at their final locked publication. N4's actual `Runtime::bridge_audio`
synchronously invokes its owner, launches/authenticates/loads the helper and
returns its negotiated facade. Its constructors are not harmless cheap calls.

`State::maintain` also executes `while link.garbage.pop().is_ok() {}` under the
controller mutex. Attach/detach/suspend replace links and clear queued messages
under that mutex. A returned `Garbage::State`, a queued `SetPlan`, or a last factory
reference can own native teardown work. Moving construction alone leaves this
second defect, including when a Session guard is outside the controller call.

The target is one deep preparation module: capture cheap immutable inputs;
prepare every processor/FFT/asset/compensation allocation on the caller's worker;
accept only that exact engine snapshot before any musical document mutation;
publish using reserved storage; return all displaced/cancelled ownership to a
scope after caller guards. Callback adoption continues to move concrete owners
and reserved history without allocating, freeing, locking, waiting or IO.

This applies the common roadmap section 3, N2/N4, `WINDFALL_PLAN.md`, and
`ARCHITECTURE.md`. It preserves `UTILITY-REPAIRS.md` R1-R6, especially R6's actual
attachment ledger; it is not a native-only movement presented as whole-engine
completion.

## Proposed public interface

Keep sampler-only compatibility entry points, but make the production Session
use the following four operations. Names and ownership below are the requested
implementation interface, subject to the parent's exact grant.

```rust
impl Controller {
    pub fn preparation_snapshot(&self) -> PreparationSnapshot;
    pub fn publication<'a>(
        &'a self,
        prepared: &'a mut PreparedPublication,
    ) -> Result<ProjectPublicationLease<'a>, PublicationRefusal>;
    pub fn take_retired(&self, into: &mut ProjectRetirement);
}

impl PreparationSnapshot {
    pub fn prepare(
        self,
        project: &Project,
        pool: &SamplePool,
    ) -> Result<PreparedPublication, ProjectPreparationError>;
    // private counterpart for already prepared sampler/clip pools
}

impl PreparedPublication {
    pub fn sampler_pool(&self) -> &SamplePool;
}

impl ProjectPublicationLease<'_> {
    pub fn install(self) -> ProjectRetirement;
}

pub enum ProjectPreparationError {
    Sampler(SamplerPreparationError),
    Native { target: PluginTarget, reason: String },
    Unsupported(&'static str),
    IdentityChanged,
    MemoryLimit { required: usize, limit: usize },
}

pub enum PublicationRefusal {
    WrongController,
    StalePlan,
    StaleStream,
    StaleFactory,
    UnresolvedProgress,
    QueueFull,
    GenerationExhausted,
}
```

`ProjectRetirement` is an owned, explicitly drained control-side carrier,
allocated/reserved outside guards. It owns displaced plan/ledger handles and
values drained from existing queues. It is not a static registry or new
unbounded shared trash queue. `take_retired` moves ownership into caller-provided
capacity; it never destroys returned native owners itself. The caller drops the
carrier only after State/recording/library/controller guards have ended.

The lease borrows the candidate for validation. A refusal therefore does not
consume or destroy it. The lease retains that exact mutable borrow; `install`
cannot accept a different candidate. Its `install` is the sole engine commit:
it takes the prebuilt fields out of that borrowed token and is infallible after
a lease is issued. The emptied token remains owned in the caller's outer scope.
All possible errors, queue capacity checks
and generation overflow checks precede the lease. The lease is short-lived and
does not compile, hash bindings, allocate, collect garbage, call a constructor,
service a native owner or join a helper. It must not contain a hidden candidate
whose destructor runs under its own guard.

`ProjectPreparationError` distinguishes sampler preparation, native preparation,
unsupported preparation and native-identity drift during construction.
`PublicationRefusal` distinguishes stale engine snapshot, factory revision drift,
unresolved progress reservation, publication capacity exhaustion and generation
exhaustion. Each carries a truthful reason suitable for Session's existing
`String`/warning routes. These are not all converted to `SamplerPreparationError`.
The existing sampler-specific accessor remains sampler-specific; a separate
general preparation/refusal accessor serves convenience calls that cannot
return a Result without breaking their interface.

T1's meter precheck and private refusal helper remain intact, at the start of
`try_prepare_project`, `set_prepared_project` and `set_plan` respectively. Preserve
the exact legacy latch
`Unsupported("Invalid song meter map; project preparation refused.")`, the typed
`Plan.meters: Result<Vec<MeterSegment>, MeterMapError>`, and the unchanged installed
plan/pattern/transport on invalid maps. The new production stage checks this
before native work. A general error can retain the typed map cause while the
existing accessor still exposes T1's exact legacy value. Native/stale errors
must not masquerade as that sampler/unsupported latch.

## Captured types and phases

Private `EnginePreparationIdentity` contains an Arc ownership identity for the
controller instance (validated by pointer equality, with no static registry),
checked non-wrapping plan-publication generation, stream generation, attached rate
and block capacity (or explicit detached state). Transport sequence, musical
playhead and elapsed fade counts are deliberately not freshness identities.
Ordinary play/stop/query operations must remain usable while construction stalls.

`PreparationSnapshot` owns `Arc<Plan>` and `Option<Arc<Ledger>>`, plus the identity
above. Change `State.hosted` to an Arc of the immutable ledger so capture clones
handles without cloning HashMaps, meters or delay-stage vectors under the mutex.
It contains metadata, not an owner handle or mutable `PlanState`. The ledger's
active and departing records stay distinct. R6 `native_identity` continues to
use the identities actually prepared or reused, not a mutable factory's current
revision or the older plan's compilation snapshot.

Private `PreparedPublication` holds that expected identity, the frozen factory
stamp, `Arc<Plan>`, the prepared sampler pool, and either an attached payload
(`Box<PlanState>`, `Arc<Ledger>`, prebuilt commit messages and retirement slots)
or an explicit detached payload. Its fields use owned optional envelopes so
install can move them out without running their destructors. The captured plan,
ledger and provider references go into returned retirement ownership too; token
bookkeeping is not an excuse to drop the last old handle under a lease.

Candidate `FactorySnapshot` records the candidate provider Arc, stable provider
identity and preparation revision, separately from the committed provider's
live revision. Bindings and their fingerprint are already candidate-owned.
For staged New/Open, the prepared revision and the installed runtime revision
are different values until the existing `install_document` step; do not reject
a valid staged replacement by comparing those two as if they were equal.
Capture the old committed runtime revision as an independent refusal guard.
`same_sources` deliberately excludes providers, so it cannot substitute for this
check. Factory identity/revision reads used during acceptance must be documented
bounded metadata operations; a user-defined blocking trait implementation is
not permission to perform native calls while guarded.

N4 read-only confirmation: its `revision()` calls `preparation_revision()` (the
staged `Option<u64>`, otherwise one Relaxed atomic load); provider identity is
the shared revision Arc pointer. Those reads perform no IO, lock, clock, wait
or allocation. `install_document` only publishes the installed revision.

Preserve the existing native identity calculation exactly:
`plugins::identity(binding) ^ factory.revision().rotate_left(17) ^ factory.provider_identity()`.
The immutable ledger adds the actual rate-sensitive preparation/latency metadata;
it is not replaced by the current factory Arc. N4's accepted facade also retains
its actual approved binary stamp and playback/render role. The returned
HostedEffect/HostedInstrument boxes are prepared at the accepted rate and engine
`MAX_BLOCK` (256 frames), with the separately negotiated helper pipeline latency
(N4's current helper block is 256 frames and its two-block pipeline adds 512
frames), before becoming eligible. They remain
unselected until installed
`Audio.transport`; native capture never selects the candidate. Private render
factories return `None` from `render_factory()` to keep the independent render
error channel instead of creating another provider around it.

N4 confirms candidate `Audio::drop` marks `alive = false` and CAS-clears only
its own token; it cannot clear a different installed selection. ProcessAudio
buffers free on that control caller. Runtime maintenance removes registry
Record/Control and reaps off RT; endpoint resources remain until confirmed exit.
These facts permit deferred candidate destruction after guards; they do not
permit dropping facade buffers under guards or treating a candidate as selected.
Adoption/process do not select the token; only installed `Audio.transport` does.

1. **Session capture.** Brief recording-before-State exclusion takes the existing
   document, source/loading/request/generation/edit/replacement guards and the
   engine snapshot. Build/validate a private document candidate using the actual
   checked command/history action. Existing source overlays, recovery IDs,
   browser root/file pins, sample-directory and review tokens remain separate
   guards. Nothing live is dispatched, loaded, pruned or installed yet.
2. **Worker preparation.** Release State, recording and library final-commit
   guards. Prepare sampler banks and playlist variants under their existing
   budgets/cancellation policy; compile the plan; freeze candidate native
   identities; reserve departure/adoption metadata; construct the entire
   `PlanState` and returned ledger from the immutable captured ledger.
   Box/Arc envelopes, queue message, indexes, meters and retirement capacity are
   constructed here. Preparation of a held owner is a metadata reuse decision:
   its constructor is never called, and no speculative duplicate is captured.
3. **Final acceptance.** Reacquire recording, caller-specific library guard,
   State, then the controller publication lease. Recheck all original Session
   guards and engine/factory identity, plus reserved storage/capacity. Only after
   both authorities accept may the caller swap its validated candidate document,
   history and pool handles or consume its review. A refused candidate remains
   in an outer ownership scope. No stop/seek, progress cancellation or runtime
   revision installation is performed on the refusal path.
4. **Commit and report.** Install the candidate through the lease using prepared
   storage. For New/Open, the required stop/transport/seek messages are reserved
   in the same commit envelope; do not call locking Controller methods while
   the lease already holds the controller guard. Commit document parameter
   metadata through N4's existing hook and keep its adoption-before-dedup order.
   Release the engine lease before transport queries/events. Preserve Session
   event ordering under State. Put displaced Session pool/document/factory
   handles in an outer retirement carrier rather than dropping last handles
   during a swap.
5. **Retire.** Exit State, library and recording scopes, then destroy unused
   candidates and the retirement carrier on the control caller. Existing
   realtime/control maintenance explicitly drains garbage outside State so
   stopped playback also retires owners. A destructor may synchronously join;
   that is why moving it after the controller mutex alone is insufficient.

Snapshot counters use `checked_add`. Exhaustion is an observable refusal before
commit. No newly invented detached thread performs final ownership release.

## Whole-state construction and fallibility

Choose whole `PlanState` construction off-lock. A native inventory alone would
still leave `EffectUnit::build` (`AnyEffect`, `EffectSlot::prepare`, meters),
`InstrumentUnit::build`, `ExternalEffect::new` (dry latency storage and parameter
arrays), `Layout::of`, `DelaySlot::seat`, `Compensation::new/with_path`, histories,
stage tap work, ramps, activity and automation vectors under the lock. E3 adds
real delay/reverb processors to that same construction. Future FIR/FFT assets
belong in this worker phase, not a new publication callback.

`PlanState::build` gains a fallible production counterpart taking the captured
factory identity and held ledger. Private `plugins::prepare` returns native
construction errors instead of silently `.ok()`-discarding them. Read the
revision before and after all constructors and use the frozen value to build
records. If it changes while a constructor is blocked, refuse the candidate
rather than recording one revision around an owner made at another revision.
Partially built owners/unused native entries stay in the worker-owned result or
are destroyed in that off-guard worker scope. Never return a placeholder as proof
that a failed helper was successfully prepared.

Missing-provider/binding behavior remains an explicit policy: retain opaque
bindings; absent providers have dry effects/silent instruments with a visible
unavailable reason. A constructor failure must be reported as a native failure;
do not infer a safe missing-plugin exception by parsing arbitrary error strings.
The concrete production rule is: `factory == None` produces an explicit
Unavailable binding; `factory.effect/instrument == Err` refuses with Native.
Answer 4 specifies the private result representation. Offline exports retain N4's independent
provider/error channel and never borrow/capture a playback owner.

PDC is calculated only after actual native preparation returns its negotiated
latency (including the bridge pipeline). Existing compensation bounds remain;
out-of-bound latency is a visible native refusal, not a clamped success that
pretends the graph is aligned. Never invent an asynchronous startup readiness
flag or assume zero/one-block native latency before negotiation.

## Audio progress and late adoption

Moving `keep_leaving` earlier unchanged is unsafe. Its `life.heard()` filters are
time-dependent: an unheard pending owner can become heard while the worker
prepares, and a heard departure can finish. Freezing removal counts is also wrong:
`State::take_over` currently reads the concrete adopted predecessor's
`removal_remaining`, then adds only that remainder and the new owner's priming.
That operation must remain at adoption. Continuous frame progress is not a reason
to restart a fade or reprepare/recreate a retained native owner.

Proposed internal seam: a preallocated `DepartureReservation` in the plan/state
adoption metadata, with at most the captured outgoing and captured active source
generation as alternatives for **one outgoing seat**. These are definition/life
metadata references, never additional native owners and never nested reservation
lineages. It reserves the possible late-heard active source rather than freezing
its current unheard status. `PlanState::take_over` chooses the actual eligible
concrete old seat by generation and owner-local heard/splice state at adoption;
completed/unheard alternatives remain in old state for ordinary control-side
retirement. Active indexes continue to find only the requested active definition.

Reserve one processor destination for the selected old owner, but do not infer
that one old-owner destination means one known reference route. Prepare a
bounded union of possible definition positions and route-stage identities, as
specified in answer 2 below. Adoption selects the actual predecessor
generation/latency/history in already reserved stages, then computes
per-generation waits from the concrete rack.
It must neither substitute marker equality for ownership nor apply active
parameters to the outgoing definition. The plan's published reservation choice
is fixed-size metadata so a subsequent control snapshot can resolve the exact
departing definition. No additional factory constructor is involved in choosing
an alternative, and a fresh candidate is never a capture/edit owner merely
because it appears in a ledger or reservation.

This is a required narrow **plan/state/rack** window, not a claim that today's
`keep_leaving` already supplies this behavior. Unresolved pending choices which
would require a third alternative must give `UnresolvedProgress`/retry before
document mutation, not append a lineage. The representation must retain the
existing at-most-one-audible-outgoing/one-current-active owner contract and the
existing twice-`MAX_EFFECT_SLOTS` installed-definition regression. Storage for
alternatives is metadata inside a single reserved outgoing seat; it cannot be
used to weaken that regression. If this cannot be implemented with bounded
reserved state and exact stage histories, return the design obstruction to the
parent before source implementation rather than fall back to a native-only fix.

The acceptance tests must include the narrow final-check-to-callback race too;
a progress-version check alone only catches changes before that check and does
not close the remaining race. The existing `Processor::adopt` call order and
T1's transport/processor changes do not need alteration for this proposal:
reconciliation lives in the existing `PlanState::take_over` seam.

## Exact answers required before the source grant

These refine the initial reservation proposal. They are a concrete design to
review, not evidence that the current implementation already has these types,
bounds or guarantees. Public preparation retains the four operations above;
the additional machinery below is private to the engine/Session module.

### 1. Borrowed capture, error/unwind ownership, and actual reuse

`preparation_snapshot` takes the mutex only to clone controller/plan/ledger Arc
handles and copy numeric identities. It never calls `maintain`, consumes a queue,
replaces a field, hashes a binding, clones ledger maps, invokes a factory or owns
a uniquely held native object. Poison recovery uses the existing `into_inner`
route. There is no fallible owner-bearing temporary inside this capture scope.
Dropping the temporary mutex guard on error/unwind only unlocks the mutex.

`publication` borrows `&mut PreparedPublication`; its errors are the small
owner-free refusal enum. The lease owns a controller MutexGuard and a mutable
reference to the outer token. It owns no PlanState, native box, factory Arc or
retirement value. Its Drop only unlocks. The candidate's prepared-message and
retirement envelopes remain fields of the caller-owned token until `install`
moves them into their destination. Bookkeeping Arc references are moved to the
outer retirement envelope, never disposed of as consumed-snapshot locals.

The private Session finishing function owns the complete staged edit as a
parameter/local **before** entering recording/library/State/lease scopes. Locked
helpers accept mutable borrows of this outer staging object, not ownership of
`PreparedSampleEdit`/`PreparedPublication`. Thus a normal return, `?`, or panic
unwinds lease, State, library and recording guards before the staged parameter
and its candidate/retirement fields. No caller passes an owned candidate to a
guarded `commit(self, state)` helper. This is the ownership change required for
the ordinary Session routes, including M1's borrowed eligibility. Public Rust
cannot prove that arbitrary external code holds no unrelated mutex; do not
claim that capability. The production Session module controls the lexical
ownership and borrowed helper types, and checking destructors verify it.

The install body contains only prepared field moves, numeric/atomic updates and
reserved SPSC pushes; no generic callback, String formatting, allocation, FFI,
fallible command lowering or panic assertion runs there. A private reserved
producer lease checks all needed slots once while owning the only producer's
controller guard. The consumer can only increase those slots. Empty retirement
destination fields are checked before the lease is issued, so install never
overwrites a live carrier field. Displaced values go directly to the outer
carrier. This is the install panic-safety obligation, not permission to place
an `unwrap` around a fallible constructor after document mutation.

Known rack seats remain `None` in the candidate's PlanState. At takeover, their
actual concrete EffectUnit/InstrumentUnit moves out of the immediately preceding
processor state by the accepted kind/native identity/generation. It carries its
current curve/smoothing/table/asset references and owner-local heard state with
it. Compensation takes the actual old line's current ring, TapCrossfade weights,
wait/pending/readiness and valid history, into reserved capacity. Neither the
compile-time Plan nor the immutable ledger supplies these mutable values.
The ledger only certifies identity, rate, kind, prepared capacity and layouts.
If a future immutable asset/table identity requires a new prepared processor,
that identity belongs in this reuse key; do not treat kind equality as proof
that a changed prepared asset can be applied in the callback. Current Copy
parameter edits retain the existing adoption-before-dedup/control behavior.

### 2. Late-heard choice, destinations, route storage, and its bound

The worker reserves at most two source generations per logical outgoing seat:
the captured audible/departing generation and captured current active generation.
It never builds DSP/native processors for either source. Their processor owners
already exist in the old processor or ordered pending states. A genuinely fresh
requested active seat has exactly one new DSP/native construction; a retained
active seat has zero. Thus alternatives add **zero** constructors, zero native
boxes and zero duplicate DSP tables. They add definition/choice metadata and
reserved compensation storage only.

Choice is a single packed AtomicU8 per reservation: unset, provisional-none/
outgoing/active, or adopted-none/outgoing/active. At lease acceptance, a consistent
bounded read of source progress publishes a provisional choice with one atomic
store. This is advisory eligibility, never prepared native identity. At callback
takeover the audio owner reads the actual two concrete seats and their Splice/
heard state, chooses the one still audible under R4's serial-predecessor rule,
moves it to the reserved destination and publishes the adopted choice with one
atomic store. The callback does not wait for the lease's progress sample. It
continues the actual removal remainder; a fresh insertion waits that remainder
plus its own actual priming. Gone/unheard alternatives stay owned by the retired
old state. A lease choice is never taken as proof that the matching native box
was heard or even taken over.

Different alternatives can occupy different tracks/chain positions. The worker
therefore reserves the union of those definition positions, with an empty
conditional placeholder where an alternative is inactive. Same-position
alternatives share a placeholder. Only one position receives the concrete
outgoing owner. Active indexes are built against this fixed union, never
relinked/allocated on the callback. The existing installed-definition limit
includes every conditional placeholder: **at most 2 * MAX_EFFECT_SLOTS = 20
rows per track**, 2560 rows at MAX_MIXER_TRACKS=128. If the union exceeds that
limit, or an unresolved earlier reservation would require a third source
generation, preparation/lease refuses visibly before Document commit; it must
not hide the additional rows in an uncounted collection to pass the bound test.
Once accepted, hearing can change which of the two choices applies, but cannot
create a third source or another destination. There is no callback
`UnresolvedProgress` return, and no refusal after Document commit.

The initial suggestion of only taking the maximum of two local delay bounds
is insufficient for arbitrary changes of reference route. The concrete proposed
reservation is a **union stage bank per compensation destination**, plus fixed
capacity scratch for resolving the causal reference route. Set `D = T + E + I`
for direct track, edge and instrument compensation destinations in the frozen
candidate, and `K = U + I` for its unique effect-generation union (`U <= 20*T`)
and possible instrument prefix stages. Reserve one aggregate CompensationLine
and at most K stage lines per destination, plus K active-stage indexes and
DelayStage metadata. Callback resolution uses bounded scans/assignments in these
preallocated banks; only selected stages process. It selects the actual source
generation/maximum before copying matching old histories. Physical reserve
capacity may exceed a selected stage's `spec.maximum`; that does not change
`same_line`'s actual identity or use a mutable intention as an ownership key.

This reserves at most `D*(K+1)` routing CompensationLines, **not 2^U complete
graphs**. It prepares zero alternative EffectUnit/InstrumentUnit objects. Each
line bound is the maximum required by the finite candidate/held alternatives,
including the actual old ledger capacity when tap history is inherited. Pure
route resolution into fixed scratch is the additional narrow state/rack work;
the existing allocating `Layout::of` cannot be called on the callback unchanged.
Its native PDC values all come from already negotiated ready owners. The
one-second causal/scalar policy still applies to the selected real path; the
larger union reservation is storage, not a new audible delay or a phase exception.

Exact requested payload accounting for each line is
`size_of::<CompensationLine>() + L*size_of::<Frame>() + tap_capacity*size_of::<Tap>()`,
where `L = checked_next_power_of_two(maximum + 1)` and tap_capacity is the actual
reserved Vec capacity (at least L). Include all bank envelopes, index/DelayStage
scratch and reservation metadata using their actual capacities and `size_of`,
with checked sums/products. On the current x64 layout Frame=8 and Tap=16 bytes;
with tap_capacity=L the variable payload is 24*L bytes per line. For example,
a one-second 48k bound has L=65536 and 1,572,864 variable bytes per reserved line.
The conservative bound is the checked sum over at most D*(K+1) such lines, not
the misleading assertion that MAX_EFFECT_SLOTS alone fixes a byte count.
Instrument/channel count and sample rate are additional inputs; the model has
no fixed maximum channel count. Factory-internal/helper process memory is also
not measured by these Rust payload sizes.

Proposed private bound for **new alternative/route reservation payload** is
256 MiB per candidate. Exceeding it or any checked-size overflow produces
`MemoryLimit { required, limit }` before constructors/Document mutation whenever
the inputs already determine the count; otherwise it refuses before lease
after actual negotiated latency is known. Native helper allocation is not
falsely charged as known Rust memory. This numerical engineering bound needs
parent review; it is not a measured device deadline or overall process-memory
guarantee. Existing sampler banks retain their independent aggregate budget.
The union-bank design is deliberately conservative; its practical CPU/memory
cost and exact history behavior need the unchanged cancellation/bounds tests
before it can be an accepted implementation.

### 3. Stream generation, outer carrier capacity, and no allocating install

Prepare an outer retirement envelope on the worker, before final recording/
State acquisition. It contains fixed Option destinations for old Plan, Ledger,
Link, backlog container and provider/snapshot bookkeeping, plus a Vec reserved
for exactly GARBAGE_CAPACITY=4096 current-ring garbage entries. Its vector is
never grown under guards. `take_retired` pops at most the free carrier slots;
full capacity leaves the next value in the existing ring. The realtime worker
creates its reusable carrier outside all subject guards as part of its control
thread startup. The publication interface does not implicitly drain it.

A stream transition moves one old Link and one entire old VecDeque backlog
container into their fixed carrier destinations. It does not drain them to
another Vec while guarded. The old link itself retains its message ring (1024
entries) and garbage ring (4096 entries) until their endpoints can be dropped
off guards. Including a full current-ring drain, a transition carrier can retain
at most 8192 ring garbage entries, 1024 ring messages, its fixed handle fields,
and the **existing backlog's actual B entries**. B is not bounded by the current
legacy controller policy; moving its container whole neither allocates nor adds
an unbounded retirement queue. Do not report a finite global backlog-memory
bound without separately changing and visibly reporting that existing policy.
Project preparation adds no messages to that backlog: admission requires it to
be empty and reserves actual ring slots before any musical mutation.

For normal edits reserve SetPlan plus a possible pattern correction (at most
two messages). New/Open reserves SetPlan, Stop, SetTransport and Seek (at most
four messages), with the existing sequence/playhead semantics committed through
the lease. Reattachment prepares fresh rings and its bootstrap messages before
the guarded link swap. Empty carrier destinations and adequate slots are lease
preconditions. Insufficient message capacity refuses QueueFull; an occupied
transition carrier is drained/destroyed outside guards before taking another
lease. `install` moves its already constructed carrier out as the return value;
it allocates no Vec/Box/Arc and clears no owner-bearing collection.

Detached publication increments plan generation and retains the compiled plan/
assets with explicitly deferred rate-specific readiness. Suspend/detach each
increment stream generation before forgetting the link and move all old
ownership outward. An attach token names exact plan generation, old stream
generation and requested negotiated rate. Successful attach increments stream
generation and installs the actual prepared Ledger; failed/stale attach changes
no engine plan/link and retires its candidate processor off guards. A failed
CPAL attempt followed by detach/suspend/another attach invalidates earlier
tokens even at the same rate. Counter overflow refuses before mutation. Device
error/stream closure cleanup uses an outer carrier, never a refusal that drops
the old native-bearing link under the mutex.

### 4. Unavailable provider versus constructor error

Keep PluginFactory's existing public `Result<Box<dyn HostedEffect/Instrument>,
String>` constructors. Change only the private preparation result into an
owner-bearing enum with non-optional ready boxes:
`PreparedNative::Effect(Box<dyn HostedEffect>)`,
`::Instrument(Box<dyn HostedInstrument>)`, or
`::Unavailable { binding_identity, reason: MissingProvider }`.
`plugins::prepare` returns `Result<PreparedNativeInventory, NativePreparationError>`.
`None` factory creates Unavailable directly, preserves the opaque binding and
builds the existing dry-effect/silent-instrument placeholder with a visible
unavailable report. `Some(factory)` returning Err is NativePreparationError;
it is never converted to Unavailable or an Option::None "ready" processor.

Ready native ledger records retain the exact accepted binding/provider/revision
calculation and actual rate/latency. Unavailable metadata is separate from that
ready-owner map. Reuse of a dry/silent placeholder may use its explicit binding/
availability metadata but must not advertise a native owner or authorize state
capture. Freeze candidate active native-owner identity from successful inventory
records before plan departure matching; compilation intent is not a replacement
for those actual records. R6's active lookup excludes departing records and
continues to use the ready ledger. Changing None to Some, provider/revision or
binding invalidates eligibility and may require fresh worker preparation.

Opaque constructors can fail only after allocating partial resources; their
cleanup remains on that off-guard worker. The outer token retains any successful
earlier entries until all guards are absent. This keeps sampler failure,
unavailable provider, native startup failure and stale publication distinct
without adding a public factory-policy framework or parsing native error text.

## Retirement and publication capacity

Change `maintain` into bounded queue forwarding only. Controller queries,
transport methods, hardware-note paths and message sending must not run implicit
native garbage collection. In particular, `frame` and `transport` can be called
while a Session guard is held and therefore cannot drop returned `PlanState`.

Use the existing message/garbage rings. A project publication lease requires
enough actual message capacity for its prepared commit envelope and no preceding
backlog that would invalidate that reservation. Full capacity is a visible
publication refusal. It does not allocate/grow `VecDeque` under publication,
discard the candidate or mutate musical state. Keep existing non-project command
ordering and hardware bounded-refusal behavior; do not invent an unbounded
project preparation queue as the workaround.

`ProjectRetirement` has capacity for a bounded drain of the existing garbage
ring. An insufficient carrier stops before popping the next value; remaining
values stay in the existing ring and preserve its callback headroom/backpressure.
Detach/suspend/reattach move the old Link and backlog container whole into the
caller-owned carrier with `mem::replace`; no `clear()` of native-bearing messages
under locks. Initial attach queue allocations and `Processor::with_queues`
construction happen before the guarded link swap. Old plan and hosted Arc values
are also moved out, not implicitly dropped by assignment.

Existing callback `GARBAGE_HEADROOM`, ownership movement and `retire` behavior
are preserved. This track does not advertise new deadline/platform guarantees
from allocator tests. It must not add a new silent overflow/leak path or use the
existing emergency `mem::forget` behavior as its normal capacity policy.

## Direct, attach/reopen, and offline callers

- **Direct Controller callers/tests:** `set_project` may capture, prepare and
  publish synchronously on their off-guard caller; `set_prepared_project` may
  complete legacy asset-only preparation there. Both expose a refusal and keep
  the old plan/transport. Static `prepare_project`/`try_prepare_project` and
  `compile_prepared_project` retain their sampler-only compatibility contract
  rather than falsely claiming native readiness. Production Session uses only
  snapshot preparation and the lease; the old path is forbidden there.
- **Detached installation:** no stream has a known negotiated rate. Accept an
  explicitly detached plan/asset snapshot with no native constructors at commit.
  Native readiness is deferred to attachment, never silently represented as a
  fully prepared streaming state. A stream attaching during detached preparation
  changes stream generation and refuses the candidate before musical commit.
- **Attachment/reopen:** `attach(rate)` captures current plan/provider identity,
  builds fresh complete PlanState, ledger, queues and processor off-lock, then
  accepts only that same plan/stream/provider snapshot. `try_attach` is the
  fallible production device route; stale attachment retries on the device
  control thread with a visible finite retry exhaustion reason. Native failure
  returns the exact cause. Successful installation freezes the actual prepared
  R6 ledger. Failed CPAL attempts and abandoned processors retire off guards;
  suspension/reopen keeps existing transport resume semantics. `Processor::new`
  retains its empty-controller compatibility construction.
- **Offline:** `render` and stems `Pass::new` currently compile a plan, `set_plan`,
  then `attach`. The same off-lock attachment construction applies. N4 owns
  render/stems/pool/export and freezes the adapter to `try_attach` or an explicit
  checked independent-preparation route. Its `render_error` additions and
  cancellation/output-removal semantics survive unchanged. No P1 render edits
  without a separate parent-coordinated N4 window.

The actual Session caller inventory to convert is:

- `edit.rs`: dispatch, dispatch_plugin_update, capture_plugin_update, automate,
  history, publish_with_prepared and push_project. Native notification validity
  remains checked at final acceptance. Capture's original selected owner is
  captured off State; any subsequent musical command uses the checked stage.
- `mod.rs`: refresh_plugins and startup/provider installation metadata. Preserve
  M1's analysis_jobs registration/Service construction and all Cargo references.
  Retry preparation releases recording/State while blocked, and rechecks
  recording/runtime revision before publication.
- `sampler_processing.rs`: SampleEditTicket::prepare/PreparedSampleEdit::commit,
  finish_sample_edit and refresh_sampler_plan. Keep request cancellation, source
  identity including loading, generation/edit/replacement guards, candidate
  overlays/recovery and the bounded history/background retry policies.
- `clip_processing.rs`: checked clip command and background plan refresh. Keep
  clip->sampler cache-miss routing and source equality. Final recording-before-
  State exclusion is mandatory; the background clip route currently lacks it.
- `files.rs`: install shared by New/Open/FLP/archive and save relink publication.
  Replacement admission precedes stop, State swap and staged runtime revision
  installation. Preserve replacement/cancel/edit checks, saved path/history and
  archive extraction ownership. Save relinks need a checked candidate publication
  or a proved audio-identity-preserving metadata-only path; never hidden native
  construction through `publish`.
- `library.rs` and `midi.rs`: add_audio_clip_from_sample and reviewed MIDI import
  currently mutate the document before `publish`. Route them through the staged
  commit and preserve review/root/file/source guards. File imports/recording takes
  already using SampleEditTicket retain those same guarded overlays.
- `audio_editor.rs` and `slicer.rs`: replace asset-only plan completion with
  snapshot preparation and acquire the engine lease before dispatch, pool insert,
  persistent-file ownership transfer or review consumption. Preserve current
  exact source and edited/review checks.
- **M1 owner adapter, not a P1 source grant:** frozen `analysis_jobs.rs:936`
  currently does Plan/sampler-only preparation; its final `:995` publication
  reaches `edit.rs:244/253/292` and the native-construction controller path.
  M1 will use the ready token at 936, borrow engine eligibility before actual
  Document dispatch at 980, then install without fallback at 995. Its artifacts
  acknowledge only after Document commit plus guaranteed engine install;
  retirement happens after all guards. Preserve its full source/loading/loaded/
  failed/request/model/job guards and zero-allocation eligibility/acknowledgement.
  M1 adopts the landed interface itself; do not edit its frozen draft.
- `samples.rs`: decoded source attachment/reload must prepare a private overlay
  before changing the live pool, keep original loading/source/request policy,
  then lease-check before attachment. No generic sample synchronization may
  fetch a newer source on the candidate's behalf at final commit.
- `audio.rs`: post-configure refresh prepares outside State. Device rate/stream
  changes invalidate in-flight candidates. `realtime.rs`: explicit retirement
  maintenance runs outside State and other caller guards.

`flp.rs`, `archive.rs`, recording imports and browser import routes use shared
install/ticket seams; read their guards, but do not edit them merely because they
call those seams. A changed return carrier should be threaded through their
existing caller scopes only if necessary, with an exact extra grant.

## Minimum serialized file windows

1. **Engine window after T1 freeze:** new
   `crates/windfall-engine/src/project_preparation.rs` (or keep the private
   implementation in controller if that is smaller); `controller.rs` for snapshot,
   lease, identity, off-lock attach and legacy adapters; `state.rs` for Arc ledger,
   fallible whole-state build and reserved adoption; `plan.rs` only native identity
   and departure reservation/linking; `rack.rs` only concrete departure selection
   and reserved compensation history; `plugins.rs` only private prepare/error
   propagation and bounded metadata documentation, preserving N4's render_error;
   `message.rs` only ownership/capacity carrier if needed; `lib.rs` exports.
   Tests stay in controller/state and existing utility modules/registration.
   No project model, meter/transport/sequencer, processor, host ABI or IPC edits.
2. **Session window after M1 freeze:** new
   `apps/desktop/src-tauri/src/session/project_preparation.rs` for candidate/guard/
   lease/retirement orchestration; narrowly registered in `mod.rs` without moving
   M1's registration. Edit `edit.rs`, `sampler_processing.rs`, `files.rs`,
   `clip_processing.rs`, `library.rs`, `midi.rs`, `samples.rs`, `audio_editor.rs`,
   `slicer.rs`, `audio.rs`, and `realtime.rs` only at the inventoried preparation/
   publication routes. Register new preparation regressions in session tests.
   Existing sampler/browser/history and native owner guards survive composition.
3. **N4 coordination window:** parent/N4 owns final offline caller adapter and
   actual unavailable/fatal error policy. P1 does not edit facade/factory/manager/
   runtime/helper files or render/stems/pool/export. A barrier factory is a real
   engine constructor dependency, not proof from an old cached native fixture.
   M1 owns the analysis Apply adapter after this interface lands. P1 does not
   edit `analysis_jobs.rs` or its request/model/artifact workflow.

These are requested windows, not permissions already obtained. Each checkpoint
must be source-only and immutable; no parent imports, generated artifacts,
README/checkpoint/parity/release edits, reset/amend/push/PR. The parent composes
the frozen owners and runs the final authority check before declaring this
narrow finding solved.

## Required actual regressions and verification

The initial design has no new green evidence. Once granted, add a registered
barrier PluginFactory constructor whose return creates an actual owned facade
with negotiated latency and instrumented destruction. Avoid sleep-based ordering.

1. Stall construction while separate threads query Controller realtime/transport
   and Session document/recording. Those operations must complete before release
   of the constructor barrier. Exercise checked dispatch, import/replacement and
   refresh paths, not just a synthetic new preparation method.
2. During the stall, perform New/Open/edit/rate/stream/factory-revision changes.
   The pending result refuses before project/pool/history/review/transport/runtime
   commit. Assert byte/identity equality, no consumed review, no partial stop and
   a reason distinct from sampler/native failure. Start recording during work
   and assert recording-before-State refusal keeps the owned take intact.
3. Reuse the actual held owner with zero constructor/capture calls. A fresh
   candidate has no selected capture owner before actual adoption. Change the
   same factory Arc's revision while the native constructor is blocked; refuse
   the mismatched candidate and destroy it exactly once off all guards.
4. Put a try-lock/checking destructor on unused prepared facades, retired state,
   failed attach, replaced backlog/link and last provider handles. Demonstrate
   both controller and external Session guards are free at destruction/helper
   join. Realtime queries under State must not accidentally collect.
5. Advance real irregular/single-frame audio through outgoing completion or
   first audible insertion while preparation is stalled and after final
   acceptance before callback adoption. Assert exact old remainder, no restarted
   fade, serial outgoing/fresh ordering, one audible outgoing/current active,
   preserved active indexes and negotiated PDC cancellation. Do not replace this
   with a cached removal-count assertion.
6. Keep R1-R6 utility assertions, especially detached precompile, late revision,
   failed attach retry/suspend reopen, 40 speculative/superseded preparations,
   the MAX_EFFECT_SLOTS/1000-round plan bound, `1e-6` cancellation and existing
   `0.04` continuity bounds. Instrument every relevant adoption/reset/retirement
   callback for zero alloc/realloc/free. No weakening, ignore or fabricated
   fixture proof.
7. Preserve sampler/history/browser request/loading/source budget/cancellation
   regressions and native capture/adoption-before-dedup regressions. Integrate
   N4's actual bridge startup/latency/retirement tests after its frozen checkpoint.
   Fill publication/retirement capacity and counter-exhaustion seams and assert
   visible refusal/backpressure rather than silent destruction or accumulation.

Native verification uses Git Bash `source scripts/msvc-env.sh`, one Cargo job,
and the unique task target `target/p1-native`; generated TS exports stay task-local.
Run focused new engine/controller/Session regressions and affected existing
utility/sampler/native-ownership groups on the actually composed source, then
the parent's required strict checks. Do not run builds or full suites merely
to validate this proposal. Allocator/source tests establish their specific
contracts, not listening quality, a hardware deadline or other-platform proof.

Additional N4 owner provenance supplied after the initial proposal: thirteen
individual graph cases are reported green, including CLAP/VST3 latency 561
(synthetic source 12 + native 37 + bridge 512), an impulse at 1024 appearing at
1585, zero callback allocations at partitions 1/7/64/480/512, and offline audio
after stripping latency beginning at 1024 with bit-equal chunk partitions.
Those are N4's results awaiting its full checks/source pin. They are not P1 test
runs or parent-composed acceptance of this still-unimplemented locking change.
