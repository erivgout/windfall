# Analyzer tap foundation — first T8 packet

Implemented against base `9c909879d8f93022085addf68f069d52cfe945b2` on
`gpt/t3-analyzer-taps-t8`. This packet provides a bounded, independently tested
stereo PCM transport and analyzer. It is exported as
`windfall_engine::analyzers`; it does **not** register engine taps, desktop IPC,
subscriptions, or drawn views. Tests supply authored PCM through the production
interface. Native helper threads are exercised without opening an audio device.

The persistent project model, existing meters, PDC processing, recording/history
guards and native ownership are unchanged. The only shared changes are the module
export, engine's direct `rustfft = "~6.4.1"` dependency, and its one dependency
reference in the lockfile. The existing lock resolves RustFFT **6.4.1** and rtrb
**0.4.0**; no package version, resolution or other dependency changed.

## Interface and supported topology

`prepare(Selection, Config)` returns
`Result<(AudioTap, AnalyzerWorker, SnapshotReader), PrepareError>`. Preparation
and every endpoint's eventual destruction run off the audio callback and outside
document/recording locks. Preparation never takes those locks itself.

Each tap accepts one stereo stream, with an immutable exact `Selection`:

- `ProjectGeneration` identifies the project/replacement generation.
- `SelectionGeneration` identifies the request/replacement of the selected tap.
- `Source` is `MixOutput` or `TrackPostFader(TrackId)`; `TrackId` is the existing
  typed project ID. These are intended future hook locations, not installed hooks.

The caller must issue generations without reuse, validate track existence against
its current project, and refuse exhaustion rather than wrap. Preparation does not
look up a project or infer a source from a buffer. A prepared tap cannot be
relabeled: a mismatching selection is refused and invalidates its old evidence.
One tap has one producer and one worker; a bounded slot stages replacements.
This packet has no bus/channel/plugin/browser/GPU tap topology, mono adapter,
source aggregation, selectable decimation or filter-response spectrum.

`AudioTap::publish(Stamp, &[[f32; 2]])` returns `Result<(), Refusal>`. It copies
at most **256 stereo frames** per call. Larger calls are refused whole. Future
engine callers must explicitly partition larger blocks and advance `first_frame`.
An empty call validates identity/rate/frame arithmetic but queues nothing and
does not establish continuity. `AudioTap::reset()` invalidates all earlier
stream evidence without clearing or destroying buffers on audio.

`Stamp` carries the exact selection, typed `ClockEpoch`, integer `first_frame`,
sample rate, source PDC latency, optional device latency, and optional actual
`GainReduction { effect: EffectId, db }`. Supported rates are **8,000–384,000 Hz**.
Each supplied latency is **0–one second** at that rate; unknown device latency is
`None`. Frame-end arithmetic is checked. Seek/loop/restart callers must advance
the clock epoch even if consecutive numeric frame positions happen to match.

`AnalyzerWorker::pump()` deterministically consumes at most `input_packets`,
including discarded stale packets, and returns a `PumpReport` of packets,
published snapshots and stale packets. It belongs on a worker/control thread.
Alternatively, `AnalyzerWorker::spawn()` consumes it and returns a native
`WorkerThread`. `SnapshotReader::poll(expected_selection)` drains at most the
configured snapshot capacity and returns an optional **borrowed** `Snapshot`.
No snapshot clone or detached heap owner is exposed. A reader holds at most one
snapshot, and the borrow cannot survive its next mutable poll.

For callback-safe ownership installation, `tap_slot()` returns
`(AudioTapSlot, TapInstaller)`. Control stages a prepared producer with
`TapInstaller::stage`; audio calls `AudioTapSlot::boundary()` at a block boundary
and publishes through `slot.active()`. Control repeatedly calls
`collect_retired()` outside locks. `TapInstaller::clear()` stages deselection.
These operations provide installation, not any existing Processor registration.
The production-interface tests show preparation, staged installation, publication,
pumping/reading, replacement and native shutdown without private test injection.

## Configuration, memory and work bounds

The entire configuration surface is `fft_size`, `hop_size`, `window`,
`input_packets`, and `snapshot_capacity`.

