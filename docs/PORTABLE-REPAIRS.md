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

## Round two: carried audio and retained history sources

The independent read-only review of
`d42c810ea7beee0ee49f60305ab0d21d30b00828` found the original two repairs closed,
then identified two existing save-data integrity defects. Its evidence was
pinned source tracing and bounded Python interleaving models, not Rust
execution. This round compiled native regressions before changing either
production seam.

Parent sampler preparation `42522322e96c04cffc831370cef762d2bf1a41d0` was merged
as `2b8f350e9bf72be64a32cd793dd086f9ae71ad85` before these checks. Both
`Refusal::Cancelled` and `Refusal::SamplerPreparation`, plus `archive:install`
and `sampler:install-prepared`, are retained. No sampler/runtime/browser/UI
source was edited by this round.

Two sessions could both observe a missing sample destination, then overwrite
each other's audio through `fs::copy`, despite no-replace project publication.
Samples now copy into request-owned temporary files under the destination
parent, sync, and publish through `persist_noclobber`. A competing arrival is
compared byte for byte or assigned another numbered sample name. Every retry
records that exact name in the project and matching history source. Failure
and collision drop only the owned temporary file; they never remove a final
sample. The native test holds both owners after the missing-file check, lets
the first publish, then resumes the second. It checks both exact numbered
project files, unchanged competitor/source/old project bytes, retained history,
clean successful documents, decoded samples after separate restarts, and
audible sampler output. A focused failure test also catches the previous
cleanup handler deleting a competitor after a missing-source copy failed.

Moving the sample root previously considered only current project samples.
Undone additions absent from capture, and additions made and undone during a
pending save, kept history-relative paths and could load unrelated destination
audio on redo. `Document::sample_sources` now enumerates current and all retained
edit source occurrences. The save identifies sources not carried in its exact
published snapshot and preserves their original absolute paths before changing
the live root. It relinks those identities before carried renames, so a
history-only relative name cannot force a history reset merely because a
carried sample acquired that name. `relink_sample_source` matches both ID and
old path, keeping different historical source versions separate. The existing
whole-ID `relink_sample` behavior remains unchanged. These helpers do not add
history steps or change dirty state; generation/edit and save serialization
guards remain authoritative.

If exact relinks conflict with the project model's distinct-path requirement,
the save reports the already-written copy and refuses to adopt its root. The
live document, old root and complete history remain unchanged. The previous
unchanged-save fallback silently started a fresh document in this case. A
native fixture with two history-only IDs for one original path now checks this
conservative refusal, the retained redo step, unchanged source/old project,
and the reported copy. No conflicting source identity is silently discarded.

Three native cases exercise undo before capture, undo after capture, and an
addition made and undone during a pending save. They check one-step redo/undo,
history and dirty behavior, old files and same-name competitors, exact decoded
audio, subsequent numbered save/reopen, and sampler output. A missing
history-only source remains an explicit missing original absolute path, rather
than using unrelated destination audio. Historical sources deliberately held
as absolute paths remain dependencies of the live document and later ordinary
saves; a portable export after redo bundles those reachable samples.

### Round-two native RED evidence

The existing worktree-local target and binding directories above were reused,
with `CARGO_BUILD_JOBS=1` and `RUST_TEST_THREADS=1`. All commands run serially
after `source scripts/msvc-env.sh`; no full workspace or fixture build was run.

```bash
cargo test --locked -p windfall-desktop --lib session::tests::files::portable_ -- --test-threads=1
```

Exit 1: 3 failed, 1 passed (0.15 seconds execution; 4m16s cached parent-source
recompile). The forced concurrent carry failed at `the second owner replaced
the first owner's audio`. Both pre-capture undo and pending addition/undo failed
at `redo loaded the destination's unrelated sound`. The captured-then-undone
control passed before repair.

```bash
cargo test --locked -p windfall-desktop --lib session::files::tests::portable_failed_sample_staging -- --test-threads=1
```

Exit 1: 1 failed (0.00 seconds execution). Reading the competitor after the
failed copy returned `NotFound`, proving its deletion by the old cleanup.

The initial post-fix native `portable_` run passed all 5 tests (0.16 seconds).
An intermediate compilation reported an iterator borrow lifetime error,
corrected by limiting the read-only iterator scope; an added project unit test
used the nonexistent `Project::default`, corrected to `Project::new`, and its
command-test error expectation needed `SampleId.0` for the existing `u32`
helper. No failed compilation is counted as a test result.

