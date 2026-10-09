# Project import arrangement seam

The importer saves multi-arrangement project identities in
`Project.playlist.arrangement_book`, using the existing checked
`AddArrangement` and `SwitchArrangement` commands. No project commands or
playback changes are required. Implementation lives in
`crates/windfall-flp/src/convert/arrangements.rs`; playlist conversion returns
the IDs of the clips and tracks it successfully created.

## Saved data

- Every arrangement in a multi-arrangement source gets an entry in source
  order, including the selected arrangement. Names are retained within the
  book's constraints: blank names use `Arrangement N`, NULs are removed, and
  names over 256 UTF-8 bytes are truncated at a character boundary. Name
  adjustments are reported.
- IDs come from the document's monotonic allocator after playlist and timeline
  conversion. Repeated imports of the same source produce the same IDs, and
  those IDs are stored in the project book. Source arrangement indices are
  not reused as document IDs.
- The active ID identifies the selected source arrangement, using the existing
  first-arrangement fallback if the selection is absent or unavailable.
- The selected entry references all successfully imported playlist clips.
  Other entries reference clips only when their complete parsed source item
  equals an item of the selected arrangement that produced a clip. Modern and
  legacy items are matched separately, once per occurrence. All clip IDs
  emitted by an item are retained, including multiple automation targets.
  Rejected items never produce references; merely sharing a pattern or channel
  does not imply the same placement.
- Track references map source track positions used by items or track records
  to existing tracks imported from the selected arrangement. Legacy blocks
  reference the top track. Unavailable tracks are omitted. Track references
  do not preserve alternate names, mute settings, colours or heights.

For a source with zero or one arrangement, the book remains at its legacy
empty default. Its playlist, allocator, timeline and report follow the
existing import path.

## Playback and reported limits

The selected arrangement still supplies the one audible playlist, including
its existing clip conversion and named/meter marker conversion. Arrangement
metadata is added afterward; no second clip payload store or alternate clip
entities are created. Switching the book's active ID changes saved metadata
only, and does not change clips, track settings, markers, meter or playback.

Names and IDs survive even when an alternate layout has no matching clips or
tracks. The report explains the single-playlist playback limit and separately
counts alternate items without matching clip references and alternate timeline
markers as dropped. Unsupported native plugin state continues through the
existing silent-placeholder and retained-state path; import does not execute it.

This implements the import seam for `win-playlist-arrangements`. It does not
certify alternate-layout playback parity. Windfall's GPL-3.0 license applies;
no proprietary code, assets or new branded type/UI names are introduced.

## Verification

Library unit tests in `convert/arrangements/tests.rs` use original synthetic
projects with two named arrangements. They check deterministic IDs, references
to existing entities, selected playlist/timeline equality with a single-source
baseline, inert switching, selection fallback, distinct layouts, repeated and
legacy placements, rejected clips, name limits and silent unsupported plugin
state retention. Import undo is not required.

Run from Git Bash at the repository root:

```bash
source scripts/msvc-env.sh
cargo test -p windfall-flp --lib
```

Verified in Git Bash after sourcing the MSVC environment: **78 library tests
passed**, including all six arrangement import tests. Rustfmt checks passed for
the changed Rust files.
