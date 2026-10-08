# Native analysis foundation (M1 first stage)

`windfall-analysis` is a native control/worker crate on the accepted A-EDITOR
source contract at `1e24389eb58f4d59f3c0a121d946e1372ef7e585`. It does not edit a
project, connect to the engine, expose IPC, or ship an inference adapter. Its
authored test adapter copies selected PCM on real CPU workers and writes real
files. It proves lifecycle only. M1 Session/IPC/UI integration and M2–M4 actual
separation, denoising, pitch detection/editing and audio-to-MIDI remain required.
No parity row is closed by this delivery.

## Input and exact source checks

`CapturedInput::capture` runs away from recording/State/audio locks. It validates
shape, nonempty inclusive-start/exclusive-end range, checked frame/sample/byte
arithmetic and every finite sample. It retains a compact owned copy of the
whole immutable input; the selection is a checked slice of that copy. Copying
avoids retaining an external `Arc<Vec<f32>>` with hidden excess capacity. The
observed allocated capacity, rather than just selected frames, is charged.
No buffer is cloned into a status response. Ready review retains the original
captured PCM until cancellation or completed control-side retirement.

The Session supplies a canonical `CaptureStamp`:

- Document generation and edit revision, exact logical source key and binding
  digest, loaded original `AudioIdentity`, content fingerprint, prepared input
  frame/channel/rate shape, and exact selected range.
- `binding` must cover the accepted exact logical source and clip/render
  settings, including offset/reverse/stretch/pitch/gain/pan/fades/routing and
  timeline placement as appropriate. A digest of a pathname alone is inadequate.
  Canonical binding rules belong to the later Session adapter, which must
  preserve existing exact source/history guards.
- `ContentFingerprint::reader` hashes streamed actual file bytes with SHA-256
  and a byte cap. `ContentFingerprint::audio` hashes a versioned prefix, little
  endian sample rate/channel/frame fields and exact Float32 bits. File bytes
  and decoded PCM use different fingerprint kinds. Sizes/mtimes are never
  treated as content hashes. Paths do not enter the content fingerprint; path
  aliases to the same bytes compare equal, same-size changed bytes do not.
- A supplied verified fingerprint is a caller contract. For file input, the
  later loader must fingerprint the **exact file version decoded** into its
  loaded source and immutable view; independently hashing a mutable path after
  decoding does not establish that identity. Capture/recheck IO occurs off
  State. The final Session check must still compare the current loaded exact
  source allocation and canonical binding under its guards.

`Review::check_eligibility` compares all stamp fields and the entire model
manifest, including revision, checksum, provenance and shape, and requires a
live current source identity. It performs no IO and proves no atomic document
installation. Stale/refused work leaves the immutable review available until
explicit cancellation, consumption or shutdown.

## Execution and resource accounting

`JobManager` owns fixed real worker threads, a condition-variable queue and
bounded retained records. `submit`, `snapshot`, `wait`, `wait_for_change`,
`review`, `cancel`, `forget`, `claim`, `usage`, `retry_cleanup`, `shutdown` are
control-side operations. They must never run on audio or under Session State
or controller locks. The adapter has no mutable document or engine pointer.

Default limits: two workers, four queued jobs, sixteen total retained records,
256 MiB admitted memory, 1 GiB staging/publication/retained disk, 256 retained
output files, 64 MiB adapter scratch per job, 4,000,000,000 work units per job,
ten-minute execution/apply timeout. Input defaults cap whole captures at
64 MiB, 23,040,000 frames, mono/stereo and 8,000–192,000 Hz. Model cache defaults
cap a model at 256 MiB, cache at 1 GiB, 32 directory entries and one import.
Callers may configure tighter bounds. No CUDA/Python installation is required.

Each admission reserves:

- Actual compact input capacity, manifest's exact model bytes, declared adapter
  scratch, one largest decoded output PCM buffer, a 2 MiB codec/IO allowance,
  and 64 KiB for its retained record/provenance/metadata.
- Twice the output byte budget for private staging and no-clobber publication,
  plus the declared number of aligned role files. Already published bytes/files
  stay charged even after job removal. Existing files in the dedicated output
  directory are included when the manager starts.
