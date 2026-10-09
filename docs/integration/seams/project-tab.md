# Project browser tab

The Windfall desktop browser offers Library and Project tabs. Library remains
the existing disk-library browser. Project is a read-only inventory of the open
document. It displays Patterns, Channels, Samples, Mixer effects grouped by
mixer track, Automations, Plugins, Retained plugin states, and Edit history.
Patterns is always shown; other groups are omitted when their arrays are empty
or absent. Tracks without effect slots are omitted from Mixer effects.

`features/browser/project-tab/ProjectTab` subscribes to `useProjectStore` in
`src/lib/store/project.ts`. It reads these fields:

- `project.patterns`, `project.channels`, `project.samples`, and
  `project.automations`: object names in document order.
- `project.mixer.tracks`: track names and each track's `effects` (`EffectSlot`)
  in signal-chain order. Slot names come from the existing effect descriptor,
  or the matching effect plugin binding's name. Disabled slots show Bypassed.
- Optional `project.plugins`: binding names, including unavailable instances.
- Optional `project.retainedPlugins`: `name`, falling back to `internalName`.
- `history.entries` and `history.cursor` on the store, separate from `Project`:
  entry labels are shown oldest first. Indices below the cursor show Applied;
  indices at or past the cursor show Undone with an outline badge and muted text.

Store updates refresh the inventory and history. Rows have no command handlers,
drag behavior, selection behavior, or editor navigation. The view does not
dispatch commands, change other editors' selections, or write the project.
The library context menu and browser shortcut scope stay inside Library.

Switching tabs remounts Library from its existing browser and library stores;
the filter, tags, favorites, expanded folders, listings, selection, and saved
tree scroll position are retained. No library store reset is introduced.

Focused verification from `apps/desktop`:

`pnpm test src/features/browser/project-tab/project-tab.test.tsx`

Awaiting QA. No parity tracker completion claim is made.
