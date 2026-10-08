# Independent sampler duration and pitch

The sampler retains tape playback by default. An explicit spectral mode stores
duration ratio (0.25–4), Fast/Standard/High quality, approximate formant
preservation and an inclusive MIDI key range. The inspector initially requests
twelve keys around the root (root minus six through root plus five, shifted at
MIDI boundaries). A user can request any ordered range within MIDI 0–127.
These musical settings save in v1 projects; tape defaults remain omitted.
Prepared audio is derived runtime data and never enters a project file or undo
entry. Each Apply produces one checked sampler edit; repeated settings are a
no-op. Filters and LFOs remain outside this feature.

## Preparation and playback

All requested keys render before publication. Trim uses the tape path's rounded
source frame boundaries. Reverse happens before spectral processing. Each
variant has `max(1, round(trimmed_frames * ratio))` frames at its source rate,
independently of the key. A contraction of one frame remains one frame.
Pitch is `key - root + tune`, prepared with the existing Rust stretch DSP.
Its individual-pass pitch bound is ±24 semitones; wider shifts use successive
duration-preserving passes and a final duration pass. Validated MIDI/root/tune
settings need at most eight passes. Extreme shifts can lose audible partials
through Nyquist limits, and repeated spectral passes compound artifacts.

An immutable table has 128 indexed slots. Note-on performs one indexed lookup,
clones existing handles and plays the selected variant at source/output sample
rate, without key-dependent resampling, cache locking or rendering. A key outside
the published range is silent and does not cut a sounding voice. The inspector
shows the range and silent-key policy; `Controller::sampler_key_supported` is the
control-side capability query. Missing/empty decoded sources remain silent.
Spectral preparation accepts mono/stereo sources at 8–192 kHz and reports other
formats instead of silently downmixing. Browser previews retain their one-shot
path. The browser mock can preserve/read musical settings, but its preparation
operation explicitly refuses DSP and leaves settings/history intact. Loaded
spectral settings also report unavailable browser audition/export.

Loop points retain their source-order normalized positions within the trim.
They round against the rendered length, with at least one frame. Reverse mirrors
that rendered interval, because the prepared buffer already runs in reverse.
Lead-in, Forward/Ping-pong interpolation, repeated traversal during release and
the absence of an outro follow the existing voice implementation. Note ends and
ADSR use the existing musical tick/envelope semantics. Looping without an
explicit envelope retains the 4 ms release; a one-shot without an envelope still
ignores note length. Velocity/pan, cut-self/groups and stealing remain unchanged.
Tape trim, interpolation, pitch, reverse and loop behavior use their original path.

## Strict retained-bank budget

The sampler budget is **256 MiB**, independent of the playlist cache. There is
no oversized-bank exception. Reservations count all rendered key buffers, bank
table storage and the full source retained by each bank. Shared sources across
different banks are conservatively counted for each bank. During preparation,
reservations also cover the trim/intermediate and output audio staging buffers.
FFTs use the existing fixed quality presets; their separate working storage is
bounded by the mono/stereo and 192 kHz capability limit. A cancellable worker
lock shared by the reservation ledger allows one FFT workspace at a time across
Apply, open, reload and export jobs. Waiting reservations remain charged; a
cancelled waiter releases its reservation without constructing DSP. This is a
cache/audio reservation bound, rather than a promise that the whole application
uses 256 MiB.

Pool snapshots share a reservation ledger. Project replacement adopts that
ledger, so an export, pending job, old plan or old voice keeps its bank charged
across a document change. Voices retain entire banks, not just a selected buffer.
Retirement transfers their last bank handle to the existing control-side garbage
queue. No bank allocation or destruction occurs in the callback.

A candidate cache contains only banks requested by its candidate project and
reuses matching source/settings banks. Source handles stay alive with the key,
preventing allocator pointer reuse from matching stale audio. Accepted tape edits
prune unused cache references. Plans, old voices and jobs still retain their
reservations after cache eviction. A replacement may therefore require room for
both old and new banks. A refusal reports requested/retained/limit bytes; reducing
the range/duration or allowing old voices/plans to retire releases pressure.
Cancellation/failure/stale publication drops private candidate buffers on the
worker and leaves the musical edit and live cache unchanged.

## Session lifecycle and cancellation

Preparation snapshots the checked candidate document, pool, generation, edit
count and replacement request. It performs sampler DSP and plan compilation
outside document/recording locks. Publication rechecks recording exclusion,
request cancellation, generation, edits, replacement request and exact source
identities before dispatching the musical command and installing the bank/plan.
Cancellation and final publication linearize under the document lock.
Cancellation is checked before each key/pass and each 4096-frame offline chunk;
one FFT construction or processing chunk must finish before it can be observed.