An intermediate file suite had 29 pass and one assertion fail because the
missing-source test assumed deleting a file discards cached decoded audio.
Production cache behavior was kept: the test now checks any cached audio is
the original sound, then cold-reopens a subsequent save and asserts a missing
original-source warning rather than destination substitution.

```bash
cargo test --locked -p windfall-desktop --lib session::tests::files::portable_source_conflict -- --test-threads=1
```

Exit 1 before the conflict-refusal guard: 1 failed (0.01 seconds), returning
success through the history-reset fallback where the test required a safe
refusal. The guard removes that fallback; it does not loosen source-path
uniqueness or the existing whole-ID relink checks.
The first post-guard file run passed 30 tests and failed only the fixture's
reported-path substring assertion because its expected Windows path contained
a mixed slash. The expectation was corrected to a native joined path; the
production refusal and its message were already correct.

### Round-two final scoped checks

Every test command below exited 0. Counts are unique tests in these scoped
filters, not including repeated regression runs:

| Command | Result |
| --- | --- |
| `cargo test --locked -p windfall-project --lib sample_sources -- --test-threads=1` | 1 passed, 0.00s |
| `cargo test --locked -p windfall-project --test commands relink -- --test-threads=1` | 5 passed, 0.00s |
| `cargo test --locked -p windfall-desktop --lib session::tests::files -- --test-threads=1` | 31 passed, 0.83s |
| `cargo test --locked -p windfall-desktop --lib session::files::tests -- --test-threads=1` | 6 passed, 0.03s |
| `cargo test --locked -p windfall-desktop --lib session::tests::versions -- --test-threads=1` | 7 passed, 0.44s |
| `cargo test --locked -p windfall-desktop --lib session::tests::archive -- --test-threads=1 --skip selected_native_plugin_capture_archive_and_export_round_trip` | 7 passed, 0.90s |
| `cargo test --locked -p windfall-desktop --lib sampler_processing_open_and_history_prepare_off_lock_and_reject_project_replacement -- --test-threads=1` | 1 passed, 0.03s |
| `cargo test --locked -p windfall-archive --lib --test archive -- --test-threads=1` | 4 parser + 9 archive integration passed, 0.06/0.20s |

Total: 71 focused tests. This includes the original bounded parser-binding
regressions and backup-shaped numbered destination round trip. Native archive
tests exercise cancellation/stale/edit/recording refusals at `archive:install`;
the sampler test exercises `sampler:install-prepared` and its history guard.
Both file-install refusal variants remain in the merged source.

```bash
cargo clippy --locked -p windfall-project -p windfall-archive -p windfall-desktop --lib --tests -- -D warnings
cargo fmt -p windfall-project -p windfall-archive -p windfall-desktop -- --check
git diff --check
```

All exited 0. Strict scoped Clippy finished in 28.94 seconds with no warnings;
formatting and whitespace checks passed. Checks reused the existing unique
target, one Cargo process at a time. No interrupted or pending process is
counted as passed.
After the final Rust doc-comment clarification, strict Clippy and scoped fmt
were rerun on the final source with exit 0 (Clippy 2.04 seconds).

The real CLAP archive fixture is explicitly excluded from this round's native
filter, and is not counted as passed. Parent reported the integrated base suite
including that fixture; this subsequent source repair still needs the parent's
actual fixture and combined integration rerun after integration. No full
workspace suite, UI tests, scanner/fixture build, bindings/WASM regeneration,
artifact edits, push or release occurred here. Version stays `v0.1.0-alpha.1`.

## Round-three source identity, ZIP local records and copy reporting

The next independent read-only review was against
`dc57588a6a0207928b00d3511e1df04dc0bc2f50`. Its evidence was source tracing
and bounded Python models, not native execution. The original allocation,
numbered-root, concurrent sample publication and ordinary history-only repairs
remain in place. This round adds compiled reproductions and repairs; it does
not establish a traversal, memory-allocation or code-execution exploit.

### Original source identities

