# Scale highlight

The piano-roll toolbar adds one **Highlight** menu. Its choices are **Off**,
**Major**, **Natural minor**, **Harmonic minor** and **Pentatonic major**.
**Highlight root** offers the twelve pitch classes C through B. It starts at
Off with root C and resets when the open piano roll unmounts.

## UI and rendering boundary

`apps/desktop/src/features/piano-roll/scale/control.tsx` owns the choice in React
state. It does not write the project, the persisted piano-roll preferences or
the existing pitch-snapping settings. The toolbar only imports and renders
`ScaleHighlightControl` beside the chord tools.

`model.ts` defines the intervals and `isInScale(key, choice)`, accepting MIDI
keys or pitch classes. Off reports every pitch as in-scale. The definitions
are independent of the existing scale stamping and snapping policies.

`highlight.ts` uses `PianoRollSession.onView` and
`TimeGridView.addOverlayPainter` to follow canvas attachment/replacement. A
translucent black veil dims only out-of-scale rows, using the current viewport
and device-pixel transform. It adds no pointer surface. Notes on shaded rows
retain 82% of their original brightness and remain visible and editable; their
scene, hit testing and edit commands are untouched. Selection marquees and the
playhead render above the veil. Off installs no painter, so the existing grid
is unchanged, including any shading from the older Scale settings menu.

Changing a choice removes the previous painter. Off and unmount remove the
painter and view subscription. Scrolling, zooming, theme changes and canvas
resizing use the existing overlay invalidation path. Existing Scale settings
continue to operate independently; this control does not quantize notes,
delete, move or filter them, or print notation.

## Validation

From `apps/desktop`:

```powershell
pnpm test src/features/piano-roll/scale
```

Focused tests cover the required pitch-class memberships, harmonic minor,
octave repetition, device alignment, Off drawing nothing, accessible choices
and roots, unchanged project/preferences/history, editing and drawing outside
the scale, state reset on unmount, and painter cleanup/view replacement.