| Quantity | Supported bound/default | Behavior at the bound |
| --- | --- | --- |
| FFT size `N` | Power of two, 32–4096; default 1024 | Other values return `FftSize`; no rounding |
| Hop `H` | Exactly `N/4`, `N/2`, or `N`; default 256 | Other values return `HopSize` |
| Window | Rectangular or periodic Hann; default Hann | Explicit enum, no custom window allocation |
| Input packets `Q` | 1–64; default 32 | Invalid capacity returns `InputCapacity`; full drops/refuses whole publication |
| Ready snapshots `S` | 1–8; default 2 | Invalid capacity returns `SnapshotCapacity`; full drops newly analyzed snapshot |
| PCM/publication | 0–256 stereo frames | Above 256 returns `TooManyFrames` without scanning/copying that block |
| Time history | Last at most `2N` contiguous stereo frames | Oldest frame replaced as new frames arrive; reset clears logical extent |
| Envelope | At most 128 bins over that history | All retained frames partitioned into bins; no silent sample truncation |
| Vectorscope | At most 128 uniformly sampled current-window points | This explicit finite summary omits sampled invalid points |
| Spectrogram | Last 8 analysis windows | Oldest slice replaced; gap/reset clears logical extent |
| Snapshot objects | Exactly `S + 2`, each fully prepared | One worker spare, at most one reader-held object, bounded ring ownership |
| Live taps | At most 8 per process, including staged/retired/cancelled owners | Further preparation returns `InstanceLimit` |
| Tap slots | At most 8 per process | Further slot preparation returns `SlotLimit` |
| Aggregate charged reservation | At most 32 MiB across taps and slots | Further preparation returns `ByteLimit`; existing endpoints unchanged |
| Slot staging/retirement | One incoming command and one retired producer | Staging returns candidate on full; boundary defers while retirement full |
| Native helper | At most one per worker, requested 2 MiB stack | OS spawn failure returns `io::Error`, consumes/cancels that candidate off thread |

`Config::validate()` and `Config::layout()` are pure preflight operations.
`AudioTap::layout()` and `SnapshotReader::layout()` expose the prepared layout;
`usage()` exposes process-wide live tap/slot counts and charged bytes. Usage is
an observation of separate atomics, not a transaction during concurrent prep/drop.
Capacity changes require preparing and staging a new tap. There is no callback
reconfiguration, queue resize or allocation fallback.

Layout is computed with target-native `size_of`, not a guessed PCM-only budget.
It includes the tap/worker/reader/shared objects, `Q` inline packets, `2S+1`
snapshot queue pointers, all `S+2` snapshots and their buffers, worker history,
window, complex work/scratch, current spectrum and spectral history. Each snapshot
includes `2N` raw stereo frames, 128 envelope bins, 128 stereo vector points,
`N/2+1` three-field stereo spectrum bins and eight stereo power slices.

`payload_bytes` reports those explicit objects/buffers. `reserved_bytes` adds
a version-audited scalar Radix4 twiddle-construction ceiling of `2N` complex f64
values, 64 KiB for dependency/control objects and overhead, and the requested
2 MiB helper stack even when using deterministic pumping. A slot is separately
charged 128 KiB. This is a conservative **requested-storage accounting contract**,
not a measurement of OS resident pages, allocator arena retention, thread runtime
bookkeeping or an exact platform RSS ceiling. Dependency upgrades require another
storage audit. The default Windows x64 layout is recorded below. Maximum supported
`N=4096,Q=64,S=8` gives `payload_bytes=5225256`, `reserved_bytes=7519016`,
10 snapshot objects and 16,384 input frames, through the same public preflight.

Global quota reservation uses control-side atomics with rollback. Credit remains
until the last endpoint/shared owner is destroyed, including a native handle
after worker exit. Retiring/cancelling a tap does not prematurely release credit.
Default and maximum preparation requested-allocation high-water checks run in the
tests, alongside aggregate byte/instance/slot rejection. A deallocation-time
observer verifies that worker-last shutdown holds credit through large-buffer
reclamation; worker fields release the shared lease after their buffers/rings.
External callers that
copy borrowed data into other storage own a separate budget.

There are explicit fallible `Vec::try_reserve_exact` preparations, mapped to
`Allocation`. Config/quota/reservation failures leave a previously prepared path
intact. RustFFT constructors, rtrb ring creation, Box and Arc allocation use
infallible standard allocation; a system allocator OOM can **abort the process**.
This packet does not claim recoverable OS OOM or inject allocator failure. All
preparation, including cleanup of a failed candidate, is off audio/locks.

One callback publication initializes/copies one fixed packet and checks at most
512 PCM scalars. Reset is constant work; installation moves at most one producer
and one retirement owner. There is no retry loop on audio. A pump visits at most
64 packets/16,384 stereo frames. At `N=32,H=8` it can analyze at most 2048 stereo
windows in a pump; at `N=4096,H=1024`, at most 16. Each analysis performs at most
two `O(N log N)` scalar FFTs, bounded statistics, eight spectral-slice copies,
and at most `2N` history/envelope work. Bounds are finite, rather than a claim that
the largest permitted config meets a particular device or UI scheduling deadline.
The ready queue stops snapshot materialization when full; FFT/history still run.

## Ownership, ordering, retirement and shutdown

PCM crosses a fixed-capacity **rtrb SPSC** queue as an inline `Copy` packet. It
contains no Vec, Box, Arc or other heap payload. Callback overflow and reset
therefore cannot free a packet payload. Snapshot boxes move between worker/reader
over a ready SPSC ring of capacity `S` and a recycle ring of capacity `S+1`.
The recycle ring initially holds `S+1` objects, with one extra worker spare.
No regular float memory is concurrently accessed by both owners. There is no
float seqlock, unchecked shared-memory access or new production `unsafe` code.