Ordinary native commands that change a spectral bank use the same guarded path.
Open/load and undo/redo prepare their target snapshots before installation.
History preparation refuses a replaced document and bounds edit/source retries.
Sample attachment/reload prepares on a coalesced background worker; budget or
capability failures retain the last audible plan and emit a project warning.
An already-running playlist worker routes sampler cache misses through this
guarded sampler worker. Native export refuses a current source whose variants
have not been published, so a failed/pending reload cannot render new audio
while realtime still retains the old plan. Export snapshots retain their own
published-bank cache references, so a later live-cache eviction cannot release
their reservation or require rendering those banks again.
Realtime playback, buffered render and streaming/stem export use the same key
range and preparation policy. Checked preparation and streaming APIs expose
errors; the convenience controller publication preserves its prior plan and
offers `sampler_preparation_error`. `Rendered::sampler_error` reports buffered
render preparation failure.

The inspector owns only its draft/request. Cancel, channel selection, project
replacement or another document patch invalidates the pending draft request.
It displays key progress and native errors and applies successful document
patches through the existing store. No derived buffer or job state is persisted.

## Verification and quality limits

Verification was run on Windows in this isolated worktree with one Cargo process,
`CARGO_BUILD_JOBS=1`, a worktree-local target directory and isolated temporary
TypeScript exports. Results below are local engineering evidence.

- Project library/integration suites passed, including the three new persistence,
  validation and history tests.
- Engine suites passed all **106 unit and 223 integration tests**. The final
  sampler filter passed 49 tests, and bank/queue unit guards passed three tests.
- Stretch passed 7 unit and 47 integration tests; four intentional ignored tests
  are measurement/probe/example-render runners. DSP passed 124 integration tests
  with two intentional benchmark/example-render ignores.
- Native sampler barrier tests passed all **11 tests**. The full session suite
  passed **156 tests**, including actual CLAP scanner/playback/export/state and
  ownership fixtures, editor/slicer and recording regressions. The compiled
  native test executable ran directly after the Cargo build, so fixture helpers
  could build sequentially without an enclosing Cargo test process.
- Workspace formatting and scoped Clippy with `-D warnings` passed for project,
  engine, desktop and stretch, including all targets.
- UI typecheck, lint, scoped Prettier and **89 inspector/rack tests** passed with
  `--maxWorkers=4`. Seven new UI tests exercise sampler preparation/capability.
- Generated 152 local binding/fixture files and a 1,740,715-byte document WASM;
  `check-bindings.mjs` and `check-sim.mjs` passed. These artifacts are excluded.

The sampler's independent zero-crossing estimator measured worst error
**0.00002433 cent** in the stationary 440 Hz fixtures at ±12 semitones and
ratios 0.5/1/2, below the 0.1-cent sine bound. The log is a local ignored artifact
at `target/n1-sampler-verification.log`; native session output is at
`target/n1-native-session-verification.log`.

Fixtures cover independent zero-crossing frequency estimates at ±12 semitones
and duration ratios 0.5/1/2, intended lengths, block/export parity, range edges,
unsupported notes without cutting a supported voice, trim/reverse, transformed
fractional and one-frame loops, release/cuts/stealing, missing/empty/reloaded
sources, budget/cancellation and retirement.
Allocator guards count allocations, reallocations and frees during processing.
Source-retention and aggregate-reservation tests cover pointer reuse and queued
worker cancellation. Project and session tests cover legacy/defaults, invalid
settings, one-step history, save/load and preparation barriers. UI tests cover
mock refusal, Apply/undo, cancel and selection/project changes while work is pending.

These fixtures do not establish musical transparency or listening quality.
The existing DSP's documented limits apply: dense partials can interfere, attacks
can smear/change energy, shifted noise can phase/lose level, sub-bass is rolled
off in transformed paths, and formant preservation approximates a voiced spectral
envelope. Very short buffers and extreme pitch passes provide little frequency
information and may become quiet. High quality increases update density rather
than frequency resolution. Arbitrary loop selections can click; no crossfade is
introduced. See `crates/windfall-stretch/VALIDATION.md` for measured DSP fixtures.
Real musical listening, installed native UI/device operation, and non-Windows
verification remain external evidence requirements.

Integration must regenerate all TypeScript/descriptors and the Rust document
WASM after merging the sampler source with the piano/editor/slicer work. Local
generated bindings/WASM and build/test output stay excluded from this feature
commit. Narrow shared seams include sampler model/commands/checking, controller,
IPC/backend registration, session publication/history/open/export, and the
fallible preparation-call adaptation in editor/slicer; their behavior is otherwise
retained. The DSP change only adds bounded cancellation to the existing offline
renderer, without changing its uncancelled algorithm.