- Every queued/running/ready/apply-claimed/retiring input, scratch or output
  reservation. Consumption cannot release reservations before the deferred
  input/staging retirement actually finishes. Completed status records keep
  their fixed memory charge until explicit `forget`.

Adapters are trusted native implementations, configured once per manager;
requests cannot enqueue arbitrary adapter objects holding unaccounted audio.
They must use bounded provided scratch and checkpoints, declare any runtime
resident memory in scratch and add adapter-specific measurements. This is
cooperative admission accounting, not an OS memory sandbox. Existing codec
internals have their own guarded metadata/channel/rate limits and bounded PCM
sink; the allowance is not proof of an arbitrary codec/model runtime's maximum
RSS. A future adapter needs its own allocation/peak-memory audit and CPU deadline
measurements. Blocking filesystem calls and a codec decode cannot be preempted
mid-call; cancellation/deadline is checked before/after them and between bounded
sample/byte chunks. No hard realtime latency is claimed.

Statuses are `Queued → Running → Ready → Consumed`, with cancellation through
`Cancelling → Cancelled` and failures to `Failed`. Queued and ready work can be
cancelled without running an adapter. Running cancellation is cooperative and
cannot publish Ready after its cancellation wins the control lock. Consumed
cancellation is a no-op. A claimed apply refuses cancellation/removal, preventing
file retirement while a caller commits. Failure/cancellation releases buffers
and staging off the manager lock before releasing their reservations.

Job, review ticket and request IDs increase without reuse, including after
forgetting. ID allocation refuses before integer rollover; checked arithmetic
refuses oversized shape/budget/deadline calculations. Status/progress sequence
numbers increase, work never regresses/exceeds its budget, progress is coalesced
instead of appended to an unbounded event list, and transition history has at
most eight entries. Each apply gets a new request ID. Work exhaustion can refuse
an apply retry; it cannot create an unlimited stream of retained outputs.

Shutdown serializes joining, stops dequeuing once shutdown starts, cancels
queued/ready jobs and asks active workers to stop, then joins off all Session
locks. Outstanding apply/retirement owners keep the shared accounting alive.
An apply dropped during shutdown retires staging; an acknowledged real commit
retains its sources. `forget` accepts only failed/cancelled/consumed records with
finished retirement. Ready work must be explicitly cancelled or consumed.

## Explicit local models

`ModelManifest` fixes ID, version, revision, exact SHA-256 and byte length/limit,
source revision/origin/author, weight SPDX/license reference, adapter ID/version,
CPU device, exact input channels/rate/frame cap and one to sixteen unique output
roles with channel counts. Provenance strings and roles are bounded. Weight
terms are separate from runtime/code terms. Nothing searches for guessed
weights, downloads or uploads. Missing models return an explicit import/offline
error. There is no production adapter disguised as successful inference.

`ModelCache::import_reader` / `import_file` reserve bounded import count/cache
disk, stream into an owned `NamedTempFile`, validate exact size and SHA-256,
sync and publish with `persist_noclobber`. Cancellation before publication
removes only staging. Publication is the import commit point; cancellation
after it cannot remove cache data. Existing and concurrently won entries are
read and rehashed rather than trusted by name/metadata. Invalid existing entries
are refused and preserved; users must explicitly repair their own cache.
`load` rehashes a bounded compact immutable model-byte copy on the worker before
the adapter runs, avoiding inference against a changing cache pathname.

Cache and output roots are dedicated single-control-authority directories;
cache clones share reservations. Symlinks/non-files in cache/output inventory
are refused. Independent authorities/external arbitrary writers are not an
interprocess quota system or a hostile filesystem security boundary. Collisions
preserve competitor bytes. The three ownership roots must be disjoint, with
fresh exclusively owned staging. Crashed staging is refused on next startup,
never speculatively garbage-collected.

## Artifacts and publication/commit protocol

`ArtifactWriter` creates a unique private job directory. Portable flat relative
names reject separators/drives/ADS/traversal/dots/spaces/device names. Outputs
must be Float32 WAV, one per declared role, exactly aligned selected frame count
and original frame origin, at the manifest rate and role channel count. Chunk
writes cap at 4096 whole-frame finite samples. Disk/count/byte/frame overflows,
nonfinite data, missing/extra/duplicate roles and bad paths are refused. Encoded
helper output first streams into bounded owned staging, then is decoded and
normalized to this contract. Completed WAVs are independently decoded for shape
and finiteness and hashed before review. No output PCM is retained between files.
Codec replacing-writer APIs see only private staging, never persistent targets.

