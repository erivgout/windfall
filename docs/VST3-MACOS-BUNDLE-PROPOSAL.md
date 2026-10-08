# Native macOS VST3 bundle proposal at root 702

Prepared 2026-10-08 by T3 in the existing bound worktree
`gpt/t3-vst3-portability-ci`, from immutable input
`5d3fe8e7c7f35cb1c0fd4538cfbc381796296387`. The parent authorized freezing
only this document in a documentation commit directly atop that input, for
independent design audit. No Mac production source, fixture, test, Cargo, CI,
import, build, install, or native execution change is authorized or made.
The parent goal remains active with 342 rows; all implementation windows below
remain proposals pending exact grants.

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
evidence; local Mac reproduction is unavailable and Mac builds are closed.

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

At the documentation-freeze briefing, the parent had pushed bounded UI repair
`427ed0a7` and CI run `37770742963` was queued. That source contains no Mac
loader repair; its outcome is not inferred or queried here. The Mac
CreatorThread fallback remains the original Rust ThreadId implementation and
an explicit possible next native failure. No Darwin identity grant is inferred.
The new fixture declaration-only Mac entry window can be serialized after
design audit; all 21 classes and frozen point/drop/owner assertions stay intact.

| Root 702 seam                              | Concrete gap                                                                                                                                                                                |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/vst3/mod.rs:72–75`                    | `Vst3Module::load` returns Unsupported before any Mac entry/factory call.                                                                                                                   |
| `src/vst3/mod.rs:25–28,365`                | Ownership currently assumes a libloading Library plus optional no-argument exit; there is no owned CFBundle or attempted-entry state.                                                       |
| `src/paths.rs:161–177`                     | A Mac directory is resolved as `Contents/MacOS/<bundle-stem>`, ignoring `CFBundleExecutable`; raw files are also accepted by the resolver.                                                  |
| `tests/common/mod.rs:53,104–116`           | Builds a Rust cdylib and copies it directly to `fixture.vst3`; it creates no Mac bundle/Info.plist.                                                                                         |
| `test-plugins/src/vst3.rs:912–931`         | Exports factory/InitDll/ExitDll/ModuleEntry/ModuleExit, but no bundleEntry/bundleExit. Factory still exposes all 21 classes, indexes 0–20.                                                  |
| `tests/vst3_native_thread.rs:115–197`      | Its real component owner-guard subprocess directly libloads the raw fixture. Publishing a Mac directory requires a Mac-only probe owner adaptation.                                         |
| `tests/vst3_host.rs`, `tests/vst3_scan.rs` | Existing whole-file Windows/Linux gates; catalog bundle construction also encodes Windows/Linux layout. Enabling these suites is a separate grant, not part of removing the eight refusals. |

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

## Minimum implementation proposal, pending exact grant

Use a Mac-only loader branch with a private CFBundle owner. Keep public
PluginHost/PluginModule APIs, the pinned VST3 interfaces, UUID conversion,
descriptor/probe/create behavior, and the Windows/Linux loading branches.

Proposed private operations in new `src/vst3/bundle_macos.rs`:

```text
BundleLocation::resolve(path) -> io::Result<BundleLocation>
    canonical bundle directory, canonical actual executable, source stamp
PreparedBundle::open(location) -> Result<PreparedBundle, PluginError>
    owned CFBundleRef; no VST entry attempted
PreparedBundle::load_factory() -> Result<(owned factory, MacBundleOwner), PluginError>
    executable loaded, mandatory exports verified, attempted entry paired
MacBundleOwner::retire(factory) -> ()
    release factory, call exit once, release owned bundle, complete lifecycle