The stream epoch is a checked `u64`, written by the sole tap owner. Reset stores
the next epoch with release ordering; worker/reader load with acquire. Exhaustion
cancels forever and returns `EpochExhausted`, without wrap. The false-only alive
flag also uses release/acquire. Counters have separate single writers, except the
off-thread defensive pool-fault counter. Their saturating loads/stores avoid
callback CAS retry loops. Counters can saturate at `u64::MAX`; concurrent snapshots
of multiple counters are observational rather than atomically consistent.

Publication continuity includes next frame, rate, clock epoch, PDC and optional
device latency. A mismatch resets before the next accepted nonempty packet. A
non-cancelled refusal resets immediately, including full input queue, bad identity,
invalid clocks and oversized blocks. This deliberately invalidates **all** older
queued/published evidence. Worker discards old-epoch queued packets, clears history,
hop progress and spectrogram logical extents, and waits for `N` fresh contiguous
frames. A pump with no new PCM still observes and clears a reset. No analysis or
time window silently spans a gap, rate change, latency-label change or replacement.
GR metadata changes do not break otherwise continuous PCM windows.

Control completes candidate preparation (and native spawn, when used) before
staging, outside document/recording locks. A full
staging queue returns `Err(AudioTap)` with the exact candidate ownership intact.
The caller can retry or destroy it off audio. Failed clear returns false and
preserves the queued/active command. Audio boundary checks retirement capacity
before removing either active/candidate owner. Full retirement increments
`deferred_installs()` and leaves both in place. A successful swap cancels the old
tap before moving it into retirement, preventing accepted old results or publishing
as the successor source. No Arc clone/drop, buffer destruction or thread join
runs in boundary. A defensive failed push preserves owners in active/pending
slots; the cancelled old owner remains unusable until a later successful swap.

Control collects at most one retired producer per call. Drop of any producer,
worker or reader cancels that pipeline. `SnapshotReader::cancel()` explicitly
cancels future publications/polls. Dropping a retired producer frees/relinquishes
its input-ring ownership off audio; remaining worker/reader owners still retain
their own buffers and quota until destroyed. Destruction of an `AudioTapSlot`
or installer, including queued/active producers, must also occur off audio after
the engine has stopped using that slot. Do not replace an AudioTap by ordinary
assignment inside a callback or destroy a slot inside a callback.

The helper polls and sleeps for 1 ms when no packets were consumed, exclusively
off audio. Cancellation is checked at each packet; one current packet can still
finish bounded analysis before the helper exits. `WorkerThread::shutdown(self)`
cancels and joins; it reports a panic as an error. RAII Drop also cancels and
joins, so abandoned handles do not detach threads. Worker buffers returned by join
are destroyed on the control owner. Spawn, join and all endpoint drop operations
must run outside document/recording locks. Native tests join their helpers;
neither playback devices nor background helpers are left running by these tests.

Reader acceptance checks exact source/project/request ticket, stream epoch and
alive flag, including a second check after draining. Old snapshots are recycled
and counted; `poll` returns `None` when no accepted result remains. It can return
the same held snapshot when no newer result exists. While UI is stalled, the queue
retains its oldest ready objects and **drops new snapshots** rather than overwriting
them. After polling frees space, a future analysis can publish again. Read frame
age and current counters when displaying results; this is not a latest-overwrite
mailbox. Pool-conservation violations increment `pool_faults`, cancel the pipeline
and reclaim off thread, rather than leak objects or invent valid output.

A returned borrow cannot be retroactively revoked if cancellation occurs after
poll's final check. IPC/UI must recheck the current exact ticket when delivering
or drawing a borrowed/copied result. Numeric clock/epoch labels never grant a
stale tap authority over a successor. Generation issuance and that final delivery
guard are explicit future integration responsibilities.

## Data and calibration

All statistics use actual supplied PCM. f32 samples are promoted to f64 before
squares/FFT, so finite f32 extremes remain representable for the supported window
sizes. There is no sample clamp or nonfinite substitution in this transport.

For each channel and unwindowed current `N` samples:

- Absolute **sample peak** is `max(abs(x[n]))`. This is not an oversampled or
  intersample true-peak meter.
- RMS is `sqrt(sum(x[n]^2)/N)`.
- dBFS is `20 log10(value)`, referenced to unit PCM sample amplitude. Silence is
  `Dbfs::Silence`, not a fabricated finite floor. A unit-peak sine has RMS
  `1/sqrt(2)` and RMS dBFS approximately `-3.0103` under this convention.
- Clip count counts finite samples with absolute value **>= 1**, per channel/window.
- If any channel sample is nonfinite, that channel has `levels=None`, explicit
  invalid count and `spectrum_valid=false`; its spectrum values are NaN. The
  other healthy channel remains analyzable. Such data must not be serialized or
  displayed as healthy zero evidence.