The Session integration must follow this protocol:

1. Capture a canonical source/view/stamp with recording exclusion before State;
   perform view preparation, fingerprinting/copying/admission off all those
   guards, then recheck exact source/document before treating the job as current.
2. Poll/await status; get a bounded review with capture/model/output metadata.
   Review itself cannot edit anything.
3. `claim(ticket, current_stamp, manifest)` checks pure eligibility and gives one
   `ApplyLease`. `prepare(relative_names)` copies verified staging into owned
   temporary output files, syncs and atomically publishes each with no replacement.
   Caller output names must be uniquely chosen. There is no multi-file atomic
   publication claim; a later collision can leave earlier final files retained.
4. Decode published outputs, prepare the ordinary pool/checked command batch and
   `Controller::prepare_project` off State. Recheck actual file content and
   loaded source state as required by the Session source contract.
5. Acquire recording exclusion **before State**, recheck capture/source/request,
   exact clip and every prepared pool source, then dispatch **one checked batch**
   using the existing prepared-plan publication/retirement path. A recording,
   project replacement, stale edit/model/source or rejected batch changes no
   document/history/pool/engine state. Pure crate tests do not prove this step.
6. Only after the actual caller commit, call
   `PreparedApply::acknowledge_commit`. This infallible transition does no IO,
   codec work or worker joining. It returns `AppliedAssets` with deferred
   retirement. Release all Session/recording guards, then `retire`/drop the assets.
   Their final files remain external sources for undo/redo/save/archive.

Dropping a lease/prepared apply after failed commit returns Ready, retaining
input/staging and any already published metadata. The first attempted destination
set is pinned; retries reuse the same validated final files rather than generating
more orphans. Cancellation after Ready removes only staging, even after partial
publication. Published files are never deleted on failure/stale/refusal/cancel,
job forgetting, controller retirement, shutdown or undo. Final bytes/files are
permanently charged for the manager lifetime and re-inventoried on restart; this
finite budget can require another directory/explicit user-owned recovery. There
is no speculative source GC or claim that a failed publication/commit is free.

If staging cleanup fails, its disk/file reservations remain charged, the record
reports failure and cannot be forgotten. `retry_cleanup` is explicit, bounded
to that job's private child directory and refuses a replacement file/symlink.
It never points at cache/persistent roots or another job. No automatic cleanup
can unlink committed or competitor files. Ordinary Drop cleanup is a fallback;
resource/accounting retirement also verifies whether the owned directory remains.

## First-stage verification and next exclusive window

Tests live in auto-discovered `tests/analysis_jobs.rs` and library unit tests.
The authored CPU fixture contract is in `tests/fixtures/README.md`; model SHA-256
is `07f6812eb7ac451adc4b010bb18b2fe098ce235498880c80ead3fad519e47d33`.
The exact 74-byte Float32 WAV SHA-256 is
`4dbb9f64ee9794914f7a9bf01ed95c3775858a4bb9413877a99218b88077528b`.
Barrier seams exercise native queue/active cancellation and shutdown without
timing sleeps. Tests inspect actual decoded files, retained bytes and directories.
They cover whole-input capacity/bounds/NaN/Inf/ranges, queue/count/concurrency,
memory/disk/work/integer overflow, monotonic transitions, absent/corrupt models,
stream read/checksum/size/cancellation errors, output codecs/IO and alignment,
ready/claimed/consumed cancellation, stale stamp/model revisions, no-clobber
collisions, refusal/retry/double consume and staged versus persistent lifetime.
No hardware, packaged app, Session undo/save/archive, browser UI or real inference
verification is claimed by these crate tests.

Run one Cargo at a time in Git Bash after `source scripts/msvc-env.sh`, with
`CARGO_BUILD_JOBS=1`, `RUST_TEST_THREADS=1`, `CARGO_TARGET_DIR=target/m1-native` and
task-local `TS_RS_EXPORT_DIR`. Cargo generated only the new windfall-analysis
lock entry, referencing already locked packages; subsequent checks use
`--locked --offline`. Verify only owned Rust formatting and strict crate
`--all-targets` Clippy. No full desktop/workspace/UI build or binding generation.