Source tracing found no production command that changes a retained sample ID
from one path to another while leaving its older path in history. `AddSample`
allocates a fresh ID (or finds a current matching path); undo does not reuse
allocated IDs. There is no sample-path setter command. Production file moves
relink matching source occurrences together, and FLP rewrites happen before
constructing the document. Production command reachability of the review's
same-ID/different-path fixture is therefore not established.

The public exact-source helper deliberately supports that model shape. A
small internal Document fixture checks every current/undo/redo project against
the unchanged project invariants and JSON cold loading. No path-uniqueness or
schema exception is introduced. A native file fixture supplies occurrences
from two individually valid document versions through the same source iterator
used by retained history. It forces current `a.wav` to be carried as
`a (2).wav`, while a different historical source already has that original
name. Before repair, cold loading the historical version decoded the current
source's PCM instead. After repair, both versions retain their distinct exact
decoded PCM through undo/redo and fresh-cache file loading. Old source files,
the destination competitor and the carried bytes are checked unchanged.
This native fixture checks the supported source/classification seam; it is not
claimed as an end-to-end command route to the differing-path Document shape.

`left_behind` now recovers the saved sample's original `(SampleId, SamplePath)`
from its carry mapping before classifying history occurrences. It never uses
a renamed output path as evidence that a different original source was carried.
Historical occurrences are preserved as their original absolute files first,
then only matching original occurrences get the carried output path. Existing
conservative conflict refusal and complete history are retained.

### ZIP local binding and checked extents

The cached locked ZIP 6.0.0 source inspected for this round was `read.rs`
(`find_data_start`, `find_content`), `write.rs` (stream writer and descriptor
emission), and `types.rs` (ZIP32 descriptor fields). The reader uses local
name/extra lengths to find data but does not compare the local name, flags,
method, CRC or sizes with its central metadata. Its bounded compressed reader
can finish deflate before exhausting a declared compressed extent, so a larger
extent can escape those predicates without a CRC failure.

The actual public archive reader accepted all 11 bounded malformed cases
before repair: six local/central mismatches (including a local traversal name
and local encryption/method disagreements), two oversized compressed extents
(past EOF and overlapping the next local record), and three incorrect signed
descriptor fields (CRC/compressed/expanded sizes). Each fixture is under 4 KiB
and contains small valid manifest/project payloads; no large index or OOM was
attempted. Extraction uses central names, so accepting a contradictory local
traversal name alone does not demonstrate traversal.

Preflight now compares local identity/flags/method and CRC/size semantics
before initializing ZIP or creating a managed destination. Checked arithmetic
bounds local headers, names, extras, compressed extents and descriptors to the
file's data region before the validated central directory. Sorted member
ranges must not overlap. Payloads remain streamed later through the existing
CRC and expanded-size checks; preflight reads only bounded metadata. Local
names retain the existing 240-byte generated-member policy, and each extra
read/allocation is bounded by the ZIP32 16-bit length.

Both signed and unsigned ZIP32 descriptors bind all three authoritative
central fields. Descriptor-mode local fields may be zero or match the central
values. The optional-signature ambiguity is covered even when a legitimate
CRC equals the signature. Valid stored/deflated streamed archives, zero/filled
local fields, UTF-8 flags, Unicode project metadata and ordinary Unicode
comments all round-trip. A positive fixture allows unknown local-only extras
that differ from the central extras; unnecessary timestamps, extra equality
or other parser fields are not mirrored. Existing directory/non-regular,
unsafe path, encryption, ZIP64, duplicate, size and malformed CRC refusals and
the canonical central-directory snapshot adapter remain intact.

### Reporting a published copy after concurrent edits

The existing real-command conflict fixture now also makes a settings edit at
`save:write`, preserving both conflicting source identities in applied history.
The project copy is published before live relinking is conservatively refused.
Both unchanged and concurrent branches now include its exact filename and
say that the live project/history stayed unchanged. The concurrent message no
longer says saving was unfinished. The test cold-loads the published captured
snapshot, compares the entire live snapshot/root, retains undo/redo, and checks
the old project and sample bytes.

### Round-three compiled RED evidence

Commands below ran serially in the existing worktree after
`source scripts/msvc-env.sh`, with `CARGO_BUILD_JOBS=1`,
`RUST_TEST_THREADS=1`, `CARGO_TARGET_DIR="$PWD/target/portable-repairs-verification"`
and `TS_RS_EXPORT_DIR="$PWD/target/portable-repairs-bindings"`.