The FFT is an ordinary forward **standard-bin** complex transform of real PCM
with zero imaginary input, using RustFFT's scalar `Radix4<f64>`. RustFFT's
allocation-performing `process()` is not used; a prepared scratch vector of the
reported `get_inplace_scratch_len()` is passed to `process_with_scratch()`.
Planning, trigonometry, FFTs, square roots/logs/phase and envelope building happen
off audio. Direct Radix4 bounds the audited plan shape and bypasses the SIMD
planner. This packet does not modify/reuse stretch's half-bin transform: that
transform intentionally omits ordinary DC/Nyquist and is unsuitable here.

For `n=0..N-1`, rectangular `w[n]=1` or periodic Hann
`w[n]=(1-cos(2*pi*n/N))/2`. Define:

```text
W1 = sum(w[n])           W2 = sum(w[n]^2)
X[k] = sum(x[n] * w[n] * exp(-i*2*pi*k*n/N))
d[k] = 1 for k=0 or k=N/2, otherwise 2
coherent_amplitude[k] = d[k] * abs(X[k]) / W1
mean_square[k] = d[k] * abs(X[k])^2 / (N * W2)
frequency[k] = k * sample_rate / N
```

There are exactly `N/2+1` ascending stereo bins. DC is bin 0; Nyquist is bin `N/2`
at `sample_rate/2`. Neither endpoint is doubled. There is no automatic DC removal.
`WindowCalibration` reports coherent gain `W1/N`, mean-square gain `W2/N` and ENBW
in bins `N*W2/W1^2`. Rectangular values are `1,1,1`; periodic Hann values are
`1/2,3/8,3/2`. Bin-centered isolated interior sinusoids have calibrated peak
amplitude in their central bin; fractional bins spread energy. There is no peak
frequency interpolation or promise of a fractional-bin amplitude correction.

Parseval gives `sum_k mean_square[k] = sum_n x[n]^2*w[n]^2 / W2` for the retained
one-sided spectrum. Rectangular integrated power is exactly unwindowed mean-square
apart from numerical error. Hann integrated power is **window-weighted** normalized
energy; it is not asserted equal to arbitrary unwindowed PCM RMS. Coherent
amplitude normalization and energy normalization are separate fields with separate
uses. Phase is the forward transform argument in radians; exact zero magnitude
has undefined phase represented by NaN. Tiny numerical bins have no meaningful
phase precision promise. No filter parameters enter this calculation.

Stereo correlation is the uncentered zero-lag normalized cross-product
`sum(L*R)/sqrt(sum(L^2)*sum(R^2))`, clamped for rounding to [-1,1]. It is not
DC-centered Pearson correlation. It is absent if either channel has zero energy
or invalid PCM. Mid/side are `(L+R)/2` and `(L-R)/2` with unwindowed RMS; both
summaries are absent if either channel has invalid PCM. The vectorscope uniformly
samples at most 128 points across the current window, including endpoints, in
mid/side coordinates. Invalid selected points are omitted and window invalid
counts remain visible; this is a summary, not a lossless phase trace.

Raw time history includes invalid samples and exact `history_first_frame`.
Envelope partitions its entire retained extent into at most 128 contiguous bins;
each has exact [first,end) frame labels, per-channel finite min/max and invalid
counts. All-invalid channel bins have absent min/max, not zero. Spectrogram has
the chronological last eight window power spectra, independent of snapshot
publication drops, with each slice's clock range and per-channel valid flags.
Use `spectrogram_slices()`/`spectrogram_count`: the rest of its preallocated
eight-entry storage is outside the current logical extent and can contain old data.

Optional GR is actual engine effect metadata supplied by a future effect hook,
not compressor gain inferred from amplitude. Valid range is finite **0–120 dB**;
invalid metadata is cleared and counted. A snapshot reports maximum reduction
only if every frame has valid metadata for the **same EffectId**. Missing/mixed
effects make it absent. Metadata applies to every frame in its stamped publication;
this is not a sample-accurate GR trace. Snapshot invalid-metadata counts are affected
frames in its current window; cumulative transport count is affected publications.

## Clocks, loss accounting and errors

`ClockRange` retains exact integer [first_frame,end_frame), rate, typed clock
epoch and explicit latency stamps. PDC-aligned first frame is signed i128
`first_frame - pdc_frames`. Estimated heard first frame subtracts the supplied
device latency as well, or is absent when it is unknown. These are derived labels,
not a measured speaker-output clock or a guarantee of device presentation time.
`pdc_time_label()` creates an off-thread `HH:MM:SS.mmm` string using integer
arithmetic, truncating sub-millisecond magnitude and preserving frame precision
beyond 2^53. Hours do not wrap at 24. Negative sub-millisecond positions can label
`-00:00:00.000`. Musical bars/beats require explicit T1 tempo/meter/anchor data;
this packet does not infer them from elapsed frames. Public manually constructed
ClockRange with zero rate returns `Invalid sample rate`; producer rejects it.

