# Native macOS VST3 bundle implementation candidate: R1 response

Prepared 2026-10-08 by T3 in the existing bound worktree
`gpt/t3-vst3-portability-ci`, from immutable input
`5d3fe8e7c7f35cb1c0fd4538cfbc381796296387`. The first proposal is immutable
`f78a82e5ac9e94af24beb68fc3a173d3ea96e7c8`, directly atop `5d`. This revision
responds to the independent R1 design audit. The parent subsequently superseded
the doc-only window with a complete Mac loader/path/owner/gate/fixture/test
implementation grant in this same branch, including mechanical fixture declarations
and test compatibility. This is a source candidate directly atop `f78`; neither
input is amended or imported. Only supported local checks run here. No Apple SDK,
native Mac build/execution, install, dependency upgrade, CI dispatch or root write
occurred. The parent arranges the final Standards/Spec pair and exact-source native
Mac CI. The full goal remains active with 342 rows; this is not Mac acceptance.

## Exact inspected state and evidence

Read git objects at root
`7027569f8b466242a75faf327381e6ac44c88d28`, direct parent
`dfe522b58ae19362a91472031057bcb80ff8644e`. The root commit records composed
CI repairs and activation gates. Nothing was checked out or imported. The
loader, paths, common fixture helper, realtime tests, SDK fixture, and four
identity regressions are byte-identical between this root and frozen `5d`.
Root processor/bridge changes are separate accepted work and remain closed.

The original native Mac log is `C:/Temp/windfall-43-macos-ci.log`, SHA-256
`b7f9ce9d08332e8f98bfe4f01474def73685164ee53a3b9d3954e365f9517585`.
Lines 3118–3195 report 12 passes and eight failures, zero ignored/filtered.
All eight failures contain `Unsupported("VST3 bundle entry on macOS")`:

- `r4_disjoint_vst3_pending_editor_and_timed_controls_process_truthfully`
- `r4_failed_native_process_never_publishes_control_readback_and_retains_final_intent`
- `r4_full_editor_queue_and_other_native_point_sources_fit_without_allocating`
- `r4_inactive_and_timed_vst3_points_reach_native_at_their_frames`
- `runtime_repair_vst3_ordinary_and_reserved_panics_release_all_channels`
- `vst3_effect_and_instrument_callbacks_allocate_and_free_nothing`
- `vst3_ownership_boundaries_move_adapters_without_allocator_calls`
- `vst3_reset_and_saturated_release_keep_every_admitted_note_off`

`record_macos_702.py` checks the captured summary, eight distinct names and
eight exact refusal messages. This is an artifact consistency check, **not a
new native reproduction**. The original native execution supplies the failure
evidence; local Mac reproduction and an Apple SDK remain unavailable.

The parent subsequently supplied the completed root702 native Mac log,
`C:/Temp/windfall-702-macos-ci.log`, SHA-256
`4b615a314d978a2b860ac673511003ae3bfa906c540bf24d945c5aa6a1249a2c`.
Run `37766014784`, Mac job `113273753439`, failed again: lines 3234–3310 show
the same eight refusals, 12 passes/eight failures/zero ignored. This worker
verified the supplied log, not a new local Mac execution. Its image is
macos-26-arm64, macOS 26.6.2, version 20260907.0351.1, with Rust 1.99.0 on
aarch64-apple-darwin. Analysis now passes 30/30 at line 1342; DSP utilities
pass 152 with three existing ignored at line 2162; analyzers pass 16 with one
existing ignored at line 2342, all before the VST failure. Full Clippy passed.

The parent also reports actual Ubuntu Rust job `113273753491` **SUCCESS** on
this same source, independently validating the Linux identity repair in CI.
This worker did not fetch or execute that Ubuntu job. The original Ubuntu
fault PC was never captured; local mechanism proof and the successful actual
Ubuntu rerun are distinct evidence. The parent subsequently reported root702
Windows failed only post-reset capture ownership; N4 owns that separate
helper/control diagnosis. It is not attributed to Mac loading or to the
Linux identity repair. No result supplies repaired Mac bundle execution or
closes the full goal/parity counters.

The newer root `427ed0a7` contains the bounded UI repair and no Mac loader
repair. Its run `37770742963`, Mac job `113289416625`, repeats the same
12 passes/eight failures/zero ignored, all eight Unsupported refusals. This
worker read `C:/Temp/windfall-427-macos-ci.log`, verified SHA-256
`ac703aa899b72e4df429b93f8622e9587a3878efa872f21af57ddf8808246da7`, and
checked the eight error occurrences and summary at lines 3260–3309. This is
another actual baseline run, not execution of this proposal. The parent
reports UI job `113289416573` SUCCESS (172 files/2662 tests), Ubuntu Rust and
all platform binding jobs SUCCESS; Windows Rust is still live at this
briefing and no final result is inferred. N4's separate versioned-reset
diagnosis remains N4-owned. The Mac
CreatorThread fallback remains the original Rust ThreadId implementation and
an explicit possible next native failure. No Darwin identity grant is inferred.
The superseding grant released the fixture declaration/entry and bundle test
compatibility seams to T3. All 21 classes and frozen point/drop/owner assertions
stay intact. N4 helper/control and P1 runtime/preparation windows stay separate.

