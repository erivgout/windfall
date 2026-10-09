# Active arrangement editing repair — 2026-10-09

> Final execution reconciliation: the combined tree passed the complete native workspace, strict checks, fresh artifact checks and final affected frontend checks. See [completion acceptance](2026-10-09-completion.md) for exact results and promoted IDs. Earlier pending statements below describe intermediate states; explicit remaining feature gates still apply.

Scope: newly placed playlist tracks and clips while an arrangement is active.
The layout filter already requires both clip membership and track membership.
Before this repair, `AddPlaylistTrack` and `AddClips` inserted project entities
without enrolling them in the active book, so ordinary additions and bounced
prints could be hidden and silent.

`lower.rs` now records active membership with an `Edit::ArrangementBook` in the
same transaction as each addition. Clip insertion also enrolls its existing
destination track. Duplicate track IDs are avoided; inactive layouts and legacy
empty books remain unchanged. Undo/redo covers entities and membership together,
and a failing batch rolls both back, including the allocator and history.

Frontend document deletion checks then exposed the complementary defect:
referenced clips/tracks could not be removed. Actual entity deletion now prunes
layout membership, clip groups that fall below two members, track parenting,
and invalid channel/sample links in the same history transaction. Explicitly
adding or setting nonexistent references remains an error. The former tests
that declared referenced deletion invalid were replaced with the editing
contract: delete succeeds, undo restores associations, and redo retires them.

Regression evidence: `arrangement_integration::additions_join_only_active_arrangement_and_survive_undo_redo`
adds an empty active and inactive layout, adds a track, places three clips on
two tracks (including the previously hidden existing track), checks exact
membership ordering and deduplication, and verifies the alternate stays empty.
Its helper checks each command's undo, redo, project validation and disk reload.
A subsequent invalid batch checks the complete snapshot is unchanged.
`removing_material_prunes_layout_and_group_references_with_undo` checks clip,
linked channel and playlist-track deletion, reference cleanup, retained group
containers, undo/redo, project validation and save/reload after each command.

Native execution: all **6 arrangement integration tests passed** in the
workspace runtime run, including active additions and deletion pruning.
Evidence: [native runtime log](2026-10-09-native-tests-all-runtime.log),
`Running tests/arrangement_integration.rs` and its `6 passed; 0 failed` result.
The targeted command is
`cargo test -p windfall-project --test arrangement_integration -j 1`.

Status eligibility: `win-playlist-arrangements` has this editing seam repaired,
but its full feature must remain partially verified. This work does not provide
independent clip positions per arrangement, grouped canvas gestures, linked
instrument/audio routing, or full interactive application acceptance.

The bounced-print regression is documented separately in
[playlist workflow QA](2026-10-09-playlist-workflow.md).