| Observation/error | Meaning/action |
| --- | --- |
| `published_packets/frames` | Accepted nonempty input publications |
| `consumed_packets/frames` | Packets accepted as current epoch at dequeue and processed; reset can race that bounded packet or discard incomplete windows |
| `dropped_packets` | Input queue-full publications only |
| `dropped_frames` | All frames in refused publications, including cancellation; not frames skipped through decimation |
| `refused_publications` | All publication refusals, with explicit returned reason |
| `invalid_samples` | Nonfinite channel scalars scanned in valid-size/stamped attempted publications, including input-full attempts |
| `invalid_metadata` | Invalid GR publications scanned before input-full decision |
| `resets` | Successful explicit/continuity/refusal epoch advances |
| `stale_packets/frames` | Queued old-epoch packets discarded by a live worker |
| `analyzed_windows` | Completed analyses, including analyses whose snapshot is dropped |
| `dropped_snapshots` | New result omitted because ready queue/pool is unavailable |
| `stale_snapshots` | Reader-refused obsolete/wrong-ticket/cancelled result objects |
| `pool_faults` | Defensive internal ownership fault; pipeline cancelled |
| `deferred_installs()` | Callback boundary attempts deferred by retirement backpressure |
| `Cancelled` | Pipeline retired/cancelled/dropped; no new accepted evidence |
| `WrongSelection`, `TooManyFrames`, `InvalidClock`, `FrameOverflow` | Whole call refused and old evidence invalidated |
| `QueueFull` | Whole call refused, counted and old evidence invalidated; no overwrite |
| `EpochExhausted` | Permanently cancelled rather than wrap/reset to stale generation |

Calls refused before the finite PCM scan do not fabricate an invalid-sample count
for unexamined input. Counters are saturating and individually observable through
`SnapshotReader::counters()`. Embedded snapshot counters are a publication-time
prefix, not current UI-drop counts; final packet-consumed accounting can follow
that snapshot. With no cancellation/destruction or counter saturation, accepted
input frames are consumed, counted stale, or still queued. On final cancellation,
remaining queues are reclaimed off thread without a claim that they were analyzed.
Reset counts/generation discontinuities expose abandoned partial windows; there is
no fabricated complete analysis of those frames. Lifecycle/prep errors never
replace an existing prepared path automatically.

## Verification and limits of evidence

Executed on **2026-10-08 UTC**, Windows 11 Home x64 (10.0.26300), Intel Core
i9-14900F (24 cores/32 logical processors), rustc 1.99.0
`b940084d7 2026-09-28`, Visual Studio 2022 Build Tools environment. Workspace
MSRV is 1.90; that compiler was not run. The narrow deprecated atomic API allowance
retains its older stable API instead of adopting a newer compiler's replacement.

Git Bash verification uses `source scripts/msvc-env.sh`, `CARGO_BUILD_JOBS=1`,
`RUST_TEST_THREADS=1`, `CARGO_TARGET_DIR=target/t8-native`, and
`TS_RS_EXPORT_DIR=target/t8-bindings`. One Cargo invocation runs at a time. No
full-workspace/Tauri/UI build, installs, dependency updates, generated binding
commits, device/listening test, GPU/browser test or other-OS acceptance is claimed.

```bash
source scripts/msvc-env.sh
export CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1
export CARGO_TARGET_DIR=target/t8-native TS_RS_EXPORT_DIR=target/t8-bindings
cargo test -p windfall-engine --test analyzer_taps --locked --offline
cargo clippy -p windfall-engine --all-targets --locked --offline -- -D warnings
cargo test -p windfall-engine --test analyzer_taps --release --locked --offline \
  release_cpu_throughput -- --ignored --exact --nocapture
rustfmt --edition 2024 --check crates/windfall-engine/src/analyzers/mod.rs \
  crates/windfall-engine/src/analyzers/transport.rs \
  crates/windfall-engine/src/analyzers/worker.rs \
  crates/windfall-engine/tests/analyzer_taps.rs
git diff --check
```

Scoped suite: **16 passed**, with the CPU test deliberately ignored in the normal
run. Strict engine all-target Clippy passed. The ignored CPU case was explicitly
run in release and passed. Owned rustfmt and diff whitespace checks passed before
checkpoint; only the granted/shared and new owned paths are included.

