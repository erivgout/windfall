# Sampler N1 repairs

The independent review pinned `68a30a6195532b7efe9bc4bbf406507bca46fa3b`.
Repairs start from parent `186652c6f91463a629feadd9bf2641f54f958956` and integrate
`19caf872`, preserving its runtime capture/control acknowledgement, archive,
browser, MIDI and generated-artifact changes.

## Findings and closure boundaries

1. **File import commits before preparation can fail:** the sampler worker
   provides the guarded candidate API described below and its reduced-budget
   tests. The browser/import owner exclusively owns `session/library.rs`, all
   ordinary/checked destination routing, filesystem/provenance guards and import
   regressions. The helper alone does **not** close this finding. The combined
   routing commit and parent integration reproduction are required.
2. **Identical Apply discards successful preparation:** prepared runtime
   publication runs independently of musical `Touched`. A no-op retains the
   unchanged patch/revision and skips live document dispatch, preserving dirty
   state, undo, redo and even an open gesture. After recording refuses a reload
   publication, identical Apply now installs the new banks and plan. The barrier
   regression checks the complete document snapshot, audible sequence parity and
   float WAV export parity.
3. **History attaches newer cache audio than its prepared bank:** history keeps
   the original live source map separately from resolved candidate sources.
   Publication attaches exactly the prepared handles and marks them loaded before
   installing the plan. It never peeks a newer cache allocation at publication.
   Sources not resolved by preparation decode later and trigger their own guarded
   publication. Original pending loads are never filled from the cache or
   superseded by prepared publication. The real WAV/cache barrier regression removes a spectral channel
   and asset, pauses Undo after preparing A, replaces the cache with B through
   sample info, then requires restored A identity, audible sequence and float WAV
   parity against the frozen prepared source.
4. **Clip Apply misses newly attached sampler sources:** final validation compares
   source maps symmetrically, including additions, removals, replacements and
   distinct empty buffers. It also checks pending loads and document replacement
   tickets. The regression pauses clip preparation with missing spectral audio,
   reloads the file and publishes its sampler bank, then requires stale clip
   refusal without changing the newer source, bank, plan or document, including
   repeated note-on parity and float WAV export parity.

## Candidate API for the import owner

Take `Session::sample_edit_ticket(&State, Command, Option<u64>,
Vec<(SampleId, AudioBuffer)>)` under the existing recording → library → State
lock order. Build the batch and predicted source ID under those same guards.
The helper dispatches only a cloned document and snapshots generation, edits,
replacement tickets, the **original** source map and pending-loading IDs.
Candidate source overlays do not participate in comparison with the original
live map. A musical no-op ignores overlays except for the guarded settled-source
assignment recovery described in R2 below. A changed overlay for an original
pending-loading ID is refused. Cache resolution skips all original loading IDs,
including unrelated sampler assets.

Drop State/library guards before `ticket.prepare()`. This performs bounded
sampler DSP and complete plan compilation off State. It does not acquire
recording exclusion, allowing recording finish to keep its existing exclusion
contract while preparing a tape clip. No source/document/history mutation occurs
until preparation succeeds and final guards pass. The existing retained-bank
ledger is shared, with no oversized exception or new budget policy.

After preparation, the caller rechecks filesystem/root/version identity off
State, then reacquires recording → library → State. It must retain its existing
pending-load/provenance and project-directory guards. Call
`prepared.commit(&mut State)` only under that exclusion. The method revalidates
generation/edit/replacement/original-source/pending-load snapshots, dispatches
once and publishes exact handles. A changed command marks attached candidates
loaded and clears failed; it preserves other jobs' pending-loading state. A
no-op changes no musical history. Successful no-op bank repair is still published;
only verified settled absent assignments can additionally attach a decoded source.

Normal sample document edits use the same ticket taken before dropping their
initial locks, avoiding a second snapshot in a newly opened document. Sampler
and playlist background publication source checks use the same exact-map seam.
Editor/slicer already validate both source directions and retain their original
owned-file/review guards; their shared prepared publication now also attaches
exact handles. Open/load constructs a decoded source pool before preparation and
does not refetch sources at installation. Export snapshots remain immutable.

