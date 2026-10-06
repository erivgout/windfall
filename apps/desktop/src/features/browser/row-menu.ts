import type { Channel } from "@/bindings"
import {
  contextSeparator,
  type ContextItem,
} from "@/components/context-actions"
import { openProjectPath } from "@/lib/flows/project"

import {
  addToRack,
  copyPath,
  refresh,
  removeRoot,
  replaceChannelSample,
} from "./commands"
import { requestPreview } from "./preview"
import { toggleFolder } from "./store"
import type { EntryRow } from "./tree-model"

/**
 * What a right-click offers for a row, or for the empty space below the
 * rows when `row` is null.
 */
export function rowMenu(
  row: EntryRow | null,
  channel: Channel | null
): ContextItem[] {
  if (row === null) return ["browser.addFolder"]

  const copy: ContextItem = {
    title: "Copy path",
    run: () => copyPath(row.path),
  }

  if (row.kind === "audio") {
    return [
      { title: "Preview", run: () => requestPreview(row.path) },
      {
        title: "Add to new channel",
        shortcut: "Enter",
        run: () => addToRack(row.path),
      },
      {
        title: channel
          ? `Replace sample of ${channel.name}`
          : "Replace selected channel's sample",
        disabled: channel === null,
        run: () => replaceChannelSample(row.path),
      },
      contextSeparator,
      copy,
    ]
  }

  if (row.kind === "project") {
    return [
      {
        title: "Open project",
        shortcut: "Enter",
        run: () => openProjectPath(row.path),
      },
      contextSeparator,
      copy,
    ]
  }

  if (row.kind === "other") return [copy]

  const items: ContextItem[] = [
    {
      title: row.open ? "Collapse" : "Expand",
      // The filter is holding it open to show what it found inside.
      disabled: row.forced,
      run: () => toggleFolder(row),
    },
    { title: "Refresh", run: () => refresh(row.path) },
    contextSeparator,
    copy,
  ]
  const root = row.root
  if (root !== null) {
    items.push(contextSeparator, "browser.addFolder")
    if (root.kind === "user") {
      items.push({
        title: "Remove from browser",
        destructive: true,
        run: () => removeRoot(root),
      })
    }
  }
  return items
}