| Production-interface test | Independent evidence |
| --- | --- |
| `sine_sample_peak_rms_dbfs_and_bin_calibration` | Analytic sine peak/RMS/dBFS and bin calibration |
| `fractional_bin_window_leakage_and_scalar_reference` | Independently authored scalar O(N²) DFT; Hann expressed as sin²; fractional-bin leakage and phase |
| `impulse_dc_nyquist_silence_noise_and_log_sweep_energy` | Impulse, DC/Nyquist endpoints, authored seeded stereo noise and log chirp; all-bin DFT and integrated window-weighted energy |
| `anti_phase_distinct_channels_vectors_and_invalid_pcm_are_explicit` | Same/anti phase, distinct channels, invalid/clip counts and vector summaries |
| `variable_partitions_preserve_hop_clocks_and_bounded_histories` | 1/7/64/127/256-frame partitions, exact hop positions, bounded history and slices |
| `queue_overrun_and_stalled_ui_are_bounded_and_counted` | Worker stall, whole-block drop, stale-frame accounting and UI snapshot drops/recovery |
| `gaps_reset_rates_latency_and_selection_replacement_clear_stale_evidence` | Immediate stale refusal; no windows through reset/gap/rate/PDC/selection changes |
| `clocks_keep_integer_frame_precision_and_explicit_latency_labels` | Frames beyond 2^54, signed PDC labels, known/unknown device latency and checked overflow |
| `actual_effect_metadata_is_tagged_and_never_inferred` | Same-effect metadata max and explicit invalid metadata absence/counts |
| `preparation_bounds_preserve_old_path_and_aggregate_credit_until_all_owners_leave` | Invalid config/byte limit, old path remains usable, default/max requested allocation high water, reader-held credit and worker-last credit during buffer reclamation |
| `instance_and_installation_bounds_refuse_without_unbounded_backlog` | Tap/slot quotas and finite staging bounds |
| `callback_paths_allocate_reallocate_and_free_zero_including_replacement_backpressure` | Thread-local global allocator guard: zero alloc/alloc_zeroed/realloc/dealloc across normal/full/reset/refusal/cancel/slot paths |
| `native_worker_starts_publishes_and_joins_on_shutdown_or_drop` | Actual native helper publication and explicit/RAII joins |
| `staged_sources_and_deselection_retire_without_stale_successor_results_or_callback_frees` | Exact staged candidate return, source/project replacement, retirement backpressure/deselection and guarded zero callback heap operations |
| `native_worker_with_reader_stalled_keeps_a_finite_snapshot_pool` | Real helper with stalled reader, finite pool and counted drops, joined shutdown |
| `finite_float_extremes_and_invalid_latency_or_metadata_never_fabricate_healthy_evidence` | Finite f32 extremes, nonfinite PCM, invalid latency refusal and guarded copying |

The allocator guard scopes only callback operations; thread/worker/control
allocations are outside it. Production audit additionally excludes callback locks,
waits, IO, FFTs, transcendental math, heap-owner drop and CAS loops. The suite does
not force u64 epoch/counter exhaustion, OS OOM, OS spawn failure, pool-invariant
corruption or physical-device load. These have explicit code/documented behavior
but are not claimed executed fault-injection coverage.

Final release measurement: 4,800,000 stereo frames representing 100 seconds at
48 kHz, default `N=1024,H=256,Q=32,S=2`, deterministic pump and reader after each
256-frame publication. Elapsed **584.586 ms**, ratio **171.06 audio seconds per
elapsed second**, maximum observed publication wrapper **47.700 µs**. The output
field calls the ratio `audio_seconds_per_elapsed_second`; the timer is wall-clock
elapsed, not OS process CPU accounting. It includes worker analysis and polling;
the publication timing has `Instant` measurements outside the callback interface.
One authored repeating PCM fixture and this scoped hardware run do not establish
worst-case scheduling or a device deadline. It produced 18,750 accepted/consumed
packets, 18,747 analysis windows and zero input/snapshot/stale/pool drops.
Default `payload_bytes=718712`, `reserved_bytes=2914168` (including stack charge).
Earlier executions of the same case measured 589.108, 584.866 and 601.186 ms, with
observed publication maxima of 37.400, 25.000 and 3.200 µs respectively. These variations reinforce
that the measurements do not provide a worst-case scheduling guarantee.

### Default parallel test contention follow-up (2026-10-08 UTC)

The foundation `421bf5ebf2406d660751eb8cbd670527e09b13d7` and naming/proposal
follow-up `51c88f45212f9592b62349a9bc11ecf9d7343bc9` remain immutable. Their
original 16-case passing runs used `RUST_TEST_THREADS=1`. Parent Windows CI for
pushed `290f`, run `37756198700`, job `113241248719`, ran `cargo test --workspace`
with default test parallelism: **14 passed, 2 failed, 1 ignored** in this target.
The original test-file lines 635 and 709 respectively observed 2 versus expected
1 live tap and unwrapped an `InstanceLimit` refusal. The local captured CI log is
`C:/Temp/windfall-290-windows-ci.log`; this is executed CI evidence, separate from
the earlier reviewers' source/API inspections.