## Evidence and limits

All three owned findings were exercised in compiled RED tests before repair.
The first no-op RED failed complete snapshot equality (revision advanced); the
history RED failed prepared/restored source identity; the clip RED incorrectly
returned success after source reload. Logs are ignored local artifacts under
`target/n1-r1-{noop,history,clip}-red.log`.

The import owner identified a follow-up lifetime gap in the helper's resolution
of unrelated pending loads. Two additional compiled RED tests pause a real
decoder at `samples:decoded`, replace the path cache through sample info, then
publish a candidate edit or Undo. Both failed because publication superseded the
outstanding decode. Candidate/history resolution now skips loading IDs, and
source synchronization leaves their jobs in control. After releasing the
decoder, the tests require its exact held source and guarded sampler bank.
The RED log is `target/n1-r1-pending-load-red.log`.

The native tests use the cached worktree target, `CARGO_BUILD_JOBS=1`, one Cargo
process at a time and isolated temporary TypeScript exports. The helper fixture
uses `SamplePool::with_sampler_budget(256 * 1024)` with a retained audible bank,
rejects a larger candidate without mutation, compares note-on output bit for bit,
and verifies export readiness. It tests success, single undo, redo, no-op source
identity and redo preservation, plus generation/edit/replacement/source-map and
pending-loading guards. Browser-owned real-file import tests remain a distinct
integration dependency for finding 1.

The first helper handoff passed all **16 native sampler/session tests** on
`19caf872`, plus **42 sampler inspector tests** with two UI workers and UI lint.
Full UI typecheck at this parent fails because the merged piano rhythm commands
need the parent's fresh bindings (`ArpDirection`, `FlamPosition`, `RhythmMode`
and expanded `NoteTransform`). No sampler TypeScript/model command changed in
this repair. Artifact regeneration remains with the parent; no generated output
is included in these commits.

The pending-load follow-up passed all **18 native sampler/session tests**,
including the stronger audible/WAV history and clip checks, and strict scoped
Clippy (`windfall-engine`, `windfall-desktop`, all targets, `-D warnings`) plus
workspace formatting. Engine checks passed **49 sampler integration tests**,
**3 bank/budget unit tests** and the new exact-source-map unit test. Callback
allocator guards again measured zero allocations/reallocations/frees at first
notes, prepared range edges, cuts, stealing, reload, eviction and retirement.
The unchanged stationary sine fixture again measured **0.00002433 cent** worst
error; this remains synthetic evidence.
Publication compatibility checks passed **80 native tests** across document,
playlist, recording, editor/slicer, save/open and export. The final local logs
are `target/n1-r1-final-{sampler,clippy,fmt,native-compatibility}.log` and
`target/n1-r1-engine-{sampler,source-map}.log`. None are committed.

## R2: settled missing source recovery on identical assignment

The composed review at `6538722c` closed all four original R1 reproductions and
reported 22 sampler tests plus the 51-case import barrier passing. It identified
one additional shared helper/import P2, independently confirmed by browser R3:
restoring a missing file and replacing its channel with that same path produces
no musical edit, so the helper discarded its decoded source and playback stayed
missing. This is one finding shared by the two ownership boundaries.

The sampler repair starts from parent `9b3001f4`, merged as `ab320d1e` into this
worktree. Scanner `101c096e`, the composed exact-handle/pending-load/clip/budget
repairs, native runtime acknowledgement and portable-save changes are retained.
The only implementation change is in `session/sampler_processing.rs`; neither
`library.rs`, generic prepared publication, source-loader code, project schema
nor generated outputs are changed.

The helper signatures remain unchanged. For a musical no-op, a decoded overlay
can enter the private prepared pool only when all of these conditions hold:

- The command explicitly assigns that sample with `SetChannelSample`, directly
  or within a batch, and the candidate channel's final assignment matches it.
- The source is registered in the candidate document.
- The original source pool does not contain it, including an empty held buffer.
- No original pending decoder owns that source ID.

