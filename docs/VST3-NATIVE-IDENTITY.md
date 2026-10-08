# VST3 fixture native identity repair: implementation and native evidence

2026-10-08. This checkpoint fixes the reproduced local Linux fixture
TLS-after-unload mechanism described in [VST3-PORTABILITY.md](VST3-PORTABILITY.md).
It does not establish the original Ubuntu job's fault PC or complete the full
Windfall goal. The original Ubuntu runner must rerun the repaired source.
The macOS loader/path/bundle fixture stage remains closed and unchanged.

## Source checkpoints and parent integration

Worktree: `C:/Users/ewhee/.t3/worktrees/windfall/gpt-t3-vst3-portability-ci`.
Branch: `gpt/t3-vst3-portability-ci`.
Original fixed base: `6d0773804199faba65f866feda21b8c4ba9a6b9a`.

- Accepted diagnosis/proposal document checkpoint:
  `42039fb13bbc0d817c317ae8de46a2c1ba09df8c`.
- Parent-authorized prerequisite source:
  `71dbec11fb4aeadc57fcf4ca90e92d045e4c90ff`, whose source parent is
  `1fa5a16de85e90e7a8fbc123ff29ba6c413156c3`.
- Local cherry-pick of that **single** prerequisite:
  `1c22651ef288888294ca2351d57edba9c16dde52`.
  No commit import of `1fa5a16d`, the full N4 branch, or `020` was performed.

The cherry-pick was conflict-free. All six resulting prerequisite blobs were
checked against exact `71dbec11`: fixture `bridge_behaviors.rs`, `ext.rs`,
`lib.rs`, `plugin.rs`, `vst3.rs`, and scanner `tests/vst3_scan.rs` matched.
Parent integration must apply original `71dbec11` separately, then this worker's
incremental identity fix. The local prerequisite cherry-pick is not the fix or
a merge to import wholesale. The original proposal document can be applied
separately if desired.

## Authorized change

N4 released only the fixture creator field, constructor assignment, two owner
comparisons, and private module declaration/import. Those are the only edits
in existing `test-plugins/src/vst3.rs`. The added
`test-plugins/src/vst3/native_thread.rs` owns the private helper.

`CreatorThread::current()` calls native `pthread_self()` on verified
`target_os=linux`, `target_env=gnu`, `target_arch=x86_64`. Its identity is
`std::ffi::c_ulong`. Equality follows the verified glibc header/native
implementation's unsigned-long value comparison. This avoids registering a
Rust current-thread TLS cleanup callback in the unloadable fixture.

All other targets keep the original `std::thread::current().id()` behavior;
in particular, Windows behavior is preserved. This is a narrow Linux GNU
x86_64 repair, not an assertion that other Unix identity ABIs or unload
behavior were verified. Native pthread identities can be reused after thread
exit; these guards compare the still-live creating owner. The helper neither
returns a constant identity nor disables either owner assertion.

The original `assert_eq!` calls and messages remain:

- `native lifecycle left the creating owner`
- `native destruction left the creating owner`

Reversing just the identity substitutions and module/import declaration
reproduces the exact `71dbec11` VST fixture text. Therefore all classes 0–20,
their IDs, class 19 controlled error, class 20 truthful 37-frame stable
deactivation, point/readback/drop/capture behavior, and scanner count 21 are
preserved from N4's prerequisite. Other five prerequisite paths remain their
exact frozen blobs. No host, processor, bridge, Cargo, feature, lock, CI, or
macOS source was edited for this repair.

## Target-header ABI verification

Native Linux used the existing offline image:

`rust@sha256:24e632c09342c20abf8312cf4f61430a911c01ed3a5e4c02b87292b1c39c5273`

Environment: x86_64 Linux, Debian 13.7, glibc 2.41, GCC target
`x86_64-linux-gnu`, WSL2 kernel `6.6.87.2-microsoft-standard-WSL2`.
Rust: 1.99.0, compiler commit
`b940084d7eb6a299eb4bfeb8e34901bc051e7ac4`, matching the failed CI compiler.