| Root 702 seam                              | Concrete gap                                                                                                                                                                             |
| ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/vst3/mod.rs:72–75`                    | `Vst3Module::load` returns Unsupported before any Mac entry/factory call.                                                                                                                |
| `src/vst3/mod.rs:25–28,365`                | Ownership currently assumes a libloading Library plus optional no-argument exit; there is no owned CFBundle or attempted-entry state.                                                    |
| `src/paths.rs:161–177`                     | A Mac directory is resolved as `Contents/MacOS/<bundle-stem>`, ignoring `CFBundleExecutable`; raw files are also accepted by the resolver.                                               |
| `tests/common/mod.rs:53,104–116`           | Builds a Rust cdylib and copies it directly to `fixture.vst3`; it creates no Mac bundle/Info.plist.                                                                                      |
| `test-plugins/src/vst3.rs:912–931`         | Exports factory/InitDll/ExitDll/ModuleEntry/ModuleExit, but no bundleEntry/bundleExit. Factory still exposes all 21 classes, indexes 0–20.                                               |
| `tests/vst3_native_thread.rs:115–197`      | Its real component owner-guard subprocess directly libloads the raw fixture. Publishing a Mac directory requires a Mac-only probe owner adaptation.                                      |
| `tests/vst3_host.rs`, `tests/vst3_scan.rs` | Existing whole-file Windows/Linux gates; catalog bundle construction also encodes Windows/Linux layout. Their Mac compatibility is now included in the superseding implementation grant. |

## Primary contracts and ABI provenance

Steinberg research is pinned to SDK public source
`586dc5e6c8012c3e4b01c79389375cbe96bdb1da`. Its
[Mac host](https://github.com/steinbergmedia/vst3_public_sdk/blob/586dc5e6c8012c3e4b01c79389375cbe96bdb1da/source/vst/hosting/module_mac.mm#L85-L184)
uses CFBundle loading and requires three exports: C-callable
`bool bundleEntry(CFBundleRef)`, `bool bundleExit()`, and GetPluginFactory.
It takes an owned factory reference and releases it before exit and bundle
reference release. It does not explicitly force CFBundleUnloadExecutable.

The [plugin entry implementation](https://github.com/steinbergmedia/vst3_public_sdk/blob/586dc5e6c8012c3e4b01c79389375cbe96bdb1da/source/main/macmain.cpp#L48-L101)
supports repeated paired entry/exit calls. It increments its counter, retains
the bundle, and records the reference before InitModule can return false.
**Inference for Windfall cleanup:** arm an attempted-entry guard before the
call, and balance refusal once with the already-resolved exit. A guard armed
only after success leaks the SDK's refused attempt. Do not call exit if no
entry was attempted; the SDK destructor's unconditional export lookup is not
a substitute for tracking that state.

Apple's [CFBundleExecutable definition](https://developer.apple.com/library/archive/documentation/General/Reference/InfoPlistKeyReference/Articles/CoreFoundationKeys.html#//apple_ref/doc/uid/TP40009249-SW1)
identifies the loadable binary independently of the bundle directory name.
Apple's [CF ownership policy](https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFMemoryMgmt/Concepts/Ownership.html)
assigns ownership to Create/Copy results and requires their release. Borrowed
Get results require different treatment.

The official [CFBundle header](https://github.com/apple-oss-distributions/CF/blob/dc54c6bb1c1e5e0b9486c1d26dd5bef110b20bf3/CFBundle.h)
declares opaque CFBundleRef, actual-executable URL lookup, load/error APIs,
and symbol lookup. It makes the caller responsible for CFError release and
warns CFBundleCreate may return an existing instance. Its directory-info Copy
API reads metadata without requiring a bundle instance; the independent
[directory-info implementation](https://github.com/apple-oss-distributions/CF/blob/dc54c6bb1c1e5e0b9486c1d26dd5bef110b20bf3/CFBundle_InfoPlist.c#L712-L715)
confirms that route. The pinned Apple CF repository is historical source,
not the installed SDK of the completed CI job.

Apple's [CFBase header](https://github.com/apple-oss-distributions/CF/blob/dc54c6bb1c1e5e0b9486c1d26dd5bef110b20bf3/CFBase.h)
and [CFURL header](https://github.com/apple-oss-distributions/CF/blob/dc54c6bb1c1e5e0b9486c1d26dd5bef110b20bf3/CFURL.h)
provide Boolean/index/path signatures. CF Boolean is an unsigned-byte value;
it must not be conflated with the SDK entry's C++ bool. Mac target headers
must verify all chosen Rust FFI declarations, opaque pointer mutability,
CFIndex/CFTypeID widths, and function signatures before implementation claims.
No Darwin pthread representation is inferred from Linux.

Apple documents [hazards when unloading code](https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFBundles/AccessingaBundlesContents/AccessingaBundlesContents.html#//apple_ref/doc/uid/10000123i-CH101-SW24).
Its historical [CF implementation](https://github.com/apple-oss-distributions/CF/blob/dc54c6bb1c1e5e0b9486c1d26dd5bef110b20bf3/CFBundle.c#L965-L1000)
also unloads during final bundle deallocation. Therefore CFRelease is neither
a proof of immediate unmapping nor a guarantee of code retention. Follow
SDK ownership; add no permanent host reference or forced-unload workaround.

## Implemented replacement minimum contract

This **replaces** the split PreparedBundle/factory/retire interface and waiting
gate at `f78:143–230`. R1 found that contract insufficient. The new minimum
adds bounded admission, owner affinity while tickets are live, deferred
retirement, and persistent source history. These are material restrictions,
not implementation details assumed to have existed in `f78`. The private module
is larger internally because callers no longer coordinate its cleanup. The exact
production lifecycle code has 13 deterministic pure regressions, executed locally;
the CF adapter and seven native bundle tests still require actual Mac CI.

Keep public PluginHost/PluginModule interfaces, VST3 ABI/UUID conversion,
descriptor/probe/create results, and Windows/Linux raw loading and Weak caching.
One Mac load owns one paired SDK ticket; Mac Rc clones share that ticket, including
the existing instance-held module. There is no Mac strong cache, explicit
CFBundleUnloadExecutable, or permanent owner to keep code mapped.

### Small owned interface

The caller interface in `src/vst3/bundle_macos.rs` is:

```text
resolve_source(path) -> io::Result<Source>
    bounded fresh metadata only; never creates or loads a CFBundle
MacModuleLease::load(path) -> Result<MacModuleLease, PluginError>
    admission, native preparation, entry, owned factory, complete failure cleanup
MacModuleLease::with_factory(operation) -> operation's result
    scoped borrow of the factory; native-call scope defers retirement until return
Drop(MacModuleLease)
    last lease retires, or transfers its preallocated node to the active owner guard