```

These describe private responsibilities, not compiled Rust declarations.
Make the helper visible to `paths.rs` through a Mac-only declaration in
`vst3/mod.rs`; no root `lib.rs` or dependency edit is needed. Mac ModuleData
has an optional Mac owner instead of Library/exit fields. Its Drop moves out
the factory and owner for ordered retirement. Non-Mac ModuleData fields and
Drop behavior remain the current code. Do not refactor processing/instances.

1. Require a real `.vst3` bundle directory on Mac. Form a file-system CFURL
   from native path bytes, checked length, and a directory flag; reject NUL
   bytes and conversion failures. Own/release every temporary CFURL/CFString.
   A raw dylib renamed `.vst3` is a truthful load error on Mac, not a route
   around bundleEntry. Windows/Linux raw loading remains supported.
2. Resolve CFBundleCopyExecutableURL, convert its file-system representation
   without lossy string conversion, canonicalize it, and require a regular
   file. Share this resolution with `paths::vst3_binary` and identity stamping;
   keep their public types/schema. Resolution does not call entry or factory.
3. Check fresh directory metadata against the bundle instance's resolved
   executable. Require a valid CFBundleExecutable for the supported modern
   `Contents/MacOS` layout. Compare the freshly declared single-file executable
   name and canonical file against CFBundle's copied executable URL; validate
   CF value types and release the copied dictionary. A changed declaration, stale CFBundle instance,
   or unresolved executable must fail identity/load consistently rather than
   approve one binary and call another. Use public CF APIs; no private cache
   flush. Native metadata-retarget tests are required. Legacy flat Mac layouts
   are outside this minimum stage and get a clear load failure.
4. Load with CFBundleLoadExecutableAndReturnError. A nullable error out-pointer
   starts null. Own/release any returned CFError even when description copying
   or conversion fails. A generic load message remains available. Verify
   bundleEntry, bundleExit and GetPluginFactory **before attempting entry**.
5. Arm an attempted-entry ticket before calling entry with the real owned
   CFBundleRef. Only call GetPluginFactory after entry succeeds; convert its
   non-null result with the existing owned ComPtr ABI. Transfer complete
   ownership to ModuleData only after the factory exists. Failure guards must
   clean up while their resolved code and bundle are still owned.
6. Normal Drop releases factory before exit, calls exit once while code is
   available, and releases CFBundle ownership last. Existing component/
   controller/processor ownership continues to hold the module until native
   objects are gone and audio ownership has returned. No new blanket unsafe
   Send/Sync implementation for CF references is proposed.

### Mac lifecycle serialization without changing the Linux cache

The minimum Mac design uses one balanced SDK entry/exit ticket per successful
host load, with Arc clones retaining that ticket for instances. SDK repeated
pairing permits this; a Mac Weak cache is not required. Existing Windows/Linux
Weak caching and raw dlopen-handle conversion remain untouched.

Use a Mac-only per-canonical-executable lifecycle gate for entry, factory
acquisition/release, exit, and final CF ownership release. Store only gate
metadata in the registry, never a strong module/factory/CFBundle reference.
Serialize SDK counter changes and retirement even when a new load arrives
before another load's destructor finishes. Independent bundles may progress
independently. Same-thread reentry into an occupied gate returns a Load error;
other callers wait for its completion. Registry/state mutexes are released
before plugin code executes; the logical reservation remains held. Audio
does not visit this registry or wait.

Failure cleanup runs within its existing reservation, not by trying to acquire
the same gate again. A poisoned gate refuses new loads but cannot bypass an
already-owned ticket's cleanup. An exit refusal leaves a refused gate for
future loads until a fresh process; its metadata holds no native code owner.
Complete/notify after explicit CF release,
including any final deallocation, not before implicit Rust field Drop. Permit
canonical symlink aliases of the same bundle. Refuse conflicting live bundle
locations/source stamps for one executable rather than silently supply the
other bundle's resource context. Changed code/metadata still resident in CF
or dyld may require a fresh scanner/host process; do not promise hot replacement
from a new size/date stamp alone. Native tests must establish these boundaries.

This is a new Mac implementation contract, not a repair claim for the existing
unproven Windows/Linux Weak-retirement race described in
[the initial portability diagnosis](VST3-PORTABILITY.md).

### Failure and cleanup ledger

| Boundary                                                       | Required observable behavior                                                                                                                                                                                              |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Bad directory/metadata/architecture or executable load failure | No VST entry/exit/factory; release all owned CF values and any CFError.                                                                                                                                                   |
| Missing entry, exit, or factory export                         | Refuse before entry; zero entry and zero exit calls; release bundle ownership.                                                                                                                                            |
| Entry returns false                                            | One attempted entry and one exit; zero factory calls; no published module.                                                                                                                                                |
| Entry succeeds, factory returns null                           | One entry, one factory call, one exit; no owned factory/module survives.                                                                                                                                                  |
| Descriptor/probe/create/capture fails after load               | Preserve existing error and object teardown; the caller's live module remains valid. Final module retirement releases factory then exits once.                                                                            |
| Multiple host loads or an instance outliving its PluginModule  | Tickets stay balanced; no exit before that ticket's native references are released. One dropped caller cannot retire another caller's ticket.                                                                             |
| Exit returns false                                             | Record the refusal in focused diagnostics; do not retry exit or call factory afterward. Still release host CF ownership, and refuse future loads through that gate. Drop cannot return a new public success/error result. |
| Reload while retirement is delayed                             | No overlapping native lifecycle section; next entry occurs only after factory release/exit/CF release completes.                                                                                                          |
| Crash/hang in native code                                      | Existing scanner containment/timeout applies. RAII cannot promise cleanup after process termination.                                                                                                                      |

## Native fixture and focused test seams

Mac-only `plugin_file(folder, "*.vst3")` publication should create:

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

After N4 releases an exact **new** entry window, add Mac-only private module
declaration in fixture `vst3.rs` and new `vst3/bundle_entry.rs`. Entry must accept
and validate a real non-null CFBundle reference and balance its own retained
references/counter, including refused attempts in test probes. Do not alter
GetPluginFactory, class tables, IDs 0–20, CreatorThread, or any processing,
parameter, capture, drop, deactivation or R4 point behavior. Keep the original
Windows/Linux entry exports intact. There is no need to edit fixture `lib.rs`.

Propose new Mac-focused `tests/vst3_bundle_macos.rs` plus independent
`tests/fixtures/vst3_bundle_lifecycle.cpp`, compiled by native Xcode clang against
native CoreFoundation and primary pinned VST3 interface headers. No new Cargo
dependency is proposed. The probe implements the exact IPluginFactory ABI and
uses an external file ledger; releasing its owned factory records an event
while entry is live. Variants omit one export, refuse entry, return a null
factory, refuse exit, or delay retirement. It must balance SDK-style retains
even on refusal. No retained observer library or Rust plugin TLS is needed.
Pin the interface-header revision and hashes before compiling this probe.

Positive tests use the **actual 21-class SDK fixture** through PluginHost and
the scanner subprocess, checking all original class IDs/order and original
valid/refusing layouts. Tests cover renamed bundles, non-stem executable names,
paths with spaces/Unicode, symlink identity, metadata retargeting, complete
parallel publication, repeated load/drop and an instance keeping code alive.
An error-path ledger is inspected after the owner has been released, not through
an extra loaded handle. Ledger balance does not prove physical code unmapping.

The frozen real-component wrong-owner regression also needs a separately
granted Mac-only fixture-probe owner in `tests/vst3_native_thread.rs` (and a
private `tests/common/macos_bundle.rs` helper). It must load the real bundle,
enter it, hold owned factory/component/bundle through the intentional foreign
thread call, and preserve both original refusal messages. Its non-Mac branch
stays `5d`. The other three identity tests remain unchanged. A raw cdylib helper
test proves identity behavior only, not host VST3 bundle support.

## Exact prospective source windows

All rows below are **proposals**, not grants:

| Owner/path                                                   | Maximum requested edit                                                                                                                                                                |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| T3 `src/vst3/mod.rs`                                         | Mac helper declaration, Mac load dispatch, platform-specific ModuleData owner fields and ordered Drop. Existing factory/UUID/probe/create bodies remain.                              |
| T3 NEW `src/vst3/bundle_macos.rs`                            | Private CF ownership/resolution/entry/exit/error/lifecycle gate.                                                                                                                      |
| T3 `src/paths.rs`                                            | Mac resolver dispatch and accurate raw/bundle documentation; existing public signatures/schema and Windows/Linux rules remain.                                                        |
| T3 `tests/common/mod.rs`, NEW `tests/common/macos_bundle.rs` | Mac-only complete bundle publication and known-fixture CF probe ownership. Existing build/CLAP/non-Mac behavior remains.                                                              |
| T3 `tests/vst3_native_thread.rs`                             | Only Mac branch of actual component guard subprocess; preserve all four tests/assertions and non-Mac behavior. Requires explicit release despite T3 ownership because `5d` is frozen. |
| T3 NEW `tests/vst3_bundle_macos.rs`, NEW lifecycle C++ probe | Native Mac path/entry/cleanup/retirement and unchanged 21-class factory/scanner coverage.                                                                                             |
| N4 serialized release: fixture `src/vst3.rs`                 | Only new Mac private module declaration; no edits until parent confirms exact release.                                                                                                |
| T3 NEW fixture `src/vst3/bundle_entry.rs` after that release | Only CFBundle entry/exit exports and their own ownership.                                                                                                                             |

Host processor/instance/bridge, fixture native_thread/lib.rs/classes, root
lib.rs, Cargo manifests/features/locks, CI YAML, P1/N4/M1 unrelated windows,
and the original realtime assertions remain closed. Existing VST host/scan
Mac cfg expansion is later work requiring a separate reviewable grant.

## External native execution and bounded validation

This worker is Windows 11 x86_64 with Rust 1.99.0 commit
`b940084d7eb6a299eb4bfeb8e34901bc051e7ac4`. Installed Rust targets are Windows
MSVC, Linux GNU and Wasm; no Mac target, Xcode/xcrun, Apple clang, native
CoreFoundation or native Mac execution environment is present. ssh exists,
but no authorized Mac endpoint/access has been provided. Existing Docker/WSL
Linux and MSVC tooling are not Mac execution. Nothing was installed or built.

The parent will arrange native Mac testing through CI at the exact authorized
candidate source, with actual runner SDK/header and executable provenance.
This worker will not dispatch CI or remotely execute without an exact grant.
The completed root702 Mac failure is a verified baseline, not a candidate
implementation result. A local Windows cross-build cannot satisfy that gate.

Run sequentially, with a timeout for every native subprocess and stop on the
first unexpected failure:

1. Record sw_vers, uname -m, rustc -vV, xcrun compiler/SDK path/version, source
   commit/clean status. Preserve actual SDK headers and hashes. Compile/run the
   excluded `mac_cf_contract_probe.c` against that SDK with C11 and strict
   warnings; verify every FFI used by the eventual helper. It is uncompiled
   here. Capture otool/nm/file output and fixture/test executable hashes.
2. The root702 completed native CI run already establishes the baseline red
   signal. The focused command for authorized native Mac CI is
   `cargo test -p windfall-plugin-host --test realtime