The actual installed target headers establish:

- `/usr/include/x86_64-linux-gnu/bits/pthreadtypes.h:27` defines `pthread_t`
  as `unsigned long int`.
- `/usr/include/pthread.h:273` declares `pthread_self` returning `pthread_t`;
  `:276` declares `pthread_equal` returning `int` with two `pthread_t` args.
- The header's inline `pthread_equal` at `:1340` returns equality of those
  values. Disassembly of the installed libc confirms compare/set-equal/return
  without dereferencing the identities.

The private C header probe compiles with `-Wall -Wextra -Werror` and uses
`_Static_assert`/`_Generic` to verify the typedef, function prototypes, size,
and alignment. It prints `size=8 alignment=8` and same-live-owner equality 1.
No unsupported ABI typedef is guessed.

Header SHA-256:

- `pthread.h`:
  `512a6c8b07a201b4e8b5b8644744c3f4c1a9e3c76c58708cf9a958f29d124d52`
- `bits/pthreadtypes.h`:
  `acad99a6a3ef489327e806c8d29b25797ca2126cd5489233837e5c9c01516cd1`

Header copies, compiler proof and libc disassembly are private diagnostic
evidence; they are not shipped source or a substitute for Ubuntu execution.

## Focused regressions

New owned integration file: `tests/vst3_native_thread.rs`. Four tests run on
both native environments; none has an OS cfg exclusion:

1. The helper accepts the calling creator and rejects another live thread
   while the creator stays alive through the join.
2. A small independently loaded cdylib imports the exact helper and rejects a
   different live owner through its native C exports.
3. The actual VST fixture factory creates class 0 using the pinned SDK class
   and interface IDs. Subprocesses deliberately call its real `setActive` and
   `terminate` guards from a different live thread, holding component and
   executable ownership on the creator. Each must fail and report its exact
   original assertion message. A removed check or constant identity would
   cause successful child completion and fail the regression. The known
   fixture's module entry exports are no-ops; this guard probe directly calls
   its factory to avoid the host's thread restrictions rejecting the misuse
   before the native assertion. It is not a physical plugin hosting claim.
4. A subprocess loads a helper cdylib on a worker, checks identity, drops the
   only library owner, exits the worker and joins it. A later unmapped TLS
   callback fails the child and therefore the parent test.

The unload regression was first run against a **private** helper snapshot with
the original Rust ThreadId implementation. It failed with the exact captured
child `signal: 11 (SIGSEGV)` after dlclose, making the parent regression red.
The repaired helper makes it pass. No production diagnostic switch, retained
module handle, fake identity, stack workaround or skipped assertion was added.

## Actual validation

Linux native inputs are private tracked-source snapshots under target. Only
the private workspace member list is narrowed to host/DSP/core, with the
previously checked offline reduced lock; package/source versions remain the
original versions. The registry is read-only. Containers use
`--pull=never --network=none`; no installs or downloads occurred.

All repaired runs below omit `LD_PRELOAD`, retained mapping and stack overrides:

- Original realtime suite, serial: **20 passed**, exit 0; 5.67 seconds in tests.
- Original realtime suite, default parallel: **20 passed**, exit 0;
  1.41 seconds in tests.
- Three additional sequential default-parallel runs, each bounded to 30 seconds
  and stopping on first failure: **20 passed each**, exit 0; 1.22, 1.26 and
  1.63 seconds in tests. No further repeats were needed.
- Final four focused regressions: **4 passed**, exit 0; 2.94 seconds in tests.
- Existing VST host: **12 passed**, exit 0.
- Existing VST scan: **2 passed**, exit 0, including native factory count 21.

The first repaired snapshot contained the first three focused tests. The final
`identity-repair-guards` snapshot adds the actual-component guard test; all
production/helper inputs are the same. Final source hashes match that snapshot
exactly. The original realtime source and every R4/readback/drop/allocator
assertion are unchanged throughout.