Before repository edits, the unchanged owner target with `RUST_TEST_THREADS`
unset reproduced **9 passed/7 failed/1 ignored** and **11 passed/5 failed/1
ignored**. Filtering only the two quota tests produced two failures; each passed
when invoked alone. The tests assumed exclusive use of production process-wide
accounting, while other fixture threads could admit or retire unrelated owners.
Neither a captured usage baseline nor a captured aggregate byte count is stable
across those independent lifetimes. The documented contract remains eight taps,
eight slots, and 32 MiB aggregate charged reservation, with credit retained until
the last shared owner is destroyed. `usage()` remains separate atomic observations.

The repair is confined to this integration-test executable: a private
`Mutex<()>` and `analyzer_test_scope()` helper. Every one of its **17 test entries**
declares `let _analyzer_scope = analyzer_test_scope();` as its first local,
including the 16 cases in the table above and ignored `release_cpu_throughput`.
The guard is retained through the entire case. Reverse local destruction releases
it after every subsequently declared endpoint/native handle; explicit shutdowns
and drops also occur while it is held. Acquisition precedes `usage()` baselines,
preparation, allocator-guard intervals, and every `Instant` timing interval.
Production callback code has no new lock. This only isolates fixture lifetimes
within `analyzer_taps`; workspace CI and other test executables retain their
normal parallelism. All original quota, signal, allocator, and timing assertions
are unchanged. A mechanical audit counted exactly 17 first-local guards and
restored the complete `51c88` test file by removing only the new import, helper,
and guard statements, byte-for-byte after newline normalization.

Poison recovery acquires the poisoned guard and first asserts that usage is
exactly zero taps, zero slots, and zero charged bytes. Only then is poison cleared
and a subsequent case allowed to execute. Libtest keeps the original case's
failure. Nonzero retained quota produces an explicit cleanup failure without
resetting counters, releasing someone else's credit, or clearing poison. A
private diagnostic compiled the exact helper from this test file and verified
native-handle/endpoint unwind cleanup and subsequent recovery. With one externally
retained tap, recovery refused at `Usage { taps: 1, slots: 0, reserved_bytes:
2914168 }`, preserving that usage and poison until real retirement. A separate
private libtest run intentionally failed its first case and passed its subsequent
case after verified cleanup: **1 passed/1 failed, exit 101**, proving the original
failure remains visible. These injected failures are diagnostic evidence, not
additional committed cases or passing claims for the production suite.

The unchanged private production-interface quota probe was rerun after the
repair. Gated unrelated admission again yielded 2 taps versus a test-local
expectation of 1; one unrelated retained owner plus seven local taps correctly
refused the local eighth. In each of 16 paired concurrent rounds, nine small
contenders admitted exactly eight plus one `InstanceLimit`; eight maximum-layout
contenders admitted four plus four `ByteLimit`. Stable charged usage was exactly
the admitted layout sum, at most 32 MiB, and returned to zero after every joined
retirement. Reader-held tap credit and installer-held slot credit remained
charged until their last owners left. Diagnostic sources/executables/logs are
private ignored artifacts under `target/t8-native/diagnostics`, not checkpoint
source or installed product hooks. All diagnostic/native helper threads joined.

Post-repair verification used the same Git Bash MSVC environment and private
target/bindings directories:

```bash
source scripts/msvc-env.sh
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target/t8-native TS_RS_EXPORT_DIR=target/t8-bindings
unset RUST_TEST_THREADS
cargo test -p windfall-engine --test analyzer_taps --locked --offline
export RUST_TEST_THREADS=1
cargo test -p windfall-engine --test analyzer_taps --locked --offline
cargo clippy -p windfall-engine --all-targets --locked --offline -- -D warnings
rustfmt --edition 2024 --check crates/windfall-engine/tests/analyzer_taps.rs
git diff --check
```

Both full-target test commands passed **16 cases, 0 failed, 1 existing CPU ignore**;
strict engine all-target Clippy, owned rustfmt, and diff checks passed. The optional
release timing case was not rerun: its first-local guard lies outside every
measured interval, and the measurement reported above retains its original
provenance. This repair adds no production changes or T8/EQ acceptance closure.

## Concrete next integration packet

The next change requires narrow grants from existing engine/desktop owners; no
closed path is edited here. Proposed hooks preserve existing PDC, callback
retirement, recording lock order and source/plan generation guards:

1. **Engine hook with T1/N4 coordination:** add one optional runtime
   `AudioTapSlot` and its control installer, boundary installation before a block,
   and bounded producer copying at the selected location. Stamp the actual
   Processor frame cursor/sample rate/clock epoch, project/request ticket and
   source-specific plan latency. Track-post-fader capture uses the actual mixed
   effect/fader/pan buffer while it is still valid, before routing/scratch reuse;
   the track's own PDC path must be used, not blindly the master latency.
   Mix-output capture includes apart voices and output gain. Current output
   finishing also scrubs nonfinite device output: introduce a narrowly reviewed
   observation point after gain and before scrub, preserving identical device
   output behavior, so diagnostics can report invalid PCM. Do not call an
   already-scrubbed zero buffer proof of healthy processing. Partition blocks
   above 256 explicitly; expose any failed publication and issue discontinuity
   epochs through seeks/loops/rate/rebuilds. Offline render/stem tap structures
   do not confer live source ownership. Do not consume existing rack/controller
   meter snapshots from the callback or steal their observations.
