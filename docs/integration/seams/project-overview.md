# Project overview

Implementation for `win-browser-project-picker`, awaiting QA. Windfall exposes
**Show project overview** (`view.projectOverview`) in the View action section.
The action opens a read-only dialog alongside the existing Project browser tab.

## Ownership and integration

`apps/desktop/src/features/project-overview/` owns the action, Zustand open flag,
dialog and view. `registerProjectOverviewActions` is registered and cleaned up
alongside `registerSpectrumActions` in layout `register-actions.ts`.
`ProjectOverviewPanel` mounts in the shared shell overlays in `app-shell.tsx`.
Closing the dialog or unregistering the action unmounts the overview view.

## Document relationships

The view subscribes directly to `useProjectStore`'s project. Rows follow
`project.channels` rack order; columns follow `project.patterns` display order.
A cell is marked only when that pattern has a lane whose `channel` matches the
row's channel ID and whose `notes` array is nonempty. Empty and missing lanes
are unmarked. Each cell has an accessible Has notes / No notes label.

The playlist list is sorted by clip `start`, then `id`, without mutating the
store's clip array. Each row resolves its playlist track name by ID and its
`ClipContent` target by type: `pattern` resolves a pattern, `audio` resolves a
sample, and `automation` resolves an automation. Missing references explicitly
say Missing pattern/sample/automation target; a missing track is also labeled.
Start ticks are shown to make the timeline order visible.

Names, cells and clip rows update with document patches or replacement snapshots.
The feature imports no command dispatch or editor selection APIs and adds no
document fields. It provides inspection only, with no navigation or editing of
other editors. Browser code and parity status are unchanged.

## Focused validation

From `apps/desktop`:

```sh
pnpm test -- src/features/project-overview/view.test.tsx src/features/project-overview/panel.test.tsx
```

This focused command passed: two test files, ten tests.

Tests cover a note in only the second pattern, an empty lane, missing lanes,
rack/column order, a clip referencing the second pattern, sample and automation
names, timeline ordering and ID ties, all three missing-target kinds, live store
updates, read-only interactions, action opening, closing and unregister cleanup.
Integrated desktop QA remains pending; this seam does not mark parity complete.