Windows native environment: Rust 1.99.0, MSVC x64 VS2022 BuildTools
14.44.35207, Windows SDK 10.0.26100.0. The initial default Rust linker selected
an incomplete VS18 installation and failed to find `msvcrt.lib`; using the
already-installed VS2022 environment and explicit linker resolved this without
installation or persistent configuration changes.

- Original realtime suite, default parallel: **20 passed**, exit 0.
- Final four focused regressions: **4 passed**, exit 0.
- Existing VST host: **12 passed**, exit 0.
- Existing VST scan: **2 passed**, exit 0, including native factory count 21.
- Scoped host Clippy `--all-targets -- -D warnings`: **passed**.
- Linux-target host Clippy from the installed Windows toolchain,
  `--target x86_64-unknown-linux-gnu --all-targets -- -D warnings`: **passed**.
  This is explicitly cross-target lint evidence, separate from native tests.
- Owned files pass rustfmt check; the incremental diff passes whitespace check.

Native Clippy is absent in the installed Linux image. It was not installed and
no native Linux Clippy pass is claimed. An additional strict Clippy run of the
standalone fixture crate fails on an unchanged N4 `collapsible_if` in the
event-flood branch at `vst3.rs:741`. Reverse-hook equivalence proves that branch
is the prerequisite's source. It is outside the released identity window and
was left untouched; no allow/skip or weakened lint policy was added. This is
distinct from the passing required host checks.

## Exact private provenance and artifacts

Directory: `target/diagnostics/vst3-portability` (ignored, excluded from commits).

- `repair-provenance.json`: base, original prerequisite, local prerequisite
  checkpoint, all final source SHA-256 values, and native executable/fixture
  hashes. It verifies final source matches the final guard snapshot.
- `identity-header-proof.log`, `identity_header_probe.c`, `pthread.h`,
  `pthreadtypes.h`: exact native header/ABI proof.
- `identity-baseline/regression-red-1.{json,log}`: original ThreadId behavior
  goes red in the new unload seam.
- `identity-repair/serial-1`, `parallel-1`, `repeat-1` through `repeat-3`, and
  `host-scan-1` JSON/log pairs: repaired native Linux evidence.
- `identity-repair-guards/focused-1.{json,log}`: all four final native regressions.
- `windows-focused-3`, `windows-realtime-1`, `windows-host-scan-1`,
  `windows-clippy-1`, `windows-linux-clippy-1` JSON/log pairs: successful final
  Windows/native and target-lint results.
- `identity-repair/clippy-1` and `windows-fixture-clippy-1`: explicit tooling
  limitation and inherited lint failure, retained for review.

Each run JSON records exact argv, exit status, elapsed time and platform tool
provenance. Earlier focused results and the missing-trait compile failure are
retained rather than substituted for final four-test success.

Key native SHA-256 values:

- Linux realtime executable:
  `1b71c33340783aa2a1c12fb2b7518e8e133e66ad705e3c7f03814fc3ff7a3e54`
- Linux final focused executable:
  `fa6cf4893ff36258f7df74e30cf7aa063d412c4f90775ff6e3ae09b19315a3fc`
- Linux full-suite VST fixture:
  `cfd4ab3c3c29285835fb02191b776b4b30bf8a7afe22d41d4f2e760d3104cf38`
- Windows realtime executable:
  `5198b382c3142901aac2f8615b9c7d79df72423f218a21be2d32fbafe889edc6`
- Windows final focused executable:
  `f06303febeef9359edc7096cbf49353e86d33f4264a18cea0b91dde8a68a583e`
- Windows VST fixture:
  `d6bec67720017d4d408c02d84a34a98ad3c7b97031acebb68f8ebe9bf943800c`

These hashes identify the recorded binaries, not cross-platform binary parity.
No remote CI dispatch, push, PR, release, root import, amend or reset occurred.
No native macOS/other Unix/physical plugin validation is inferred from these
headless fixture results. Parent integration and actual Ubuntu confirmation
remain separate from this worker's completed narrow local repair.