Source preparation/compilation still runs off State and uses the same strict
retained-bank ledger. The ticket records eligible recovery IDs separately from
the original source map. Final commit first revalidates generation, edits,
replacement tickets, exact original source map and loading set under the caller's
recording exclusion. It then attaches only those exact prepared handles, marks
them loaded and clears their failed flags before publishing the prepared plan.
It never performs generic cache synchronization for a no-op. Live document
dispatch remains skipped, so revision, dirty state, open gesture, undo/redo,
musical settings and the edit counter remain unchanged.

Already-held sources retain their identities. AddSample-only registration,
settings Apply, unrelated assignments and pending-source no-ops do not gain a
general source-attachment capability. Other pending loads retain ownership. An
over-budget preparation returns an error before attachment or publication; old
source/failure/history/plan/export state and audible voices remain intact.
Filesystem identity, root, recording, path-alias, provenance and folder checks
remain browser-owned at both ordinary and checked import routes.

The compiled helper RED at the merged parent returned success but failed at
`settled missing source was not recovered`. Its log is
`target/n1-r2-settled-source-red.log`. The browser owner separately reported all
six real ordinary/checked tape/spectral/budget recovery RED cases at `6538722c`;
those route tests are owned and run in the browser worktree.

The helper GREEN and all **22 owned sampler/session tests** pass with
`CARGO_BUILD_JOBS=1`, `RUST_TEST_THREADS=1`, the existing cached `target/n1-native`
and isolated temporary TypeScript exports. Four added tests cover tape and
spectral recovery with exact decoded identity, unchanged snapshot and a real redo
branch, supported keys, nonzero runtime and float WAV parity; retained 256 KiB
budget refusal with unchanged failure/source/history/export and continuing old
voice output; registration/settings/unassigned-overlay and held-empty exclusion;
and an actual pending-decoder barrier retaining its original source. Existing
R1 no-op Apply, original source-map guards and pending-load follow-ups also pass.
Strict desktop all-target Clippy (`-D warnings`) and workspace formatting pass.
Local logs are `target/n1-r2-{settled-source-green,sampler-tests,clippy,fmt}.log`.

Four narrow engine guards also passed: exact source-map/empty-allocation
identity, first-note/range-edge/cut/steal/reload/eviction/retirement allocator
counts, retained old-voice bank retirement, and supported/unsupported range plus
bit-identical export. Callback allocations, reallocations and frees remain zero.
Their logs are `target/n1-r2-engine-{identity,callback,retirement,range}.log`.

The browser owner confirmed composed closure of the shared recovery P2 after
cherry-picking helper `25ca8b0360eca45897e79273ff80392c1ae539f5` as
`c515e453e228c5a90c0a2e3f2ee21142dd1735c1`. All **35 native library tests** passed,
including the six actual ordinary/checked tape, spectral and budget-refusal
cases that compiled RED before the helper. Those fixtures perform real
save/New/Open and settled decoder failure, then restore the WAV. They verify
the exact recovered handle, cleared failure, unchanged full snapshot, edits and
redo, supported keys, nonzero playback and bit-identical runtime/export output.
All 51 earlier prepared-import race cases and pending reload/redo remained GREEN.

This composed evidence was executed and reported by the browser owner, separately
from the sampler-worktree checks above: **62 scoped native tests**, **268 UI
tests** and strict desktop all-target Clippy passed. The report describes the
composed state after the helper cherry-pick; the browser's final source commit
was still pending when this evidence was recorded. No library routing or helper
source changes were needed for those recovery tests. No protocol, schema or
artifact regeneration is required for this helper-only repair. No full
UI/workspace loop was rerun in the sampler worktree for this change.

No DSP algorithm, prepared range policy, callback allocation path, tape geometry,
plugin runtime/provider, archive format, MIDI panic policy or generated bindings
is changed in this round. Musical listening, installed UI/audio-device operation
and non-Windows verification remain external evidence requirements. Sine
estimates and bit parity establish their measured properties; they do not prove
musical transparency. Formants remain approximate and the documented transient,
partial/noise/sub-bass/extreme-pitch limitations still apply.