Proposed next child ownership, after acceptance and explicit parent grant:
new `session/analysis_jobs.rs`, `session/tests/analysis_jobs.rs`, IPC `analysis.rs`
and `features/analysis/*` with native source/history/save/archive and UI review
tests. Serialized registration window: workspace analysis dependency entry,
desktop Cargo dependency, `session/mod.rs` module/Inner construction/drop,
IPC lib export, native commands/lib invoke registrations, Backend/Tauri/mock
methods and audio clip-inspector action entry. Existing editor/edit/library/
files/clip_processing, sampler/cache and Document exact-source/history internals
remain reserved. Parent owns generated TS/WASM, shared root documentation/parity
and final integration/publication. Mock must honestly report native inference
unavailable. No production adapter is available for a user-visible successful
analysis until a separately reviewed actual algorithm/model is integrated.

## Dependency license evidence

The only dependency graph addition is the native analysis package; sha2 0.10.9
and tempfile 3.27.0 already existed in Cargo.lock. Default features only (std;
tempfile randomness via existing getrandom). No runtime, FFI model binary,
network library or weights were added. Existing windfall-core/codec/serde/
thiserror are reused unchanged; preserve codec's existing Symphonia MPL-2.0,
Vorbis BSD-3-Clause and LAME LGPL obligations described in its manifest.

Local primary license files from the exact locked registry source were inspected
on 2026-10-08. The added references use the MIT option, compatible with this
GPL-3.0-or-later native app; MIT notices must accompany distributions. A table
of exact primary-file hashes and attribution follows. Platform-transitive
licenses are recorded even where this Windows build does not select them.