```

These responsibilities are implemented. There is no
factory extraction or public retire operation. The private MacModuleLease
encapsulates OwnedState (factory, CFBundle, exit and attempted-entry flag) and
one reserved Retirement node (cleanup action, stable record handle and owner
ticket). The node already holds the same state through a private Rc, so final
Drop transfers it without allocating. ModuleData's Mac field is one lease,
not a factory/owner tuple. Only OwnedState releases either native owner.
The two private `with_factory` callers return descriptors or component/controller
references; they never extract a factory reference. Temporary factory casts die
inside the scope. Mac module handles use Rc because those handles stay on their
creating owner; non-Mac Arc/Weak caching is unchanged. No blanket Send/Sync or
Clippy suppression was introduced for the Mac owner.
Created component/controller references remain paired with the existing
instance-held module, whose teardown remains after those native references.

The two existing factory access sites in `mod.rs` (descriptors and object
creation) need a scoped adapter to this interface. Their existing native call
bodies, UUIDs, errors and results remain. This is an explicit correction to
`f78`'s claim that all factory access bodies can remain untouched. No processor
or instance implementation change is proposed. The existing Objects unsafe
Send/Sync contract still requires lifecycle teardown on the creating owner
after audio ownership returns; this proposal neither repairs nor waives P1's
separate preparation/retirement gates. No new blanket CF Send/Sync is proposed.

Lifecycle refusals are Busy, Capacity, StaleSource, Poisoned, ExitRefused and
NativeRefused. Context conflicts are StaleSource; the CF adapter additionally
reports invalid bundle/residency/native load/export/entry/null-factory causes.
All load errors use the existing PluginError::Load with an
explicit reason, for example `macOS VST3 Busy: lifecycle occupied`. Busy and
Capacity are refusals, never successful cached leases, empty class lists,
automatic retries, or skipped tests. Drop cannot return a result; it records
exit refusal in reserved metadata while completing host ownership release.

### Bounded preparation without creating a bundle

Require a `.vst3` directory and the modern `Contents/Info.plist` plus
`Contents/MacOS` layout. Reject raw renamed dylibs and legacy flat bundles in
this minimum stage. Preserve non-Mac raw load support. The preliminary resolver
uses bounded filesystem reads, a regular-file check, and native path bytes;
it rejects NUL, lossy conversions, non-single-file CFBundleExecutable values,
and invalid CF value types. It never calls CFBundleCreate, executable lookup,
symbol lookup, or any load operation.

Instead of the former directory-info Copy path, read at most 256 KiB plus
one overflow byte from the fresh plist. Parse supplied bytes with an owned
CFData and CFPropertyListCreateWithData, immutable option, default allocator;
release the plist/data and nullable CFError on every path. This avoids asking
CF to read an unbounded, concurrently growing file or to create a bundle to
resolve it. This is a deliberate smaller supported metadata contract, not
a private Apple cache-flush operation. Apple's pinned
[property-list header](https://github.com/apple-oss-distributions/CF/blob/dc54c6bb1c1e5e0b9486c1d26dd5bef110b20bf3/CFPropertyList.h#L129-L133)
defines that input and ownership. CF parser allocations are external to the
host storage budget; bounded input is not proof of a fixed CF heap ceiling.

BundleSource holds canonical bundle/executable paths, directory/file device
and inode identities, executable size/modified/change timestamps, and exact
bounded plist bytes. Preserve the existing public path/size/date identity
schema. Private extra facts detect hard-link aliases, context changes and
observed generations; verify Darwin filesystem timestamp behavior natively.
The shared resolver gives `paths::vst3_binary` the declared executable rather
than guessing the bundle stem. Its existing Option signature still represents
resolution failure as None; the loader calls the Result interface directly
so it preserves Busy/Capacity/identity diagnostics instead of collapsing them.

No source stamp proves arbitrary byte equivalence, resource-tree immutability,
or an atomic approved-file snapshot during hostile concurrent writes. This
minimum requires stable published files during a load, rejects every observed
generation change, and does not claim hot replacement or substitute for P1's
versioned preparation. An undetected same-metadata mutation or externally
loaded dependency is a containment/preparation limitation, not proof that
the new source executed. Native tests must exercise observed retargeting.

### Admission and storage limits

These concrete limits are implemented host policy, not SDK maxima. Capacity
tests cover ticket, per-image, preflight, record and owned-byte limits. Increasing
them needs a separate policy review. Allocate cleanup storage before native
preparation. Host allocation abort is a containment limitation, not an unwind
cleanup guarantee; fallible byte/vector reservation refuses with Capacity.

| Resource                        | Proposed bound and admission rule                                                                                                                                                                                       |
| ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| In-flight load scratch          | 16 permits before parsing; at most two host source/read buffers of 256 KiB plus one overflow byte per load, plus bounded path buffers. Persisted clones are charged separately to history. CF allocations are external. |
| Canonical path                  | At most 16 KiB per path. Over-limit input returns Capacity before CFBundleCreate.                                                                                                                                       |
| Persistent known-source records | 256 fixed stable slots, at most 8 MiB total owned paths/plist/history bytes; no moving occupied records or native pointers.                                                                                             |
| Entry tickets / cleanup nodes   | 512 process-wide, at most 32 per executable. Reserve one node and all terminal diagnostic fields before CFBundleCreate, even for a load that later fails before entry.                                                  |
| Deferred retirement             | At most one node per outstanding ticket; linked through the already-reserved nodes. No queue allocation, growth or ticket duplication in Drop.                                                                          |
| Held reservations               | One root plus at most one current draining record, held with atomics. Other queued records remain protected by live owner affinity until their turn. No bitset or growing held-record collection.                       |
| Stored terminal diagnostics     | Fixed status bits, native-loaded marker and ticket counters. The native C++ probe writes its own external event ledger; production metadata has no growing event log.                                                   |
| Caller aliases                  | At most 16 extra absolute lexical paths per record, charged by allocated capacity to the same 8 MiB budget. Established alias retargeting remains refusing even if the new canonical target is unrelated.               |

Scratch permits, ticket cells and persistent byte charges are separate. A
live ticket's cleanup reservation cannot be borrowed by a subsequent load.
Any exhausted limit or allocation refusal returns Capacity before bundle
creation and leaves existing tickets cleanable. Lease clones do not allocate
entry tickets; these bounds concern loader bookkeeping, not arbitrary instance
counts or a foreign plugin's own allocations. Standalone path resolution has
the same per-call byte/path bounds, without allocating a lifecycle ticket.

Records are stable for process lifetime once CFBundle creation is about to
be attempted: commit the known source/context before that call, not after
entry succeeds. Record both canonical bundle and executable path bindings
and filesystem identity, so replacing an executable inode or retargeting a
known bundle cannot create an unrelated fresh gate. The preliminary checks
and registry lookup must compare both axes, including hard-link aliases.
Symlink aliases that canonicalize to the same bundle/context are permitted
within the alias budget. Preserve their absolute caller-path binding, not only
the canonical target; a later alias retarget cannot become an unrelated fresh
record. Distinct bundle resource contexts for one executable are refused.

After clean retirement, keep the original generation/context. Admit only an
unchanged one. Any observed replacement, missing known source, conflicting
context, mismatched CF executable, native preparation/exit refusal or poisoned lifecycle makes
the applicable known record permanently refusing until process restart.
Refusal does not replace the old stamp with the attempted new stamp. Even if
the old file is restored, the terminal refusal persists. No retired/refused
record is evicted to make room; a full registry returns Capacity. The implementation is stricter than the former provisional-cancellation
proposal: no committed record is evicted or canceled, even when later ticket
capacity or native creation fails. Admission may consume bounded history before
native work; it never silently restores fresh admission. Conflicts mark the
established record refusing.
The registry retains metadata, never a factory, CFBundle, image handle or
strong module owner. Process restart is a caller-visible refusal remedy,
not a retention workaround or an automatic retry by the loader.

### Cycle-free reservation and owner policy

A short registry mutex protects admission transactions only. Admission uses
try_lock: contention returns Busy, poisoning returns Poisoned and disables
new admissions. No filesystem/CF/plugin operation, native/user destructor, allocation,
formatting, wait or join runs with it held. History records and terminal
atomic fields have stable addresses; release/acquire publication occurs only
after their native ownership stages are complete. A logical reservation is
separate from that mutex. There are no condition variables or logical waits.

While any ticket is staging, live, queued or cleaning for an executable, all
its tickets have one lifecycle owner. Another thread's load returns Busy,
including while the gate is idle but owner affinity is still live. The same
owner may load another ticket outside a native-call scope. Owner affinity can
transfer only after all tickets and reservations are terminal; the persistent
generation/context history does not transfer or disappear with it. Independent
executables on different owners can load concurrently within the limits.

Any load reentered from **any** native-call or cleanup scope on its own thread
returns Busy immediately, even for a different executable. An unrelated worker
loading an occupied/foreign-owned executable also returns Busy. The loader
does not retry or wait for idle. A plugin that ignores refusal and waits
indefinitely remains a foreign hang requiring process containment; the host
does not promise to make arbitrary callback code progress.

The owner guard is installed before CFBundleCreate and covers creation,
validation, executable load/initializers, export lookup, entry, factory
acquisition, revalidation, failure cleanup and final CF release. Apple's
[initializer contract](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man3/dlopen.3.html)
puts initializers before a successful load returns. Pinned CFBundleCreate
[final processing](https://github.com/apple-oss-distributions/CF/blob/dc54c6bb1c1e5e0b9486c1d26dd5bef110b20bf3/CFBundle.c#L1167-L1179)
also permits CFPlugIn registration. Thus neither bundle creation nor missing-
export bundle release is allowed outside the reservation. No assumption that
these operations are metadata-only survives from `f78`.

Within the reservation, reread and compare fresh source facts before creating
the CFBundle, compare its Copy executable URL to the preliminary source, and
check again before entry and publication. There is no wait to resume with an
old resolution; each later Busy retry by a caller starts fresh. A first-ever
native owner already reporting CFBundleIsExecutableLoaded is refused as
ResidentUnknown, not adopted as proof of the current generation. For an
established unchanged generation, an already-loaded result does not establish
unmapping or hot replacement; it is consistent with other native owners.
[CF's already-loaded contract](https://github.com/apple-oss-distributions/CF/blob/dc54c6bb1c1e5e0b9486c1d26dd5bef110b20bf3/CFBundle.h#L256-L261)
is the reason retired history cannot be forgotten. CFBundleCreate itself may
perform work before these checks; the reservation contains it, but a subsequent
refusal cannot retroactively prevent foreign initialization already performed.

`with_factory` wraps all existing host factory uses in an owner native-call
scope, including temporary cast release. An outer scope owns the bounded
retirement queue and affected reservations. Nested scoped borrows of already
published leases on that same owner share this scope; they create no new entry
ticket or lifecycle call. This does not permit reentrant load or cleanup.
Borrow lifetimes keep their own lease alive. Descriptors and creation keep
their current public results; they do not return false empty results on gate
contention. Under the existing correct-owner contract, their executable cannot
be reserved by a different owner while that lease is live.

Last-lease Drop outside a native scope acquires that owner's idle reservation
and drains immediately. Drop reentered from native code instead moves the
entire OwnedState into its existing reserved node on the outer guard's queue,
without waiting, calling exit recursively, or releasing the bundle. The node
is transient native ownership needed to finish the interrupted operation;
it is not stored permanently in the metadata registry. The outer guard drains
on that same owner after the outermost native call/borrow returns and before
clearing its reservations or returning the top-level load/factory operation.
For another executable's queued node, affinity ensures its reservation is
idle or already held by that same outer guard. No foreign owner can seize it.

During draining, all new loads reentered on the cleaning owner return Busy.
Further drops add only existing, preallocated ticket nodes. Each node is
cleaned once, so at most 512 nodes can be drained in one outer operation;
at most two reservations are simultaneously occupied by that guard. Nested cleanup never recursively
starts another drain. A native call that never returns has no bounded cleanup
time; node/storage bounds are not execution-time bounds. When the queue is
empty, clear reservations only after every required native release returned.
There are no waiting callers to notify; future try-admission observes terminal
state. A failed load and all its pending cleanup finish before its error returns.

An actual off-owner last Drop violates the existing unsafe lifecycle contract;
it cannot be made safe by draining on a foreign reservation holder or silently
leaking a node. The invariant failure takes an explicit fail-stop
containment path, with reserved reason bits, rather than running native cleanup
on the wrong thread. This is not a new supported ownership path or a claim
that P1's owner-return mechanism is verified. Native wrong-owner tests and
parent review of this refusal remain required before source acceptance.

### Owned state and cleanup transitions

Host-only preflight and the admission transaction precede native ownership.
A committed Ticket cancels its reservation if Rust unwinds before scope
installation; no CFBundle exists then. The complete state and retirement node
are allocated before the native guard is installed, and that guard precedes
CFBundleCreate and every subsequent native acquisition. The unpublished lease
owns the complete state during preparation and is returned only after successful
factory acquisition, source checks and pending cleanup. There is no success
tuple whose factory can outlive a missing bundle owner.

| Stage          | Owned facts / armed cleanup                                                                                                  | Next normal action or failure retirement                                                                                            |
| -------------- | ---------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| Preflight      | Scratch permit; no bundle, ticket, entry or factory                                                                          | Resolve bounded metadata; Busy/Capacity/invalid input releases host-only storage.                                                   |
| Reserved       | Established source/context and aliases, owner, gate, ticket and preallocated cleanup node; no CFBundle yet                   | Record stays established before CFBundleCreate. Capture native-loaded history only after an actual successful executable load.      |
| BundleOwned    | Returned CFBundle is captured immediately, before any fallible Rust work                                                     | Validate copied URL/metadata/residency, then load executable; on failure release all CF owners inside the reservation.              |
| Loaded         | CF owner protects symbol lookup and any initializer side effects                                                             | Resolve all three mandatory exports; missing export retires without entry or exit.                                                  |
| EntryAttempted | Exit function is known; attempt flag is armed **before** entry                                                               | False result retires with one balancing exit, no factory call.                                                                      |
| FactoryOwned   | Non-null factory is captured immediately with the existing owned ComPtr ABI                                                  | Revalidate source and transfer complete state to a lease, or retire it.                                                             |
| Live           | Leases/instances retain that one entry ticket and its reserved node                                                          | Last Drop becomes Queued or Cleaning; ordinary factory borrows cannot split ownership.                                              |
| Queued         | Node owns factory/CFBundle and all armed stages; native frame still active                                                   | Outermost owner guard drains after native code returns.                                                                             |
| Cleaning       | Remove factory from state, release it; clear entry-attempt flag then call exit once; remove bundle from state then CFRelease | Finish each phase before advancing to the next native release; additional reentrant drops queue.                                    |
| Terminal       | No host factory/CF owner; fixed status/history persists                                                                      | Return scratch/node capacity; publish idle only after all cleanup/deallocation returned. Refused/Stale/Poisoned cannot admit again. |

Temporary CFURL/CFString/CFData/plist/CFError owners are local guards. During
native preparation they die within its reservation before final bundle release.
Out-pointers start null. Never perform formatting or allocation between a
returned owned pointer and capturing it in its guard. Every Create/Copy
reference is released exactly once; Get references remain borrowed. All
owned temporaries must have explicit retirement before the reservation can
finish, rather than depend on a later implicit field Drop. No observer handle
is held open to make unload tests pass.

### Rust unwind, poison and foreign containment

Mutex poisoning is not the reservation state. Rust documents that poisoning
is advisory and does not reliably track foreign exceptions or every panic
([Mutex contract](https://doc.rust-lang.org/std/sync/struct.Mutex.html#poisoning)).
The guard therefore owns explicit stages independently of the registry lock.
It is non-panicking for host-controlled paths: no unwrap/expect, checked-counter
panic, allocation, formatting, logging callback or user destructor during
cleanup. Take/clear each native cleanup obligation before invoking it, so a
host unwind cannot issue it twice. Capture fixed failure bits without building
a string. A guard unwinding from host Rust code disables new admissions and
marks its records Poisoned; it still drains the acquired state and queued nodes
in factory → attempted exit → final CF release order on the owner.

Admission commits preallocated history slots under the short mutex, with no
foreign work. A record can stay established even when the subsequent ticket
capacity check refuses. If that transaction is poisoned, later admission
observes the poisoned error and disables admission; it does not recover partial
metadata or clear poison to resume loads. Already-admitted cleanup uses its stable handles,
owned stages, reserved nodes and terminal atomics, never that mutex or a fresh
lookup. Thus a poisoned registry cannot make Drop wait, allocate or abandon
native ownership. If poisoning interrupted a host-only provisional transaction,
the disabled registry may retain its bounded unused charge; it still cannot
admit again or contain a permanent native owner. Unexpected invariant failure
uses the explicit containment path, not a second panic during unwind.

This guard handles **host Rust unwinding between returned FFI calls**, not a
foreign function that fails to return an acquired pointer or crosses the ABI
with an exception. No cleanup progress is promised through abort, SIGSEGV,
process termination, native hang or allocation abort. Rust's
[FFI unwinding contract](https://doc.rust-lang.org/nomicon/ffi.html#ffi-and-unwinding)
states that a panic crossing a non-unwind ABI aborts and a foreign exception
entering Rust there is undefined behavior. catch_unwind is not C++ exception
containment. Preserve the exact C/C++ SDK ABI; do not relabel it C-unwind.
Such events need existing subprocess isolation/timeout; production host
crash protection remains a separate full-goal gate. No retention or stack
workaround is proposed to conceal them.

### R1 response and bounded schedule proofs

Line references identify immutable `f78`, not this revised document. These
describe implemented invariants. The matching 13 pure lifecycle tests have
executed on Windows and native Linux, including full 512-node cleanup and the
record/byte limits. CF initializer/export/refusal ownership assertions remain
native Mac test gates; pure schedules do not prove them.

| R1 finding                                        | Concrete implemented schedule / required native outcome                                                                                                                                                                                                                                                                                                                                      |
| ------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1, P1, reentrant retirement (`143–151,194–207`)   | Owner T has live ticket A; its load B reserves the same executable and entry drops last A. A transfers its existing node; no exit runs on that callback stack. After B's native calls return, T drains A: factory release, one exit, CF release. B's reservation remains held throughout. At most 32 same-executable nodes exist, within 512 total; no cleanup capacity is acquired in Drop. |
| 2, P1, wait cycle (`198–202`)                     | T1 owns A and T2 owns B. Their native callbacks try cross-loads: each same-thread native-scope check returns Busy before a registry/native action. A callback's joined worker loading A likewise receives Busy because A is reserved/foreign-owned. No host logical wait edge exists. Later retries are new caller operations, not loops inside the loader.                                  |
| 3, P1, reservation coverage (`159–180,194–208`)   | Reserve node/history/gate before CFBundleCreate. An initializer reentering load gets Busy; reentrant Drop queues. If export lookup fails, release copied CF values and the loaded bundle before publishing terminal gate state, with zero entry/exit. Creation, final deallocation and loaded-failure release are all covered.                                                               |
| 4, P1, stale retired identity (`163–170,210–214`) | A retires cleanly while another native owner may retain its image. Replace the same-path executable or plist; canonical bundle/path history still finds A even if inode/key changes. Mark StaleSource and refuse B before bundle creation. Restoring A cannot erase StaleSource. An unchanged generation may reload, but no observed changed generation is admitted until restart.           |
| 5, P1, unwind (`176–180,204–209`)                 | A host panic after FactoryOwned but before publication invokes the existing guard: disable admission, take/release factory, clear/call attempted exit once, take/release bundle, mark terminal. A poisoned mutex is never needed by cleanup. Reentrant nodes drain on T with their own reserved stages. A foreign abort/exception is expressly outside this schedule.                        |
| 6, P2, growth (`194–207`)                         | The 257th known source or a request exceeding 8 MiB history gets Capacity with zero CFBundleCreate/entry/factory calls. The 513th ticket likewise gets Capacity without consuming an existing cleanup cell. Dropping all 512 existing tickets still needs no extra storage. Exit-refused/stale history is not evicted to turn the next request into fresh admission.                         |

During drain a callback can drop a ticket for another executable on T: append
its reserved node, hold that executable's idle reservation, and continue the
same drain. Reentered loads cannot create new T tickets, so the queue reaches
a fixed point after no more than the initially outstanding 512 nodes, provided
all foreign calls return. This also covers exit/factory/CFRelease callbacks
that trigger Drop. Native progress on unrelated owner/key pairs adds no ticket
on T because live owner affinity rejects them. An instance outliving its
PluginModule merely keeps its existing ticket; it does not create a queue
cycle or permit early exit.

If exit returns false during this drain, mark Refused before exposing terminal
state, never retry exit, release host CF ownership anyway, and refuse future
admission. Existing other tickets still retire with their own one exit each;
the refusal is not permission to drop their cleanup obligations. If a source
change is detected after native work has begun, mark StaleSource, publish no
new module, and retire under the same reservation; detection cannot undo
initializers already run.

This Mac design does not repair or claim reproduction of the separate
unproven Windows/Linux Weak-retirement race described in
[the initial portability diagnosis](VST3-PORTABILITY.md). The frozen Linux
identity fix and Windows behavior remain unchanged.

## Native fixture and focused test seams

Mac-only `plugin_file(folder, "*.vst3")` publication creates:

```text
fixture.vst3/
  Contents/Info.plist
  Contents/MacOS/windfall-fixture
