# Portable project repairs

Source base: `055365868ef59a2c5b58833737b5142bc58971f2`. The independently
reviewed defects were reported against `bfe1be6c92e8db82fcbc458ff945228c85577147`
and remained present in the source base. Package version stays
`0.1.0-alpha.1`.

## ZIP parser binding

Preflight formerly selected an EOCD whose comment ended at EOF, while locked
ZIP 6.0.0 independently searched the original file. Its relaxed comment
boundary and retry loop could select another EOCD/ZIP64 record, allocating an
index from counts that preflight never checked. A count check after construction
could not protect this allocation.

The locked source inspected locally was
`zip-6.0.0/src/read.rs` (`get_metadata`, `read_central_header`,
`central_header_to_zip_file_inner`, `parse_single_extra_field`),
`src/read/config.rs`, `src/read/magic_finder.rs`, and
`src/spec.rs` (`find_central_directory`). `ArchiveOffset::Known` constrains the
directory probe but does not itself stop EOCD discovery or retry.

Preflight now retains its bounded directory bytes, rejects structural records
inside the archive comment, and refuses ZIP64 size/offset sentinels and extras.
The parser sees that snapshot relocated to offset zero plus a canonical ZIP32
footer with the validated count and size. Local member offsets still address
the original file. During footer discovery, earlier bytes are hidden. Once the
directory probe begins, the reader permits the four-byte probe rewind and then
only sequential directory reads, excluding the footer. Failed parsing cannot
seek back to another candidate. Original file access is enabled only after
index construction succeeds. This adapter deliberately matches the exactly
pinned ZIP 6.0.0 read sequence; changing that dependency requires reviewing the
adapter and rerunning its fallback and normal-roundtrip tests.

The allocation regression uses an in-memory fixture smaller than 4 KiB, with a
validated two-entry directory and a commented ZIP64 count of only 32. It
asserts refusal at preflight, before calling ZIP at all, and checks that public
extraction neither reaches extraction IO nor creates its managed destination.
It never invokes ZIP on that untrusted count. A separate bounded fixture shows
the stock parser accepting an empty alternate EOCD after an invalid AES extra,
while the guarded parser refuses fallback. ZIP64 member-override fixtures
check refusal before construction. No OOM reproduction was attempted.

## Numbered sample roots

For a base such as `Backup/Song 2026-10-06 18-13-05.windfall`, backup recognition
resolves project audio above `Backup`. Adding `(001)` changes recognition, so
the numbered result resolves audio inside `Backup`. The previous save flow
carried samples using the base filename's root, then updated the filename.

The numbered writer now supplies each actual publication candidate to the
ordinary carry preparation before serializing that candidate's project. The
successful candidate returns its prepared project, relinks and sample root.
Capture, save/record serialization, live document generation/edit checks and
atomic staged no-clobber publication retain their existing authority. A failed
publication race retries from the captured project and preserves competitors.

The native regression opens a real backup with relative WAV audio and saves to
the timestamp-shaped base in a different `Backup` folder. It skips an occupied
number, resolves a sample collision without overwriting it, checks history and
dirty state both with and without concurrent edits, preserves the original
project/backup/base/older version, deletes the original WAV, restarts the
session, and checks that reopened audio is loaded with identical decoded
samples and the same sample root, then runs the sampler and checks audible
output. The independent-owner race test holds both first candidates in
preparation, forcing a publication collision; its returned preparation must
match the filename that ultimately wins publication.

## Validation

Commands run on Windows in Git Bash after `source scripts/msvc-env.sh`, with
these worktree-local settings for every Cargo invocation:

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR="$PWD/target/portable-repairs-verification"
export TS_RS_EXPORT_DIR="$PWD/target/portable-repairs-bindings"
```

Only one Cargo invocation runs at a time. Generated files and target contents
are excluded from the source commit. No bindings, WASM or model regeneration
is needed by these repairs.

Red commands and results:

```bash
cargo test --locked -p windfall-archive --lib eocd_in_comment_zip64_is_rejected_before_index_initialization
```

Exit 1: one test failed at `ambiguous comment must be refused before calling
ZIP's index parser`; execution time 0.01 seconds after the isolated build.

```bash
cargo test --locked -p windfall-desktop --lib session::tests::versions::backup_shaped_numbered_destination_carries_relinks_and_reopens_audio
```

Exit 1: one test failed because the live sample root was `away`, while the
numbered file's resolved root was `away/Backup`; execution time 0.74 seconds
after the isolated dependency build. Both regressions ran against their
unrepaired seam before the corresponding fix.

Green commands and results:

```bash
cargo test --locked -p windfall-archive --lib --test archive -- --test-threads=1
```

Exit 0: 4 parser tests and 9 archive integration tests passed (0.02 and 0.27
seconds of test execution). The initial post-fix run had 3 parser tests pass
and one test assertion fail because ZIP's display string reports only
`i/o error`; inspecting the underlying `ZipError::Io` established that the
fallback guard produced the expected error. The assertion was corrected, with
no production change, before this successful run.

```bash
cargo test --locked -p windfall-desktop --lib session::tests::versions -- --test-threads=1
cargo test --locked -p windfall-desktop --lib session::tests::files -- --test-threads=1
cargo test --locked -p windfall-desktop --lib session::tests::archive -- --test-threads=1 --skip selected_native_plugin_capture_archive_and_export_round_trip
```

All exited 0: 7 numbered-save tests (0.43 seconds), 25 ordinary-save/session
file tests (0.86 seconds), and 7 archive-session tests (1.58 seconds). The real
CLAP fixture test is intentionally excluded from this scoped run: it builds a
separate fixture/scanner and belongs to the parent's combined native/runtime
validation. No scanner or plugin fixture was built here, and the prior
verification record in `PORTABLE-PROJECTS.md` is not claimed as this repair's
own result.

```bash
cargo clippy --locked -p windfall-archive -p windfall-desktop --lib --tests -- -D warnings
cargo fmt -p windfall-archive -p windfall-desktop -- --check
git diff --check
```

All exited 0. Strict Clippy includes the library and test targets of these two
packages. There were no remaining warnings or formatting errors. In total,
52 focused tests passed. No full workspace suite or application artifact build
was run for this repair.

After a T3 server restart interrupted the turn before commit, the existing
worktree still contained these edits and the private build cache. The old
Clippy process handle was unavailable. Fresh invocations of every green
command above completed with exit 0: strict Clippy (6.02 seconds), all 52
focused tests, and scoped formatting. Test execution times on this recovery
run were 0.08/0.27 seconds for archive parser/integration tests, 0.47 seconds
for numbered saves, 0.77 seconds for ordinary files, and 1.04 seconds for
archive sessions. No interrupted result is relied on for final validation.

## External evidence

The original review was read-only and did not reproduce native/OOM failure.
These bounded fixtures establish parser selection and rejection ordering,
without measuring an attacker-sized allocation or process termination.
Cross-machine throughput, power-loss recovery on specific filesystems,
macOS/Linux dialogs and a third-party plugin corpus remain external checks.
Root owns combined integration validation, UI synchronization and generated
bindings/WASM/parity; this repair does not change those surfaces.
