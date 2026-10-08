# Portable project archives and numbered saves

Windfall's desktop File menu can export a portable `.zip` project archive,
open it through **Open project**, and **Save new version** to a new numbered
`.windfall` file. These operations are native capabilities. The browser preview
explains that the desktop app is required; it does not fabricate ZIP files or
filesystem reservations.

## Workflow

**Export portable project archive** captures the current project, session
transport settings and native plugin state through the existing selected-owner
capture API. It includes all listed sample assets: factory, external, project,
recorded and edited audio, including the sources used by sliced clips. An export
does not move the live project, rewrite its paths, add an undo step or mark it
clean. Edits made during export remain in the live document; the archive contains
the captured snapshot.

Every missing source is listed in an error and no archive is published. Audio
validation also refuses existing unreadable/non-audio sources. A persistent,
scrollable project archive report keeps long lists reviewable after the toast
disappears. Schema 1 refuses incomplete exports. Pick a **new** archive filename:
existing files are never overwritten, even if another process creates the target while
the export is running. The File menu's **Cancel project archive** interrupts an
active export or archive open between IO chunks. Cancellation before the final
publication/install boundary leaves the live project and unrelated files alone.

Opening an archive uses the same unsaved-change confirmation, recording
exclusion, replacement ticket and staged plugin-factory installation as ordinary
project open. It opens as a copy with no current save path. **Save** then asks for
a normal `.windfall` destination and carries its project samples through the
existing Save as flow. Successful extractions live in the app's settings folder
under `projects/project-*`. They are retained, including after replacement or
Save as, because historical documents and external references may still use
their files. Automatic pruning of successful extractions is a later lifecycle
feature. Failed, stale and cancelled requests remove only their own temporary
extraction.

**Save new version** writes `Song (001).windfall`, then `Song (002).windfall` and
so on. An existing suffix of three to six decimal digits is advanced rather than
nested. Padding is at least three digits, with a maximum of 999999. Occupied
names, including directories and dangling links, are skipped; at most 10000
candidates are tried per request. Unsaved projects use the normal Save picker
for the base name, then write the first unused numbered file. The base file and
all older versions stay untouched.

The numbered file is staged and synced, then atomically published without
replacement. That publication is its filename reservation, so two independent
owners cannot overwrite one another. Numbered saves share ordinary save
serialization, plugin capture, sample carrying, relinking and generation/edit
checks. Carrying and relinking use the numbered destination's sample root,
including when the chosen base name resembles a timestamped file in `Backup`.
Each publication candidate is prepared for its actual filename; a competing
writer causes a retry without overwriting its file. Successful saves update the
current path only for the saved document;
concurrent edits remain dirty. The UI refetches authoritative state after a
numbered save so an old save cannot assign its path to a replacement document.

Carried samples are also staged, synced and published without replacement.
Independent saves sharing a sample filename reuse it only for identical bytes;
otherwise each project records the unique sample name it actually published.
Failed staging removes only that request's temporary file.

When a save moves the sample root, source paths retained only in undo/redo
history keep their exact original files as absolute references. This includes
additions undone before capture and additions made and undone during a pending
save. Paths in all matching history entries are relinked without adding an undo
step. Those files are deliberately referenced in place: keep them available
while the live document or subsequent ordinary saves reference them. A portable
export after redo bundles their audio. A missing historical source stays
missing; unrelated same-name audio in the destination is never substituted for
it. Source identities use the original sample ID and path: a carried file's
renamed output path does not identify a different historical source that
previously had that name.

If keeping the exact sources would create conflicting stored paths, the
already-written copy stays at its reported filename and the live document keeps
its old sample root and complete history. The error includes that filename even
when an edit made during the save caused the conflict.

## Archive schema 1

`windfall-archive` is a native crate used by the desktop shell, outside the
`windfall-sim` dependency tree. A ZIP contains:

- `manifest.json`: schema number, project member and one metadata row per sample
  ID with its unique project path, deduplicated audio member, byte length,
  original name and original `SamplePath`.
