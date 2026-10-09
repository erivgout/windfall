# Windfall playlist automation Step drawing

The playlist toolbar's **Step** toggle uses **H**, which is free in the
playlist keymap. It is session state in `usePlaylistStore`, excluded from
persisted preferences and the project. Opening another project turns it off
and cancels any held stroke preview.

Step changes only the existing `add-point` intent on an automation clip's
curve body in Draw, Paint, or Select. With Step off, adding a single ordinary
point and dragging points behave as before. Existing point presses still
move the point selection; multi-point moves and the LFO dialog are unchanged.
Bend handles, clip edges, Slice, Slip, and solo retain their existing paths.

The session captures the snap spacing and curve view at the press. Each
pointer sample snaps to the song grid, maps through the clip's start and
offset, and clamps to its curve window. Alt at the press or Snap None uses
individual integer ticks. Fast and backwards strokes fill every crossed grid
tick, interpolating the pointer values between samples; revisiting a tick
updates its value. An existing tick updates its last point rather than adding
a duplicate, preserving existing jumps. Written points have a straight bend
and `hold: true`, except the last point of the whole curve, whose hold is
false. A stroke ending before a later curve point still holds its final
written point.

`automation/step.ts` performs the point writes without changing the input
curve. `PlaylistSession` owns the preview and uses the existing `setCurve`
seam on release to send one `setAutomationPoints` command and make one undo
step. Escape or pointer cancellation discards the preview and sends nothing,
including on a later release. A curve already at 4,096 points refuses the
gesture with the same message as adding a point. If a stroke would exceed
capacity, the entire preview is cancelled and no partial edit is dispatched.
The project model and engine are unchanged.

Run from `apps/desktop`:

```sh
pnpm test src/features/playlist/automation/step.test.tsx
pnpm test src/features/playlist/automation/point-selection.test.ts
```

The Step tests cover a held stroke committed in one command, existing-tick
updates, final-point holds, backwards and returning strokes, ordinary point
insertion with Step off, cancellation and Escape, full curves and capacity
overflow, clip offsets and window clamps, selected-point moves with Step on,
the toolbar and H shortcut, and reset on project replacement. The existing
point-selection suite checks the unchanged multi-point gesture behavior.