| Command before production repair | Result |
| --- | --- |
| `cargo test --locked -p windfall-archive --test archive local_ -- --test-threads=1` | Exit 1; 3 failed, 0 passed; all 11 malformed cases accepted; 0.13s execution |
| `cargo test --locked -p windfall-desktop --lib portable_carry_output -- --test-threads=1` | Exit 1; 1 failed, 0 passed; wrong historical PCM after cold loading; 0.02s |
| `cargo test --locked -p windfall-desktop --lib portable_source_conflict -- --test-threads=1` | Exit 1; 1 failed, 0 passed; concurrent branch omitted the published filename; 0.04s |

The new native helper fixture initially had private-module access and a
missing type import compilation error, corrected before the compiled RED run.
The first concurrent conflict fixture discarded its redo-only source when
adding the pending edit, so it did not retain the intended conflict. It was
corrected to store both sources in applied history before the pending edit;
only that corrected compiled run is the RED evidence above. No compilation
failure or inappropriate fixture interleaving is counted as reproduction.

### Round-three final scoped GREEN checks

All commands below exited 0 on the repaired source, using the same serial
MSVC environment and unique cached target/binding directories. Counts are
unique tests, excluding repeated development runs.

| Command | Result |
| --- | --- |
| `cargo test --locked -p windfall-project --lib source_tests -- --test-threads=1` | 2 passed, 0.00s |
| `cargo test --locked -p windfall-project --test commands relink -- --test-threads=1` | 5 passed, 0.01s |
| `cargo test --locked -p windfall-desktop --lib session::tests::files -- --test-threads=1` | 31 passed, 1.13s |
| `cargo test --locked -p windfall-desktop --lib session::files::tests -- --test-threads=1` | 7 passed, 0.06s |
| `cargo test --locked -p windfall-desktop --lib session::tests::versions -- --test-threads=1` | 7 passed, 0.44s |
| `cargo test --locked -p windfall-desktop --lib session::tests::archive -- --test-threads=1 --skip selected_native_plugin_capture_archive_and_export_round_trip` | 7 passed, 1.20s |
| `cargo test --locked -p windfall-desktop --lib sampler_processing_open_and_history_prepare_off_lock_and_reject_project_replacement -- --test-threads=1` | 1 passed, 0.04s |
| `cargo test --locked -p windfall-archive --lib --test archive -- --test-threads=1` | 6 parser + 15 archive integration passed, 0.06/0.41s |

Total: 81 focused tests, preserving the prior 71 and adding 10. The three
compiled RED filters are included in these GREEN suites. The archive fixtures
also check truncated/missing metadata boundaries and unsigned descriptor
damage before extraction. The CRC fixture now binds the same wrong CRC in
both local and central records, ensuring the later streamed reader still
rejects payload CRC disagreement after preflight succeeds. Existing unsafe
directory/non-ASCII generated paths remain refused.

The forced independent save interleaving still verifies both numbered files
reopen their own exact PCM and produce sampler output, with competitors and
old project/audio bytes unchanged. History-only moves, additions undone before
or during capture, dirty/history semantics, no-replace publication races and
the backup-shaped numbered destination remain covered. Source inspection and
native tests retain both `Cancelled` and `SamplerPreparation` install refusals
and the `archive:install` / `sampler:install-prepared` barriers.

```bash
cargo clippy --locked -p windfall-project -p windfall-archive -p windfall-desktop --lib --tests -- -D warnings
cargo fmt -p windfall-project -p windfall-archive -p windfall-desktop -- --check
git diff --check
```

All exited 0. Strict Clippy finished in 14.03 seconds with no warnings. No
cancelled or merely observed running command is counted as passed. Changes are
limited to owned source/tests and these two documents; no models, schema,
timeline, UI, runtime, generated bindings/WASM or artifacts changed in this
round. The package stays `v0.1.0-alpha.1`.

The actual native CLAP archive fixture is explicitly excluded and remains
parent-owned, together with the combined integration rerun on the subsequent
source commit. Parent reported base checks including that fixture; those do
not establish this later source's combined acceptance. Cross-machine large
project throughput, power-loss behavior on specific filesystems, other OS
dialogs and third-party plugin corpora remain external evidence. There was no
full workspace suite, fixture/scanner build, UI test, artifact regeneration,
push or release here.
