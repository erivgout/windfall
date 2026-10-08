# Project and native preparation: P1 interface proposal

Status: **proposal only; production lock ownership remains open**. This file is
the first isolated deliverable, not an implementation or a new acceptance claim.
No existing source, generated bindings, artifacts, parity files or release
records were changed. No Cargo/build/test command was run for this design stage.

Design revision R2 responds to the independent review of immutable
`07cdb24b5628ae79ae56c7933df26a050abfe1ce`; that checkpoint remains unchanged.
The parent granted only this incremental document response and temporary
diagnostics. The four findings are answered in sections 2a/2b (row mapping and
pending publications), 2c/2d (selected history and inherited capacity), 3a
(owned publication intent), and the attachment caller/file window below.
Existing serial callback adoption is the policy; no processor transaction
or same-callback commit-group guarantee is proposed.
R2 additionally read root `a0ec6372200251c3170178cb29737a229dca5ba4`, the N4
runtime writer/adaptation draft, and installed rtrb 0.4.0 READ ONLY. None was
imported. All implementation/test obligations below remain unimplemented.

The isolated branch is `gpt/t3-project-preparation-p1`, with fixed base
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
        intent: ProjectPublicationIntent,
    ) -> Result<PreparedPublication, ProjectPreparationError>;
    // private counterpart for already prepared sampler/clip pools
}

impl PreparedPublication {
    pub fn sampler_pool(&self) -> &SamplePool;
}

impl ProjectPublicationLease<'_> {
    pub fn install(self) -> ProjectRetirement;
}

pub enum ProjectPublicationIntent {
    Edit,
    Replace { transport: TransportPatch }, // New/Open/FLP/archive
}