| Locked source | License option / primary file | Primary SHA-256 | Attribution |
| --- | --- | --- | --- |
| [sha2-0.10.9](https://github.com/RustCrypto/hashes) | MIT / LICENSE-MIT | b4eb00df6e2a4d22518fcaa6a2b4646f249b3a3c9814509b22bd2091f1392ff1 | Copyright (c) 2006-2009 Graydon Hoare; Copyright (c) 2009-2013 Mozilla Foundation; Copyright (c) 2016 Artyom Pavlov |
| [tempfile-3.27.0](https://github.com/Stebalien/tempfile) | MIT / LICENSE-MIT | 8b427f5bc501764575e52ba4f9d95673cf8f6d80a86d0d06599852e1a9a20a36 | Copyright (c) 2015 Steven Allen |
| [cfg-if-1.0.5](https://github.com/rust-lang/cfg-if) | MIT / LICENSE-MIT | 378f5840b258e2779c39418f3f2d7b2ba96f1c7917dd6be0713f88305dbda397 | Copyright (c) 2014 Alex Crichton |
| [cpufeatures-0.2.17](https://github.com/RustCrypto/utils) | MIT / LICENSE-MIT | ae9baa7beea910273c2f384c2a6b721fb7bd02bda3436074a1072e4ee689f985 | Copyright (c) 2020-2025 The RustCrypto Project Developers |
| [libc-0.2.190](https://github.com/rust-lang/libc) | MIT / LICENSE-MIT | 123a331b5dbf04c30097fa43b8f858bc85df671fe776de498d01f3d6b7c1f69e | Copyright (c) The Rust Project Developers |
| [digest-0.10.7](https://github.com/RustCrypto/traits) | MIT / LICENSE-MIT | 9e0dfd2dd4173a530e238cb6adb37aa78c34c6bc7444e0e10c1ab5d8881f63ba | Copyright (c) 2017 Artyom Pavlov |
| [block-buffer-0.10.4](https://github.com/RustCrypto/utils) | MIT / LICENSE-MIT | d5c22aa3118d240e877ad41c5d9fa232f9c77d757d4aac0c2f943afc0a95e0ef | Copyright (c) 2018-2019 The RustCrypto Project Developers |
| [crypto-common-0.1.7](https://github.com/RustCrypto/traits) | MIT / LICENSE-MIT | 3521672491a3479422d5fe1aca6645dd2984090f85da6e5205abfb18fb7a6897 | Copyright (c) 2021 RustCrypto Developers |
| [generic-array-0.14.7](https://github.com/fizyk20/generic-array.git) | MIT / LICENSE | c09aae9d3c77b531f56351a9947bc7446511d6b025b3255312d3e3442a9a7583 | Copyright (c) 2015 Bartłomiej Kamiński |
| [typenum-1.20.1](https://github.com/paholg/typenum) | MIT / LICENSE-MIT | a825bd853ab71619a4923d7b4311221427848070ff44d990da39b0b274c1683f | Copyright (c) 2014 Paho Lurie-Gregg |
| [version_check-0.9.5](https://github.com/SergioBenitez/version_check) | MIT / LICENSE-MIT | b7e650f3fce5c53249d1cdc608b54df156a97edd636cf9d23498d0cfe7aec63e | Copyright (c) 2017-2018 Sergio Benitez |
| [fastrand-2.5.0](https://github.com/smol-rs/fastrand) | MIT / LICENSE-MIT | 23f18e03dc49df91622fe2a76176497404e46ced8a715d9d2b67a7446571cca3 | No separate copyright line in this primary text; retain its notice. |
| [getrandom-0.4.3](https://github.com/rust-random/getrandom) | MIT / LICENSE-MIT | 523a42c25d245dde9c015f882cec7f4555aad883382a6cf19b4b7d9b2cd5419b | Copyright (c) 2018-2026 The rust-random Project Developers; Copyright (c) 2014 The Rust Project Developers |
| [r-efi-6.0.0](https://github.com/r-efi/r-efi) | MIT / AUTHORS | d027e91dbc9cdbb2f1190068e498bd6b61cff022b6a032b191021ba658d96111 | Copyright (C) 2017-2023 Red Hat, Inc.; Copyright (C) 2019-2023 Microsoft Corporation; Copyright (C) 2022-2023 David Rheinsberg |
| [once_cell-1.21.4](https://github.com/matklad/once_cell) | MIT / LICENSE-MIT | 23f18e03dc49df91622fe2a76176497404e46ced8a715d9d2b67a7446571cca3 | No separate copyright line in this primary text; retain its notice. |
| [rustix-1.1.5](https://github.com/bytecodealliance/rustix) | MIT / LICENSE-MIT | 23f18e03dc49df91622fe2a76176497404e46ced8a715d9d2b67a7446571cca3 | No separate copyright line in this primary text; retain its notice. |
| [bitflags-2.13.2](https://github.com/bitflags/bitflags) | MIT / LICENSE-MIT | 6485b8ed310d3f0340bf1ad1f47645069ce4069dcc6bb46c7d5c6faf41de1fdb | Copyright (c) 2014 The Rust Project Developers |
| [errno-0.3.14](https://github.com/lambda-fairy/rust-errno) | MIT / LICENSE-MIT | 8764a597675778ddfd4e25f81b08a05dbcf089ac05662df7613fe67f150e3aa2 | Copyright (c) 2014 Chris Wong |
| [linux-raw-sys-0.12.1](https://github.com/sunfishcode/linux-raw-sys) | MIT / LICENSE-MIT | 23f18e03dc49df91622fe2a76176497404e46ced8a715d9d2b67a7446571cca3 | No separate copyright line in this primary text; retain its notice. |
| [windows-sys-0.61.2](https://github.com/microsoft/windows-rs) | MIT / LICENSE-MIT | c2cfccb812fe482101a8f04597dfc5a9991a6b2748266c47ac91b6a5aae15383 | Copyright (c) Microsoft Corporation. |
| [windows-link-0.2.1](https://github.com/microsoft/windows-rs) | MIT / LICENSE-MIT | c2cfccb812fe482101a8f04597dfc5a9991a6b2748266c47ac91b6a5aae15383 | Copyright (c) Microsoft Corporation. |

Retain the attribution above and each exact package license in distribution notices.
rustix and linux-raw-sys also carry COPYRIGHT files retaining contributor copyrights.
r-efi uses the MIT section of AUTHORS; its UEFI backend is not selected on Windows.
The common MIT permission/warranty text is reproduced here from sha2/LICENSE-MIT:

```text
Copyright (c) 2006-2009 Graydon Hoare
Copyright (c) 2009-2013 Mozilla Foundation
Copyright (c) 2016 Artyom Pavlov

Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
```
