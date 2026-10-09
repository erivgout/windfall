# Windfall playlist automation point selection

Point selection lives in the playlist session and belongs to one automation
clip. It is not saved in the project. Opening another project clears the
selection and cancels any held curve preview. Clips sharing an automation
still share its saved curve, but only the owning clip highlights its selected
points. Selected markers are filled with the curve's ink; other markers stay
hollow.

A plain click selects only the pressed point. Shift-click toggles that point;
pressing a point in another clip replaces the selection, including with Shift.
A plain press on a selected point preserves the selection for dragging and
collapses it only on a click without a drag. The existing point menu continues
to act on the pressed point only. Double-click hold and Shift axis locking
during a drag retain their existing behavior.

Dragging a selected point applies the same tick and value delta to every
selected point. The pressed point uses the existing playlist snap, including
Alt and Snap None. The shared tick delta is clamped to the clip window and to
one tick before or after unselected neighbours, so moving points cannot
collide or pass them. The shared value delta is clamped to keep every selected
value between 0 and 1. Point indices, curve bends, and hold flags retain their
order and shape.

The session previews the curve without changing the project. Release sends
one existing `setAutomationPoints` command through `setCurve`, producing one
undo step. A move clamped to no change sends nothing. Escape and pointer
cancel use the existing session cancellation path, discard the preview, and
dispatch nothing. Inserting or deleting a point clears stale selection
indices. The LFO write dialog is unchanged.

Run the focused test from `apps/desktop`:

```sh
pnpm test src/features/playlist/automation/point-selection.test.ts
```

It covers selection toggling, plain clicks, one-command grouped moves,
snapping, contiguous and nonadjacent order clamps, shared value limits, no-op
moves, cancellation, switching automations, project replacement, point-menu
scope, selection ownership for shared curves, existing jumps, and selected
marker painting.