pub enum ProjectPreparationError {
    Sampler(SamplerPreparationError),
    Native { target: PluginTarget, reason: String },
    Unsupported(&'static str),
    IdentityChanged,
    StreamTransitioning,
    UnresolvedProgress,
    SizeOverflow,
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
and block capacity, plus Detached/Starting/Running/Closing phase. Starting and
Closing snapshots refuse StreamTransitioning off guards; their publication
counterparts refuse StaleStream. Transport sequence, musical
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
stamp, owned publication intent, candidate first-pattern metadata, `Arc<Plan>`,
the prepared sampler pool, and either an attached payload
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
   Stage the candidate Session pool from the ready sampler pool here too, before
   taking a lease: the lease's mutable borrow of Ready must not force a later
   sampler-pool accessor or cache completion under State.
3. **Final acceptance.** Reacquire recording, caller-specific library guard,
   State, then the controller publication lease. Recheck all original Session
   guards and engine/factory identity, plus reserved storage/capacity. Only after
   both authorities accept may the caller swap its validated candidate document,
   history and pool handles or consume its review. A refused candidate remains
   in an outer ownership scope. No stop/seek, progress cancellation or runtime
   revision installation is performed on the refusal path.
   Bind the owned intent to late transport as specified in 3a, then obtain the
   exact borrowed lease before any successful musical Document dispatch.
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

The returned retirement carrier immediately moves into an outer
`Option<ProjectRetirement>` checked as None before admission. Never declare
`let retired = lease.install()` inside the State scope: later event-reporting
unwind would drop it before the State guard. Never overwrite even an empty
prior carrier under guards; its reserved Vec owns heap storage. The private
finishing seam owns this outer destination and the staged edit, so ordinary
Session callers cannot transfer candidate ownership into a locked helper.

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
it. Its prepared kind/native identity/rate are bounded metadata fields carried
with the unit; callback comparison performs no binding hash, factory call or
revision read. Compensation takes the actual old line's current ring, TapCrossfade weights,
wait/pending/readiness and valid history, into reserved capacity. Neither the
compile-time Plan nor the immutable ledger supplies these mutable values.
The ledger only certifies identity, rate, kind, prepared capacity and layouts.
Generation retention and the builder's `known` decision use the same actual
Ledger reuse predicate. Equal kind plus exact prepared binding/provider/revision
and rate preserves the predecessor generation and moves its unit; an ineligible
unit gets a fresh generation **and** a fresh off-guard construction. Do not leave
a fresh-generation seat None merely because a target-only native record matched.
N4's staged Runtime wrapper and installed Runtime can have different factory
Arc pointers but the same provider identity and accepted revision. Raw factory
Arc inequality cannot break generation retention while the builder reuses that
same native owner. Preserve the existing XOR identity calculation; strengthen
the common predicate with the captured provider/revision/rate metadata, rather
than asking the old mutable factory for its past revision.
If a future immutable asset/table identity requires a new prepared processor,
that identity belongs in this reuse key; do not treat kind equality as proof
that a changed prepared asset can be applied in the callback. Current Copy
parameter edits retain the existing adoption-before-dedup/control behavior.

### 2. Late-heard choice, destinations, route storage, and its bound

#### 2a. Physical row to source-definition mapping

Use private `DepartureReservation` metadata with two flat `SourceDefinition`
entries at most. Each entry contains an ordinary outgoing definition (no nested
reservation), its EffectLife/generation, source kind, and the actual prepared
native record from the captured Ledger where applicable. No entry contains a
native/DSP owner. Each also names its candidate destination `(track, row)`.
Same-position alternatives share a physical row; different positions reserve
two rows. A row records its reservation index and the alternatives allowed
there. All such rows are always leaving for active indexing. Requested active
definitions have separate ordinary rows and retain their existing active-only
indexes. A fresh active's predecessor is the reservation index, so takeover can
find the selected outgoing unit's actual remainder, whichever row it occupies.

The worker computes positions using the existing keep_leaving serial ordering
with possible source definitions rather than a worker-time heard filter. It
keeps the existing removed-track policy (no destination means immediate
departure), then merges identical positions. There is no callback insertion,
relink or removal of vector rows. Only the chosen position receives the concrete
outgoing unit; unchosen positions stay empty. A source retained as the exact
requested active owner is excluded from the outgoing alternatives.

Let `P` be physical rows, `R` logical reservations, and `G` distinct possible
generation identities. Every physical placeholder counts in `P`:
`P_track <= 2*MAX_EFFECT_SLOTS = 20`, `P <= 20*T <= 2560`.
Each reservation has at most two alternatives, `R <= P`, and there are at most
`2*R` alternative entries plus the ordinary active definitions in the metadata
count. Count actual `G` separately; it is not bounded by `P` alone. Ten shared
outgoing rows choosing g0/g1 plus ten fresh g2 active rows have `P=20`, `G=30`.
There are ten live outgoing destinations and ten fresh active constructors,
with **zero** alternative constructors, native boxes or DSP tables.

The ready Ledger has active records separate from departure-generation records.
Departure metadata can name both prepared alternatives even when one physical
row is shared. Every native record came from a successfully constructed or
reused prior unit at the accepted rate; provisional progress is never evidence
that an owner was prepared. R6 native_identity still queries active records only.

#### 2b. Successive pending publications and final choice

For each predecessor reservation, the worker performs one Acquire read of its
packed choice. `AdoptedNone` contributes no departure; `AdoptedA/B` contributes
that one ordinary source definition. An unset/provisional choice contributes
both flat source definitions, regardless of the provisional value. Add the
predecessor's ordinary active definition if this edit makes it depart; exclude
an exact retained active owner. Deduplicate by EffectLife Arc ownership plus
generation/kind/prepared native identity, not by project id or a numeric marker
alone. More than two possible source definitions
for a logical outgoing seat gives `UnresolvedProgress` before constructors or
Document commit. The caller retries from a fresh snapshot after adoption.
No child stores a parent reservation or follows a recursive choice lineage.

Thus a pending g0/g1 reservation plus active g2 permits a queued parameter edit
retaining g2: its flat outgoing universe stays `{g0,g1}`. Removing/replacing g2
requires `{g0,g1,g2}` and refuses until the predecessor is actually adopted.
After adopted g0 or g1 is observed, `{selected,g2}` fits; adopted-none leaves
only g2. Metadata therefore never accumulates older unresolved generations.
Reading an adopted choice while audio advances is safe: that final identity
never changes again; its unit may finish, which final takeover handles as None.

Lease checks cover the exact predecessor plan/stream/actual Ledger and all
reserved positions/capacities. An advisory progress sample may change while the
worker or lease runs; it cannot prune the flat universe. In particular, two
separate EffectLife loads can observe g0 before it finishes and g1 afterwards;
the proposal does not require those loads to be a coherent owner snapshot.
The sole callback writer chooses from the actual immediately preceding state:

- Continue an audible outgoing unit with its current Out remainder, excluding
  Gone even if its owner-local heard boolean is still true.
- Otherwise choose the departing ordinary active unit only if it has actually
  been heard; start its departure using its actual current splice scale.
- Otherwise choose None. Exact retained active owners stay active.

R4's serial waits exclude two simultaneously audible predecessors. Takeover
looks in the concrete old chains using the unit's life/generation and prepared
kind/native metadata. A shared physical row's nominal g0 definition is not the
lookup key when it actually contains selected g1. The old plan's adopted view
supplies definition/placement, while the unit certifies ownership and progress.
Takeover neither uses the provisional choice nor instantiates a missing source. It moves the
chosen unit intact and publishes `AdoptedNone/A/B` with one Release store.
Unselected sources remain in the old state for control-side retirement; an
empty conditional row does not finish an unselected source's progress marker.
The fresh insertion waits the selected unit's **actual** removal remainder plus
its own negotiated priming. No selection error, third-source refusal, allocation
or wait is possible after lease issuance or Document commit. Ordered queued
SetPlans ensure the predecessor state exists at this takeover, even if neither
queued plan has processed sound yet.

#### 2c. Ordered selected views and history transfer

For compensation, use a bank keyed by physical row, not by every alternative
generation: `K = P + I` possible simultaneously selected stage positions, where
`I` counts actual instrument prefix positions. Each destination has one aggregate
line, at most K stage lines, and preallocated selected-index/spec buffers. Resolve
the chosen rack definitions and causal route into those buffers using bounded
numeric scans in routing order. Preserve Layout::of's current tie breaking,
prefix factoring and one-second causal/scalar policy. The allocating Layout::of
cannot run on the callback. Native latency is already negotiated.

Both old and new Compensation expose an ordered selected view. Scalar mode
means **selected length zero**, even if the physical bank contains inactive
lines. Processing, retarget, snap, tail/readiness accounting and every history
decision use that view. Physical-bank length/order is never a route decision.
At adoption, resolve all selected specs first; then copy all histories; only
then retarget all selected lines. Retargeting a preceding stage before copying
its successor's input history would lose the actual prior transfer.

History rules reproduce the current rack algorithm on selected views:

1. Copy the old aggregate raw ring and current tap/wait/pending/readiness/valid
   state into the reserved new aggregate line.
2. Compute the fixed-prefix sum from new selected non-matrix stages only.
   Scalar promotion requires old selected length zero, settled old aggregate
   taps, and that sum equal to old aggregate delay.
3. For each new selected stage in causal order, first match an old selected
   stage by exact `key/generation/spec.maximum/matrix` and copy its actual
   history. Inactive lines are not candidates for matching.
4. With no match, scalar promotion reconstructs input from raw history at the
   selected fixed prefix. Otherwise reconstruct from the preceding **selected**
   new stage's copied output history if there is one.
5. For a first insertion before a retained suffix, compare the ordered selected
   following stages against old selected retained stages, filtering exactly
   the existing completed-departure condition (leaving with longest tap zero).
   Equal lengths and same_line matches mean raw identity input for the new
   first stage; applying the aggregate tap here would count the suffix twice.
6. Other first-stage reference changes copy old aggregate history. Preserve
   R3's short-history fallback: if its audible tap exceeds valid history, use
   the sum of old selected audible taps, bounded by actual valid history.
   This preserves the existing causal fallback, not an inverse phase promise.
7. Align waits by exact selected generation against the newly adopted concrete
   rack, then retarget aggregate and selected stages with the existing fade,
   pending/readiness and joining policies. Inactive stages contribute nothing.

The lease's choice may have been g0 while takeover chooses g1 during an unfinished
tap transition. Every step above uses the actual old state's selected view and
current lines at takeover, not the lease's route or compile-time tap mixture.

#### 2d. Inherited storage and exact payload admission

Ledger delay metadata records aggregate ring capacity and tap-vector capacity,
plus each physical bank row's prepared capacities and flat alternative specs.
It retains the same final-choice metadata needed to resolve old selected views;
it does not claim provisional specs are the actual heard route. These immutable
records certify storage already prepared; mutable history still comes only
from PlanState. Snapshot capture clones the Ledger Arc, not these maps/vectors.

For an unresolved destination, first implementation always prepares a new bank
off guards and leaves the old bank to retire with its state. It does not leave
an unresolved `line=None` and hope an incompatible old line can be moved later.
An unchanged fully resolved layout may retain the existing exact-layout move
fast path, but only with identical bank mapping and certified adequate capacities.

For each destination let H be the maximum of its held aggregate capacity and
all held potentially selected stage capacities. Reserve the new aggregate and
each possible new stage for at least H and its own largest possible requirement.
Reserve tap storage for at least the maximum inherited tap capacity and the new
ring length. This conservative rule also covers first-stage aggregate fallback;
checking only a same_line match would miss that transfer. Every transferred
old tap delay fits the new prepared range, so TapCrossfade::take_history does
not clamp/merge distinct inherited taps. Stage `spec.maximum` and same_line
identity remain the selected real path values, independent of storage capacity.
Use maxima of finite prepared capacities, not an additive lineage of prior
reservations; repeated handoffs do not grow capacity solely through inheritance.

There are at most `D*(K+1)` lines, `D=T+E+I`, `K=P+I`, not exponentially many
complete graphs. Count alternative-definition storage separately from this
physical line count. Exact line payload is
`size_of::<CompensationLine>() + L*size_of::<Frame>() + tap_capacity*size_of::<Tap>()`,
where `L=checked_next_power_of_two(checked_add(required_capacity,1))` covers both
current requirements and H. Count all bank envelopes, chosen index/spec arrays,
route scratch, both alternative definitions and retirement/message envelopes
using actual capacities. Every sum/product is checked; overflow is `SizeOverflow`,
not a fabricated numeric required-byte value. Current x64 Frame=8, Tap=16; with
tap capacity L the variable payload is 24*L bytes per line.

The proposed private new route/alternative payload limit remains 256 MiB per
candidate, subject to parent review. A full bank for T=8,E=7,I=0,P=160 with every
line reserved to L=65536 requests 2,415 lines and 3,798,466,560 variable bytes
before envelopes: it **refuses**, even for a legal dense project. This shows the
conservative design's cost, not practical feasibility or measured memory. At
L=1024 the same variable payload is 59,351,040 bytes. Native/helper process
memory is not inferred from these Rust sizes. Exceeding a determinable budget
refuses before constructors; negotiated latency may cause a later MemoryLimit,
still before lease, with successful candidates retired off guards. Never clamp
latency or drop history to claim Ready. Existing sampler budgets remain separate.
The unchanged R3/R4/R5 signal and allocation regressions are required production
proof of this selected-bank implementation, not already green evidence.

### 3. Stream generation, outer carrier capacity, and no allocating install

#### 3a. One owned intent and late-bound commit envelope

`PreparationSnapshot::prepare(project,pool,intent)` takes the owned
`ProjectPublicationIntent` shown in the public interface. Edit is used for
ordinary checked commands, history, imports, sample/clip work and plugin refresh;
Replace covers New/Open/FLP/archive with its saved `TransportPatch`. No separate
public methods or generic under-lock callback are needed. The prepared token
owns the intent, candidate first-pattern/membership metadata, and a fixed
four-slot message envelope. The worker builds its SetPlan payload and all
owner-bearing fields; it reserves optional owner-free numeric message slots.

Replacement preparation normalizes a supplied saved pattern against candidate
patterns, falling back to the candidate's first pattern. New supplies Pattern
mode and no saved pattern/loop override, matching the current files.rs rule.
Mode/pattern/loop fields omitted from the patch inherit the **late current**
controller state, not worker-time transport. First-pattern and validity data
are candidate-owned and prepared before guards.

Before issuing the lease, bind the fixed slots and final small controller/shared
updates using the state protected by that lease:

| Intent  | Reserved slots | Bound messages and effects                                                                                                                                                         |
| ------- | -------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Edit    | 2              | SetPlan, then SetTransport only if the late requested pattern is absent and a candidate first pattern exists; preserve late play/stop/seek, mode and loop                          |
| Replace | 4              | Stop with current sequence.wrapping_add(1), SetPlan, SetTransport with normalized saved patch over late state, then Seek(0); final requested playing=false and shared tick/start=0 |

This preserves the existing transport sequence arithmetic; new plan/stream
identity counters still use checked_add. Stop retains the existing hardware
panic and detached/resume behavior. A detached token installs its plan and
those same control values without claiming rate-specific native readiness or
creating/dropping a fake SetPlan state. Attachment/resume separately binds its
bootstrap messages to the late control state.

Bind/check only numeric fields and indexed candidate metadata. Generation,
stream/factory/predecessor identity, empty outer retirement destinations,
empty backlog and enough **actual** producer slots are all checked before the
lease is returned. A failed check leaves the intent/payload in the outer token
and changes no live sequence, transport, shared position, document, pool or
history. A prior play/stop/seek request does not itself stale the token; it is
preserved by Edit and intentionally superseded by accepted Replace.

Install first moves displaced ownership to the outer carrier and applies the
already bound control values, then publishes the prepared message slots in the
table's order through the sole producer. It calls none of Controller::stop,
set_transport, seek, send or maintain recursively. Owning the controller guard
prevents another producer from consuming reserved capacity; the audio consumer
can only add space. No QueueFull branch or native/fallible preparation remains
after Document commit. The source implementation must encapsulate that sole-
producer proof; it cannot use a legacy allocating backlog fallback.

Use existing per-message pushes for this policy. Consumer interleaving between
pushes and stopping at GARBAGE_HEADROOM remain permitted existing serial
semantics. There is no claim that all four messages are processed in one
callback. A batch tail adds no required guarantee here and is not selected for
this implementation, avoiding new initialized-prefix unwind ownership. There
are no processor transaction edits in this window.

N4 factory revision reads remain bounded metadata, but a controller mutex does
not freeze arbitrary trait implementations. Actual Session retry/replacement
revision writers serialize under the same recording-before-State admission.
Plugin refresh stages a preparation revision using existing prepare_document
rather than invalidating installed revision with retry before preparation can
fail. Existing install_document publishes it only in the accepted commit
scope; direct callers' independent later revision changes are separate new
transactions after the lease's acceptance point. Preserve actual ready Ledger
stamps rather than refreshing them from a live revision during install.

#### 3b. Carrier and stream transitions

Prepare an outer retirement envelope on the worker, before final recording/
State acquisition. It contains fixed Option destinations for old Plan, Ledger,
Link, backlog container and provider/snapshot bookkeeping, plus a Vec reserved
for exactly GARBAGE_CAPACITY=4096 current-ring garbage entries. Its vector is
never grown under guards. `take_retired` pops at most its initial free carrier
slots and GARBAGE_CAPACITY, even if audio concurrently refills the ring;
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

Normal edits and New/Open use 3a's exact reserved intent/order. Reattachment
prepares fresh rings and its bootstrap message envelopes before
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

rtrb's final endpoint owner drops queued values. A whole-Link move alone does
not prove control-side final destruction if an old Processor can outlive it.
The device adapter marks Closing and advances the checked stream identity in
a short guard scope **before** dropping the stream; it neither destroys old
ownership nor finalizes transport there. It preserves Supervisor's shutdown
order for resources and resume calculation: close/drop the old stream outside
Session/controller guards, then finalize suspend/detach from the late control
state and retire the old endpoint. The outer transition carrier retains that
control endpoint through Processor teardown. All musical publication is refused
while Closing, including the interval where unit fields have been destroyed but
the rtrb Consumer field still exists. producer.is_abandoned() is a further bounded
metadata check, not the sole ownership/liveness fence. A still-attached direct
replacement
without shutdown ordering refuses StaleStream before mutation. A failed
prepared attach owns both endpoints and its Processor in the outer staging
scope until accepted; abandoned candidates retire off guards. N4's process/
endpoint lifetime through confirmed helper exit remains its responsibility.

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
- **Attachment/reopen:** crate-private `try_attach(rate,mode)` returns
  `Result<(Processor,AttachmentAdmission,ProjectRetirement),AttachmentError>`;
  mode is Device or Independent, and AttachmentAdmission is a private numeric
  identity token for finishing device startup (already final for Independent).
  AttachmentError wraps preparation versus publication refusal distinctly. It captures the exact
  current plan/provider and detached stream identity, builds fresh complete
  PlanState/Ledger/queues/Processor off guards, then accepts only that source
  identity. A live attached link is not silently replaced. For reopen, its
  worker clone of Plan.current contains only the requested active graph: strip
  outgoing/conditional effect and departing instrument rows, give fresh
  concrete active units fresh EffectLife markers, and relink indexes off-lock.
  The disposed old stream supplies no mutable history or native reuse evidence.
  Freeze the successful units' actual rate/revision/latency in the R6 ledger;
  never treat stale compile-time life.heard as evidence of a fresh unit hearing.
  Successful attach changes stream identity and the installed prepared plan
  identity together; both counter advances are prechecked. `Processor::new`
  retains its empty-controller compatibility route. Existing crate-private
  infallible attach may remain only for the empty constructor and successful
  test fixtures; on error it explicitly reports/panics off guards, never returns
  a fake healthy processor. Production device/render adapters use try_attach.
  The device mode installs the prepared link as Starting; project admission
  remains closed until CPAL build/play has succeeded and the same numeric
  admission token marks it Running under a short guard. Independent offline
  attachment is immediately Running and uses its separate factory. No callback
  transaction or wait is added. An actual starting Processor still belongs to
  the unchanged installed document; candidates for a different document cannot
  gain capture ownership during this interval.
- **Device adapter, proposed P1 serialized window:** device.rs::build obtains
  `(processor,admission,retirement)` through try_attach before creating the
  Feeder/CPAL callback. Its outer retirement destination exists before any controller
  guard. Native preparation/refusal is converted to a preparation-specific
  backend failure before calling CPAL, not success or an unavailable unit.
  A private build-error distinction prevents fatal native preparation from
  being silently retried as an unsupported driver format. Stale preparation
  gets at most **three fresh snapshot attempts** on the existing device control
  thread; exhaustion reports the last refusal and attempt count. No latency
  deadline or detached startup thread is invented. Constructor failure stops
  that attempt. Starting fences musical admission even if CPAL internally
  destroys the Feeder before returning a build error. CPAL build/play failure
  disposes its candidate Feeder/stream outside all subject guards, then moves
  the Starting link into retirement and marks Detached. On success the exact
  admission identity changes Starting to Running before open reports success.
  Supervisor configure/poll/failed-open and thread-exit cleanup keep a control
  endpoint until their Processor has stopped. Preflight stream-generation
  headroom before closing: the device thread is the sole production writer
  for that controller, and an attach attempt reserves capacity for successful
  attach plus possible failed-CPAL detach. Closing/Starting tickets own only
  numeric identity and outer carrier borrows, never a mutex guard across CPAL
  work. A token from before Closing/Starting cannot publish while native units
  are disappearing, even at the same sample rate. Overflow is a visible refusal
  before transition mutation. Scripted backend fixtures use the same fallible adapter.
  These are requested device/controller windows, not source grants already given.
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
   `message.rs` only ownership/capacity carrier if needed; `lib.rs` exports;
   **`device.rs`** only the build/Feeder try_attach adapter, private preparation
   versus driver error propagation, finite stale retries, and Supervisor
   configure/poll/failed-open/thread-exit retirement scopes and their existing
   scripted fixtures. P1 requests this adapter window explicitly; N4 retains
   offline render/stems adapters. Tests stay in controller/state/device and
   existing utility modules/registration.
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

R2's four review conditions require these additional explicit schedules:

1. Queue a shared g0/g1 reservation with fresh g2; queue an edit retaining g2
   without extra construction; refuse a removal requiring three unresolved
   sources before Document dispatch. Adopt the predecessor, retry with its
   actual selected identity, then advance outgoing completion and g1/g2 first
   hearing after lease acceptance. Count physical rows and alternative entries
   independently; preserve active indexes and source stamps throughout.
2. Give physical banks inactive lines around a selected suffix and an empty
   selected scalar view. Delay takeover until g0/g1 choice and tap mixture
   change; check promotion, inserted-first suffix, reference switch and current
   readiness against ordered selected views. Inherit a ring/tap capacity larger
   than new spec.maximum, retaining distinct old taps without clamping. Preserve
   unchanged R3 short-history and R4/R5 signal assertions, zero callback alloc/free,
   and bounded capacities across repeated handoffs.
3. While worker preparation is blocked, play, stop and seek and change the
   requested pattern. Edit uses late control values and repairs a removed
   pattern; Replace uses saved patch plus late omitted fields and the exact
   Stop/SetPlan/SetTransport/Seek order. Exercise full/one-short/exact-capacity
   queues and consumer progress between each push, including headroom stopping
   partway through the serial messages. Refusal changes none of the musical
   state; success needs no same-callback transaction assertion.
4. Use the real fallible device adapter and its scripted backend for constructor
   Err, failed CPAL attempt, same-rate stale retry, exhaustion, suspend/reopen and
   full garbage/message/backlog retirement. Last-reference destructors record
   that relevant caller guards are free and the old Processor has stopped before
   final control-endpoint release. Keep native error, stale admission, sampler
   failure and explicit unavailable provider distinct. N4 owns helper/bridge
   shutdown and offline integration evidence on its final source pin.

Additional N4 owner provenance supplied after the initial proposal: thirteen
individual graph cases are reported green, including CLAP/VST3 latency 561
(synthetic source 12 + native 37 + bridge 512), an impulse at 1024 appearing at
1585, zero callback allocations at partitions 1/7/64/480/512, and offline audio
after stripping latency beginning at 1024 with bit-equal chunk partitions.
Those are N4's results awaiting its full checks/source pin. They are not P1 test
runs or parent-composed acceptance of this still-unimplemented locking change.

## Stage1 engine implementation provenance

This section records the granted engine implementation, not production P1
acceptance. The original proposal pins `07cdb24b5628ae79ae56c7933df26a050abfe1ce`
and `7519e5eddfc33cebdc498cbad263df1aee218b3a` remain immutable in history. The
original base is `6d0773804199faba65f866feda21b8c4ba9a6b9a`. The sole authorized
prerequisite import was `abd044dd0f332a07cd996315d640ddf0ac417479`, merged locally
as `937891439e4c9f268ddfa5cd91c1b7c6a9d4e8ff`. No other owner branch was imported.

The engine API is now `Controller::preparation_snapshot`, owned
`PreparationSnapshot::prepare(project,pool,intent)`, borrowed
`Controller::publication(&mut ready)`, and `lease.install() -> ProjectRetirement`.
`take_retired(&mut outer)` is explicit. The new implementation is a child module
of Controller, so its lease can guard existing private controller state without
exposing that state or introducing another mutex. The sampler pool accessor is
borrowed; callers must keep the token and returned retirement outside their
document/recording guard scopes. Refusal does not consume or destroy the token.

Snapshots retain the current Plan and actual Ledger Arcs, plan/stream counters,
stream phase and old installed factory stamp. Work builds the entire PlanState,
DSP/native units, dry histories, route banks, processor storage and message/carrier
envelopes off controller guards. No native constructor is moved into an async
placeholder. Provider identity/revision are documented bounded metadata reads.
Factory `None` retains unavailable bindings without a native-ready Ledger record;
constructor `Err` returns Native. A different factory Arc with the same provider,
revision and rate retains the actual generation and owner. A staged revision is
compared separately from the old installed factory and actual held Ledger.

Admission validates exact controller/Plan identity, plan and stream generations,
phase/endpoints and both factory stamps. It binds late transport fields and
reserves actual producer slots before mutation; a nonempty existing backlog also
refuses. The sole producer remains mutex-guarded and the consumer can only add
space. Edit emits SetPlan and, when necessary, the repaired pattern transport.
Replace emits Stop -> SetPlan -> SetTransport -> Seek(0) in that exact order.
The initialized messages are pushed serially. They can be adopted in different
callbacks under existing garbage headroom. No callback transaction was added.

Installation moves displaced Plan/Ledger/snapshot ownership into the carrier
that remains in the caller's borrowed token until the controller guard is
released. A cancelled or unwinding lease restores its initialized envelope and
leaves all native ownership with the outer token. Queries and lock acquisition
never drain garbage. `take_retired` captures at most the remaining preallocated
4096 entries and leaves additional owners in the existing ring. Attach/close move
one whole link and the actual existing backlog container outward. This does not
bound the legacy backlog globally or add a separate retirement queue.

Departures are physical rows with at most two flat source alternatives. A
restored active row is separate: P20 can represent G30 across ten slots. Repeated
pending publications flatten alternatives and reuse actual Ledger generations.
A third unresolved removal source refuses before constructors and admission.
Same-ID active revision/kind replacement follows existing `Plan::keep_leaving`:
it supersedes the active owner directly and retains existing outgoing definitions;
it does not turn every changed active generation into another departure. Actual
callback units choose the source at takeover, preferring an audible outgoing
owner and preserving its remainder, then a heard active owner, otherwise None.
No native constructor or capture selection occurs in this choice. A subsequent
edit observes AdoptedNone rather than promoting provisional source metadata.

Compensation banks use an ordered selected prefix for process, empty/scalar
promotion, prefix/suffix scans and history transfer. Inactive physical rows never
participate. A global held maximum capacity conservatively certifies every
destination/stage's inherited ring and tap universe. New conditional banks use
H=max(held capacity,sample rate), K=P+I stages and D=T+E+I destinations. The
checked 256 MiB reservation calculation includes line/sample/tap payload,
physical stage envelopes, `(2T+D)*K` route specs, flat reservation envelopes/Arc
counters, layout numeric and Vec storage, destination/carrier envelopes, 4096
garbage entries and four initialized messages. Path reservation uses reserve_exact.
It is a conservative route/alternative reservation bound, not total Rust RSS,
ordinary project/processor metadata, other DSP workspaces or helper-process
memory. Those existing preparations still run off guards. Overflow is SizeOverflow;
an excessive determinable reservation is MemoryLimit before constructors.
Repeated handoffs use the held maximum rather than summing prior reservations.

Native dry history uses negotiated latency. A native route exceeding the existing
one-second cross-track compensation bound returns explicit Unsupported before
publication; it does not silently clamp that latency or change the old plan,
clock or owners. Native dry-history size overflow/limit is a distinct native
preparation failure. Neither rule invents a helper latency or hardware deadline.

The device adapter prepares off guards and admits Starting, then completes the
exact stream identity only after driver build/play succeeds. Closing fences
publication before dropping the old stream, and endpoint/backlog retirement
follows processor shutdown. Identity drift retries at most three fresh snapshots;
native startup failure is fatal and is not a format fallback. Scripted backend
tests now use this fallible attachment route. Existing offline infallible adapters
remain a separate N4 composition obligation; they panic explicitly on preparation
failure rather than installing a missing unit as ready.

### Initial regressions and authorized test migrations

The initial reservation policy wrongly added changed same-ID active generations
to departures. It failed the unchanged R3 prepared-native-revision process count
(expected 30, observed 65) and R5 superseded-owner process count (expected 0,
observed 1). The Plan-window correction described above fixes both. They were
rerun individually with exact test names and both passed, then all 42 utility
cases passed with original owner/process/drop/signal/latency/allocation assertions.
No tolerance, timing, skip or ignore was added.

An early provider-stamp read also failed T1's unchanged invalid-meter metadata
count assertion. Convenience preparation now validates meters before capturing
provider metadata; compile starts with no factory stamp and fills it only for a
valid map. T1's gate/helper, typed meter Result and Unsupported message remain.
The invalid-map refusal/old-plan/transport/native-owner/healthy-retry test passes.

Utility test-only migration is limited to the granted
`tests/engine/utility_effects.rs`: the attached static preparation publication in
`utility_effects_r5_pending_revisions_keep_latency_changes_and_precompiled_plans_bounded`
uses snapshot preparation with Edit and a borrowed lease; all 33 standalone
frame/native-drop observations use `observe_and_retire`. The helper returns from
frame before creating/capturing/destroying its outer carrier. Controller's owned
R6 setup similarly uses `r6_publish_ready` at its two attached publication sites
and `r6_collect` at retirement observations. Detached precompiled/late-revision
setup remains legacy; R6 signal/owner/revision/allocator assertions are unchanged.
State's retained-owner test registers the actual generation through
reserve_departures before its second build; its original assertions are unchanged.

The first full engine run passed lib 130/130 and integration 280/281. Its sole
failure was
`sampler_reload_evicts_cache_but_old_voice_bank_stays_charged_until_control_retirement`:
the old second standalone transport query implicitly collected garbage and the
new accessor correctly retained both 12448 charged bytes instead of the asserted 6224. The parent granted exactly three lines after the second transport query
following run(1000,31): create ProjectRetirement, take_retired, drop the outer
carrier before the unchanged one-bank assertion. The earlier transport query
after run(500,13), which must retain both banks, is untouched. No other sampler
test or production sampler/cache source changed. The exact migrated case passes.

### Verification scope and remaining composition gates

The final engine runs use scripts/msvc-env.sh, one Cargo job,
target/p1-native and task-local target/p1-native/ts-exports. Focused readiness
proofs cover blocked constructors and concurrent control queries, stalled plan/
stream/rate/factory changes, actual-Ledger reuse versus staged revisions,
P20/G30/third-source refusal, delayed outgoing/first-heard/None selection,
selected history and inherited taps, late Edit/Replace transport, queue capacity,
memory/size/counter refusal, cancellation/poison/unwind/full retirement, native
failure/unavailability, bounded stale attachment retries and same-rate reopen.
Native fixture destructors check external document and recording mutex freedom
and controller mutex freedom. These are engine-owned fixtures, not actual Session
or helper-process integration proof. Device failure schedules model driver
build/play outcomes and consumed processor closures; they do not test CPAL
hardware or establish confirmed native helper joins.

Final source verification: lib 131/131 and engine integration 281/281, including
all 42 utility cases, all 18 P1 readiness proofs, all four R6 cases, sampler cases
and current T1 engine cases. Strict engine all-targets Clippy, owned Rust formatting,
document Prettier and diff/scope checks must pass before this source is frozen.
The initial strict run found five ordinary Clippy issues in new engine code;
they were fixed without lint suppressions or assertion changes.

Stage2 is still closed. Production Session edit/helper/import/New/Open/plugin
refresh and M1 borrowed eligibility/install/artifact acknowledgement have not
been converted. Attached static set_prepared_project explicitly reports
Unsupported("Attached project publication requires an engine readiness token.")
and leaves the engine plan installed; it has no native-under-lock fallback.
set_project is documented for callers outside document/recording guards.
Session's existing musical commit/error adapters and realtime retirement worker
must be composed before activation. N4 render/stems/factory/error-channel/helper
lifetime/capture integration remains its owner window. No UI artifacts, parity,
release, hardware deadline, listening or other-platform acceptance is claimed.
This checkpoint establishes the engine contract for review, not closure of the
production locking finding or global desktop compatibility.