```

Declare CFBundleExecutable `windfall-fixture`, package type BNDL, format version,
bundle version, and a scratch-bundle-specific identifier. Deliberately make
the executable name different from the bundle stem. Publish a complete sibling
temporary directory then rename under the existing COPY mutex. Never let an
`exists()` check accept a partly constructed bundle. Keep CLAP and non-Mac
copies/build commands unchanged.

Initially wrap the existing cdylib as that executable. Natively inspect its
Mach-O type/CPU/dependencies and load it through CFBundle. If the target OS
rejects that output, stop and report the native error before proposing a new
Mac fixture linker step. Do not substitute Library::new as support evidence.

The parent released the required declaration/entry window. The only shared
fixture edit is the Mac-only module declaration in `vst3.rs`; new
`vst3/bundle_entry.rs` supplies entry/exit. Entry accepts
and validates a real non-null CFBundle reference and balances its own retained
references/counter, including refused attempts in test probes. Do not alter
GetPluginFactory, class tables, IDs 0–20, CreatorThread, or any processing,
parameter, capture, drop, deactivation or R4 point behavior. Keep the original
Windows/Linux entry exports intact. There is no need to edit fixture `lib.rs`.

New Mac-focused `tests/vst3_bundle_macos.rs` uses independent
`tests/fixtures/vst3_bundle_lifecycle.cpp`, compiled by native Xcode clang
against installed CoreFoundation. It has small independently written ABI
declarations checked against primary pinned VST3 interfaces, without SDK
implementation copying/linkage or a Cargo dependency. The clang subprocess has
a 30-second timeout. The probe implements the exact IPluginFactory ABI and
uses an external file ledger; releasing its owned factory records an event
while entry is live. Variants omit one export, refuse entry, return a null
factory, refuse exit, or reenter during the second native entry. It must balance SDK-style retains
even on refusal. No retained observer library or Rust plugin TLS is needed.
The interface provenance is Steinberg pluginterfaces
`4f547e8e102b47de4a8b8aaf343c73b700786372`, independently resolved with
read-only git ls-remote and browsed pinned objects:
[factory](https://github.com/steinbergmedia/vst3_pluginterfaces/blob/4f547e8e102b47de4a8b8aaf343c73b700786372/base/ipluginbase.h#L56-L198),
[FUnknown](https://github.com/steinbergmedia/vst3_pluginterfaces/blob/4f547e8e102b47de4a8b8aaf343c73b700786372/base/funknown.h#L342-L366).
Native static assertions check every CF function used and actual target widths,
including CFPropertyListFormat's CFIndex width. CF parser/string facts also use
the same pinned Apple CFPropertyList/CFString headers. The Rust loader has
only a Unix frontend typecheck here, not an Apple ABI/link or execution result.

Positive tests use the **actual 21-class SDK fixture** through PluginHost and
the scanner subprocess, checking all original class IDs/order and original
valid/refusing layouts. Tests cover renamed bundles, non-stem executable names,
paths with spaces/Unicode, symlink identity, metadata retargeting, complete
parallel publication, repeated load/drop and an instance keeping code alive.
An error-path ledger is inspected after the owner has been released, not through
an extra loaded handle. Ledger balance does not prove physical code unmapping.

The seventh native test compiles the ledger with a process-local C callback
address. On the second bundleEntry, that callback drops the first PluginModule,
checks that factory/exit events have not occurred, and tries a reentrant load.
Outcome assertions run after the C callback returns; no test assertion unwinds
across FFI. Its transient callback state is removed after the second load, with
no extra CF/library observer. Require the precise deferred-release ledger and
Busy outcome on native Mac, not merely successful compilation.

The released Mac-only real-component probe uses the private
`tests/common/macos_bundle.rs` owner from `tests/vst3_native_thread.rs`.
It loads the real bundle,
enters it, holds owned factory/component/bundle through the intentional foreign
thread call, and preserve both original refusal messages. Its non-Mac branch
stays `5d`. The other three identity tests remain unchanged. A raw cdylib helper
test proves identity behavior only, not host VST3 bundle support.

## Exact implemented source scope

The superseding grant covers these paths; all changes are in this bound worktree:

| Owner/path                                                   | Maximum requested edit                                                                                                                                                               |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| T3 `src/vst3/mod.rs`                                         | Mac declaration/dispatch and one owned ModuleData lease; two mechanical scoped factory adapters. UUIDs and descriptor/probe/create results remain.                                   |
| T3 NEW `src/vst3/bundle_macos.rs`                            | Private CF owner/resolver/stages; NEW `src/vst3/lifecycle.rs` and `src/vst3/lifecycle/tests.rs` contain bounded admission/retirement plus 13 pure regressions.                       |
| T3 `src/paths.rs`                                            | Mac resolver dispatch and accurate raw/bundle documentation; existing public signatures/schema and Windows/Linux rules remain.                                                       |
| T3 `tests/common/mod.rs`, NEW `tests/common/macos_bundle.rs` | Mac-only complete bundle publication and known-fixture CF probe ownership. Existing build/CLAP/non-Mac behavior remains.                                                             |
| T3 `tests/vst3_native_thread.rs`                             | Only Mac branch of actual component guard subprocess; preserve all four tests/assertions and non-Mac behavior. Release is included in the superseding grant; `5d` remains immutable. |
| T3 NEW `tests/vst3_bundle_macos.rs`, NEW lifecycle C++ probe | Native Mac path/entry/cleanup/retirement and unchanged 21-class factory/scanner coverage.                                                                                            |
| N4 serialized release: fixture `src/vst3.rs`                 | Only new Mac private module declaration, released by the parent.                                                                                                                     |
| T3 NEW fixture `src/vst3/bundle_entry.rs` under that release | Only CFBundle entry/exit exports and their own ownership.                                                                                                                            |

Host processor/instance/bridge, fixture native_thread/lib.rs/classes, root
lib.rs, Cargo manifests/features/locks, CI YAML, P1/N4/M1 unrelated windows,
and the original realtime assertions remain closed. Existing `tests/vst3_host.rs` and `tests/vst3_scan.rs` are enabled on Mac under
the granted test compatibility scope. Host changes only its platform cfg; scanner
adds correct Mac bundle packaging. Assertions are unchanged. Mac parallel test
owners get separate complete fixture copies; same-owner calls share their path.
Non-Mac fixture publication and suite behavior remain unchanged.

## External native execution and bounded validation

This worker is Windows 11 x86_64 with Rust 1.99.0 commit
`b940084d7eb6a299eb4bfeb8e34901bc051e7ac4`. Installed Rust targets are Windows
MSVC, Linux GNU and Wasm; no Mac target, Xcode/xcrun, Apple clang, native
CoreFoundation or native Mac execution environment is present. ssh exists,
but no authorized Mac endpoint/access has been provided. Existing Docker/WSL
Linux and MSVC tooling are not Mac execution. Nothing was installed. Authorized
local Windows/native Linux builds and excluded Unix frontend checks ran; no
Apple SDK build, link or execution ran.

The parent will arrange native Mac testing through CI at the exact authorized
candidate source, with actual runner SDK/header and executable provenance.
This worker does not dispatch CI; the parent arranges the exact-source Mac run.
The completed root702/root427 Mac failures are verified baselines, not candidate
implementation results. A local Windows cross-build cannot satisfy that gate.

Run sequentially, with a timeout for every native subprocess and stop on the
first unexpected failure:

1. Record sw_vers, uname -m, rustc -vV, xcrun compiler/SDK path/version, source
   commit/clean status. Preserve actual SDK headers and hashes. Compile/run the
   native C++ ledger probe against that SDK with strict warnings. Its static
   assertions verify every CF FFI used by the loader/fixture, including actual
   target widths; the earlier excluded `mac_cf_contract_probe.c` remains only
   an uncompiled preliminary probe. Capture otool/nm/file output and fixture/test
   executable hashes.
2. The root702 completed native CI run already establishes the baseline red
   signal. The focused command for authorized native Mac CI is
   `cargo test -p windfall-plugin-host --test realtime r4_inactive_and_timed_vst3_points_reach_native_at_their_frames -- --exact --nocapture --test-threads=1`,
   with the original test. At the candidate commit require a native pass with
   the original assertions. The existing baseline failure need not be rerun
   merely to restate a proven unconditional refusal.
3. Run the new CFBundle path/entry/error ledger tests serially. Require both
   real-bundle positive loading and all missing-export/refusal/null-factory/
   factory-before-exit/retirement cases. The new native entry callback must
   drop old A during B's entry, observe no F/X during the callback, receive Busy
   on reentrant load, and then observe I/E/G/E/G/F/X/F/X after both tickets
   retire. Check native CFError cleanup for invalid binaries; conversion faults
   and CF allocation failures remain unobserved error-path inspection gates.
   A successful link or CFBundleCreate alone is insufficient.
4. Run `cargo test -p windfall-plugin-host --test vst3_native_thread`. All four
   regressions must pass, including wrong live owner and actual unload before
   worker exit. Mac still has the original ThreadId fallback; no Mac identity
   repair is presumed. If it faults, capture native PC/maps/backtrace and actual
   pthread/TLS headers before requesting a distinct identity grant. Never keep
   code loaded or skip the regression to make this gate pass.
5. Run unchanged realtime 20 serial (`-- --test-threads=1`), then default
   parallel. All eight named VST cases must pass with every original allocator,
   point/readback, admitted-note, failure and drop assertion intact. Follow
   with at most 10 sequential default-parallel repeats, each bounded to
   30 seconds after fixtures are built. Stop on the first failure and diagnose;
   do not turn repeated passing compilation into a repair claim.
6. Run new native Mac 21-class scanner/lifetime tests and host all-feature,
   all-target Clippy/fmt. The newly enabled original Mac VST host 12/scan 2 suites must also pass; their
   assertions were preserved. A native
   arm64 result is arm64 evidence; x86_64 or universal claims need actual
   validation of those slices, not cross compilation.
7. In the bound source worktree, using the implementation/build grant, run
   Windows identity 4, realtime 20 default parallel, VST host 12 and scan 2;
   host all-feature/all-target Clippy/fmt. Run native Linux focused 4, realtime
   serial/default parallel, VST host 12/scan 2 with no retention shim; at most
   10 sequential 30-second parallel repeats, stop first failure. Preserve
   exact `5d` helper/non-Mac behavior. Parent-reported Ubuntu CI success at702
   is already separate confirmation for the Linux repair; later Mac-candidate
   regression results must retain their own source/platform provenance.

Acceptance is a narrow headless Mac bundle/entry/ownership/fixture gate at the
tested commit and architecture. It does not prove physical plugin compatibility,
native Mac editors, full desktop release support, other OS native execution,
or parity closure. The checkpoint is an implementation candidate for parent native CI and the
final Standards/Spec pair. It cannot claim Mac acceptance before those results.
Frozen `5d` and `f78` stay frozen.

## Reviewable artifacts

This document accompanies the owned implementation/test source checkpoint. Private
excluded artifacts are under `target/diagnostics/vst3-portability/macos-702`:
14 exact root source snapshots, both native failure excerpts, primary source
copies,
`provenance.json`, and the uncompiled native header probe. The recording script
is `target/diagnostics/vst3-portability/record_macos_702.py`. Provenance includes
git blob IDs, source/log SHA-256, fixed SDK/Apple commits, local compiler/targets,
the parent-supplied actual Ubuntu success, and an explicit
no-worker-Mac-execution marker. Existing Linux diagnostics and the frozen
identity repair were not modified.

## Local candidate validation record

These results exercise the owned candidate source; they do not change the
attribution of the earlier root CI results. Native Mac remains pending.

| Check                                               | Observed result                                                                                                                                                                                                                   |
| --------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Windows x86_64 MSVC host library tests              | 65 PASS, one existing ignored fixture-stream test; includes all 13 new pure lifecycle regressions.                                                                                                                                |
| Windows native identity/unload, VST host, scanner   | 4/4, 12/12, 2/2 PASS, respectively. Original assertions/messages remain.                                                                                                                                                          |
| Windows realtime                                    | 20/20 PASS serial and default parallel; no ignored tests or diagnostic retention. Final source also passed default parallel.                                                                                                      |
| Native GNU/Linux x86_64 host library tests          | 58 PASS, one existing ignored fixture-stream test; includes all 13 pure lifecycle regressions.                                                                                                                                    |
| Native GNU/Linux identity/unload, VST host, scanner | 4/4, 12/12, 2/2 PASS on the final source snapshot.                                                                                                                                                                                |
| Native GNU/Linux realtime                           | 20/20 PASS serial, then 20/20 PASS default parallel. No extra repeat loop, retained native observer, preload instrumentation, or stack workaround.                                                                                |
| Host strict Clippy                                  | Windows all-feature/all-target PASS; Windows-hosted GNU/Linux target all-feature/all-target PASS.                                                                                                                                 |
| Mac Rust integration frontend                       | All selected Mac loader/path/test branches PASS strict all-feature/all-target Clippy in an excluded Unix frontend copy; framework link directives removed only in that copy. This is no Apple ABI, SDK, link or execution result. |
| Fixture strict Clippy                               | FAIL on the preserved nested if at `test-plugins/src/vst3.rs:743` (`collapsible_if`). The fixture diff is only the two-line Mac module declaration. No frozen class/processing edit or lint suppression was made.                 |
| Native Linux Clippy availability                    | The existing offline image lacks cargo-clippy; no installation attempted. Linux target linting above ran on the installed Windows Clippy frontend, separately from native test execution.                                         |
| Owned formatting/whitespace                         | rustfmt check, Prettier 3.9.9 check and git diff --check PASS.                                                                                                                                                                    |
| Native Mac SDK/ABI/lifetime acceptance              | PENDING: seven new native bundle tests, original 4 identity/unload, 20 realtime serial/default parallel, 12 host/2 scan, strict Clippy/fmt and actual Mach-O inspection. No candidate Mac build or execution is claimed.          |

Native Linux used the existing offline Docker image
`rust@sha256:24e632c09342c20abf8312cf4f61430a911c01ed3a5e4c02b87292b1c39c5273`,
Rust 1.99.0 `b940084d7eb6a299eb4bfeb8e34901bc051e7ac4`,
x86_64-unknown-linux-gnu, Debian glibc 2.41. The excluded
`mac-candidate-linux-checkpoint-final` snapshot records SHA-256 for every source
input, native command, status, time and complete output. Only its private
workspace member list/lock were reduced to the existing offline core/dsp/host
dependencies; committed Cargo files are unchanged. Tests ran sequentially with
a 300-second build/test subprocess cap and stop on test failure; final realtime
execution took under four seconds per run. The Mac plan retains its separate
30-second post-build repeat cap. The failed fixture lint is reported, not treated
as a passing check.

The excluded `mac-cfg-frontend-final/provenance.json` records the exact source
inputs and the private cfg/link substitutions. Additional primary copies are
under `mac-candidate-primary`, with downloaded hashes below. These are research
artifacts, not redistributed SDK implementation or installed target headers.

| Pinned primary file                      | SHA-256                                                            |
| ---------------------------------------- | ------------------------------------------------------------------ |
| Steinberg `4f547e8` `base/ipluginbase.h` | `e10e9a4b9b0811c392af5758542e1f72875b56ebe7e1673d5075bc1160d9fd0b` |
| Steinberg `4f547e8` `base/funknown.h`    | `e0d9609224fe15491c9ccd1463d964c303f1d5a6149fabf849e87f2c14be1951` |
| Apple `dc54c6b` `CFPropertyList.h`       | `fd29510f66b4c626a4ff3442f6e1514ba4895702b4d394e83bede011c9608b6a` |
| Apple `dc54c6b` `CFString.h`             | `cf75529893cdc01679d6e02ebfdf2c3ba1357ff671a49ae9f2468507737e99b3` |

The final checkpoint contains only the 15 owned source/test/document paths
listed above. Diagnostic scripts, binaries, downloaded headers, build outputs
and logs remain excluded. No import, amendment, root write or remote operation
occurred. The parent arranges one final independent Standards/Spec pair and
the actual exact-source Mac CI run; this worker creates neither a review thread
nor a nested agent. The 342-row full goal remains active.