- `project.windfall`: ordinary checked format-v1 JSON, with the optional session
  settings beside the project. Opaque plugin bindings, state bytes, parameters
  and retained unsupported import state remain in this JSON.
- `audio/0001.wav` and similar generated members: one member per exact distinct
  source byte sequence. CRC32 and length narrow candidates; a streamed byte
  comparison establishes equality, so CRC collisions cannot merge different
  files.

The project JSON points at unique `samples/<sample-id>.<extension>` paths. Open
materializes regular files at those paths from the deduplicated members. This
preserves the project model's requirement that separate sample IDs have distinct
paths, plus every channel, clip, source offset and slicing attachment. Generated
member paths are ASCII; Unicode and original nested paths remain metadata.
Archives can include original absolute source paths as metadata.

No plugin executable or plugin installation directory is bundled. An archive
is portable with respect to audio, not a guarantee of plugin availability or
identical sound on a machine without its plugins. Missing-plugin state stays
opaque for later recovery. Existing host behavior supplies silence/bypass and
its capability reports. Production desktop VST3 enablement is another worker's
delivery; this change consumes the current capture API without changing it.

## Bounds and publication

Default bounds are 4096 ZIP entries (including both JSON files), 1 GiB per
member, 8 MiB per JSON member, 2 GiB archive bytes and 2 GiB expanded disk bytes.
Expanded accounting includes the unique audio and the per-sample materialized
files. Creation also bounds source staging. Member names are at most 240 bytes.
Source size and modification time are checked before and after staging; a
detected change refuses the export. External writers must keep source files
stable while packaging; this is not a filesystem snapshot service.
The central directory is capped at 4 MiB **before** ZIP parsing allocates an
index. Parsing uses the validated directory snapshot and a canonical ZIP32
footer, with alternate EOCD discovery and parser fallback disabled. Structural
EOCD/ZIP64 records in archive comments and ZIP64 member overrides are refused
before index initialization. Ordinary archive comments remain supported.
Preflight also binds each local record's name, flags, compression method, CRC
and sizes to its central record. Checked header, extra, compressed-data and
descriptor ranges must stay before the central directory and must not overlap
other members. ZIP32 data descriptors, with or without their optional signature,
must match the central CRC and sizes; descriptor-mode local fields may be zero
or match those values. Local extras may differ from central extras, but ZIP64
overrides and damaged extra fields are refused. These checks read bounded
metadata rather than decompressing payloads during preflight. UTF-8 flags,
Unicode metadata/comments and valid stored/deflated descriptor archives remain
supported under the existing generated-member path policy.
IO streams in 64 KiB chunks and checks cancellation at each chunk and at
the final boundary.

ZIP64, multidisk, encrypted, directory and non-regular UNIX entries are refused.
Only stored and deflate compression are read. All member names are checked
independently of the host OS: absolute/UNC/drive paths, traversal, backslashes,
control characters, Windows special characters/device names and trailing dots
or spaces are rejected. Duplicate and case-colliding central-directory names
are rejected before ZIP parsing can coalesce them. Manifest membership, schema,
sample IDs, project paths and sizes must match exactly; unlisted files and
external/factory audio references in the archived project are refused. Expanded
reads are bounded even when metadata lies, and ZIP CRC errors fail the request.
Native audio validation has separate decoded-float limits of 256 MiB per source
and 512 MiB total. This bounds expansion within compressed audio formats as
well as ZIP expansion. Exports and opens exceeding these limits fail with a
named source report; ordinary project loading retains its existing policy.
An otherwise valid archive with unreadable audio is refused before installation.

Export stages under the destination's existing parent and syncs before an
atomic no-clobber publication. Extraction occurs in a new request-owned directory
and is retained only after guarded installation. No archive IO, hashing,
compression, allocation, lock, wait or destruction is added to the audio
callback. Plugin capture remains off the document lock; save/record ordering is
preserved. Cancellation cannot undo an already committed publication/install.
Audio decoding and native plugin capture are existing synchronous control-side
operations; cancellation is checked between those operations and after slow
preparation at the final installation boundary. They are not interrupted inside
a decoder or plugin call.