r4_inactive_and_timed_vst3_points_reach_native_at_their_frames --
--exact --nocapture --test-threads=1`, with the original tests. Expect the
   original Unsupported; record it if rerun, then use a separately authorized
   candidate checkout. The existing CI failure need not be rerun merely to
   restate a proven unconditional refusal.
3. Run the new CFBundle path/entry/error ledger tests serially. Require both
   real-bundle positive loading and all missing-export/refusal/null-factory/
   factory-before-exit/retirement cases. Check native CFError cleanup for invalid
   binaries; review or inject a focused fallback test for description-copy/
   conversion failure, without claiming an unobserved native allocation fault.
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
   all-target Clippy/fmt. Existing Mac-gated VST host/scan suites remain honestly
   outside these counts until their separate adaptation is granted. A native
   arm64 result is arm64 evidence; x86_64 or universal claims need actual
   validation of those slices, not cross compilation.
7. In the bound source worktree, after separate source/build authority, run
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
or parity closure. This checkpoint pins only the proposal document; no
implementation checkpoint is offered. Frozen `5d` stays frozen.

## Reviewable artifacts

This new document is the sole file in the documentation checkpoint. Private
excluded artifacts are under `target/diagnostics/vst3-portability/macos-702`:
14 exact root source snapshots, both native failure excerpts, primary source
copies,
`provenance.json`, and the uncompiled native header probe. The recording script
is `target/diagnostics/vst3-portability/record_macos_702.py`. Provenance includes
git blob IDs, source/log SHA-256, fixed SDK/Apple commits, local compiler/targets,
the parent-supplied actual Ubuntu success, and an explicit
no-worker-Mac-execution marker. Existing Linux diagnostics and the frozen
identity repair were not modified.