2. **Control/effect lifecycle:** prepare candidate queues/workers without
   document/recording locks, then recheck typed source/project/request identity
   at the narrow installation point. Preserve the existing recording-before-State
   ordering if either must be held for validation. Invalid/prep/staging failure
   leaves the old producer usable; a real replacement clears old display evidence
   and remains pending until an exact-ticket fresh snapshot arrives. Collect
   retired producers and join cancelled workers off locks/audio. Coordinate with
   E1/E2 for a non-destructive actual effect GR observation; amplitude cannot
   supply compressor reduction. Musical big-clock anchors/maps belong to T1.
3. **Native desktop IPC/subscriptions with M1:** add session-owned select,
   subscribe/unsubscribe, cancellation/status and bounded result delivery over
   the existing realtime infrastructure. Carry exact tickets/epochs/frame ranges,
   clock/rate/PDC/device-latency status, current drop/invalid counters, validity
   and silence explicitly. JSON cannot safely convey NaN as healthy zeros: encode
   absent/invalid channel/bin/phase states intentionally. Bound outgoing event
   payloads/backlog and count downstream losses as well. Enforce ticket checks at
   final delivery/project replacement, and report native unavailable honestly.
   Parent owns registration/generated bindings; no mock subscription earns native
   availability or source-selection acceptance.
4. **Purpose-built views:** implement accessible keyboard/labelled source
   selection, distinct big clock/dB meter/spectrum and Wave Candy-style time,
   spectrogram/phase/vector views; add the EQ's actual PCM spectrum separately
   from its existing parameter-derived response and a mixer waveform view. Reuse
   renderer/theme/realtime subscriptions and bounded drawing data. Expose pending,
   invalid/gap/drop/unknown-latency state. Drawing/GPU/screenshots stay outside
   the callback. Add interaction/cleanup tests for each distinct view.
5. **Fresh integrated/native acceptance:** exercise distinguishable mix and
   track stereo fixtures through actual device callback/native subscription,
   selected-source changes/deletion/project replacement, seeks/loops/rate/PDC
   changes, invalid PCM and stalled worker/UI. Verify old results cannot enter the
   new source's view. Run new engine/UI/native checks against fresh parent-owned
   integrated artifacts; record physical-device and other-OS evidence separately.

The complete T8 rows `vis-fruity-big-clock`, `vis-fruity-db-meter`,
`vis-fruity-spectroman`, `vis-wave-candy`, and `win-mixer-waveform-view` remain
**open**. EQ2's live-spectrum audit and the limiter's additional scrolling
gain-reduction/product behaviors also remain open. This foundation and its
synthetic test feeds do not complete those product acceptance gates.

## Primary references and provenance

Accessed **2026-10-08 UTC**. Equations/fixtures/implementation above are independently
authored; no third-party implementation or research code was copied. Documentation
claims below are restricted to the primary sources actually read.

- [RustFFT 6.4.1 crate documentation](https://docs.rs/rustfft/6.4.1/rustfft/):
  transform bin ordering, lack of implicit normalization and direct algorithm
  construction versus SIMD planning.
- [RustFFT 6.4.1 Fft trait](https://docs.rs/rustfft/6.4.1/rustfft/trait.Fft.html):
  allocation-performing process convenience method, scratch-supplied method and
  scratch-size query. The installed 6.4.1 Radix4 constructor was additionally
  inspected for finite twiddle/base/scratch storage.
- [rtrb 0.4.0 documentation](https://docs.rs/rtrb/0.4.0/rtrb/): fixed-capacity
  SPSC ownership and bounded push/pop; full queue returns the item, without overwrite.
- [NIST, Properties of FFT Spectra](https://tf.nist.gov/phase/Properties/ten.htm):
  leakage/window tradeoffs and Hann's 1.5-bin equivalent noise bandwidth.
- [Donnelly and Rust, NIST, FFT Part III: Classical Spectral Estimates (2005)](https://tsapps.nist.gov/publication/get_pdf.cfm?pub_id=150010):
  primary spectral-estimation/windowing context. Coherent/energy scaling above is
  derived explicitly here and checked against an independent scalar DFT/Parseval,
  not assumed from a library plotting convention.

Existing stretch FFT source and `crates/windfall-stretch/LICENSE-THIRD-PARTY` were
read for the half-bin distinction and license context. RustFFT and rtrb are both
MIT OR Apache-2.0; this packet uses their existing resolved crates and cites APIs,
not copied source. Failed 403 research downloads (including the Fermilab-hosted
Heinzel FFT PDF) are not claimed as consulted evidence.