## Dependency and verification record

ZIP is pinned to `zip = 6.0.0`, defaults disabled with `deflate-flate2` only.
`flate2` uses its Rust/miniz backend. Encryption, bzip2, zstd, xz/lzma, ppmd,
deflate64, zopfli and native compression helpers are not enabled. The exact
locked ZIP/deflate/index dependencies' license texts were inspected and their
MIT alternatives are reproduced in
[`THIRD-PARTY-NOTICES.md`](../crates/windfall-archive/THIRD-PARTY-NOTICES.md).
These are GPL-3.0-compatible. Cargo's lock contains ZIP's inactive fuzz-only
arbitrary entries; they are not runtime dependencies.

Validation is run on Windows using Git Bash, `source scripts/msvc-env.sh`,
`CARGO_BUILD_JOBS=1` and the isolated worktree's
`target/portable_projects-verification`. Binding tests use a temporary
`TS_RS_EXPORT_DIR`. Local binding/WASM regeneration is verification only and
is excluded from the source commit.

Checks on 2026-10-07 (Windows; every command below exited 0):

| Check | Result |
| --- | --- |
| `cargo test -p windfall-archive --test archive` | 8 passed |
| `cargo test -p windfall-desktop --lib session::tests::archive` | 8 passed, including a real CLAP fixture and native audio export |
| `cargo test -p windfall-desktop --lib session::tests::versions` | 6 passed |
| `cargo test -p windfall-desktop --lib session::tests::files` | 25 passed |
| `cargo test -p windfall-desktop --lib session::tests::plugins` | 2 passed |
| `cargo test -p windfall-desktop --lib session::tests::plugin_recording` | 3 passed |
| `cargo clippy -p windfall-archive -p windfall-desktop --all-targets -- -D warnings` | Passed |
| `cargo fmt --all --check` | Passed |
| `pnpm --dir apps/desktop test src/lib/flows/portable.test.ts src/features/layout/archive-report.test.tsx src/lib/actions/registry.test.ts src/lib/flows/edit.test.ts --maxWorkers=4` | 31 passed in 4 files |
| Desktop `typecheck` and `lint` | Passed |
| `node scripts/check-sim.mjs` | Current local WASM, 1,634,799 bytes |

Local `scripts/gen-bindings.sh` generated 146 binding files, and
`scripts/build-sim.sh` regenerated WASM under this worktree's `target/sim` with
one Cargo build job. Integration must regenerate/review the combined bindings
and WASM; none are part of this source commit. The existing Windows CLAP test
helper expects `target/debug/windfall-desktop.exe`. For these checks, the
scanner was built in the isolated verification target and its executable was
copied to that path **inside this worktree**. The fixture DLL uses the existing
worktree-local `target/plugin-session-fixtures` directory. These are local test
artifacts only; no other worktree's target metadata was used.

Tests cover exact audio deduplication, metadata, missing-source reporting,
traversal/case/symlink/size/schema/CRC/truncation refusal, injected read/write/
publication errors, cancellation at every IO phase, rollback ownership,
competing archive destinations, native session source removal and playback,
missing opaque plugin state, edit/replacement/reload/recording barriers,
numbered-save races, decoded-audio limits and dirty-state preservation. The real
CLAP fixture also checks that numbered saves capture the selected native state.
UI tests exercise native-only
capability, picker cancellation/errors, archive busy/cancel state and numbered
save semantics with mocked native responses, without inventing a filesystem.

Real cross-machine large-project throughput, power-loss recovery on specific
filesystems, macOS/Linux desktop dialogs and a real third-party plugin corpus
remain external checks. Synthetic Windows tests do not establish those results.

The subsequent parser-binding, local-record/extent validation, numbered
sample-root, concurrent carried-audio publication, history source-lifetime and
published-copy reporting repair evidence is
recorded in [`PORTABLE-REPAIRS.md`](PORTABLE-REPAIRS.md).
