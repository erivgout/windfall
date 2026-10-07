import { toast } from "sonner"

import type { BrowserRoot, Channel, DispatchResult } from "@/bindings"
import { attempt, reportError } from "@/lib/errors"
import { openProjectPath } from "@/lib/flows/project"
import { backend } from "@/lib/ipc"
import { receivePatch, useProjectStore } from "@/lib/store/project"
import { useUiStore } from "@/lib/store/ui"

import {
  clearInfo,
  forgetInfoUnder,
  requestInfo,
  requestPreview,
} from "./preview"
import {
  applyRoots,
  expandFolder,
  loadRoots,
  refreshFolder,
  revealRow,
  selectionOf,
  setFilter,
  toggleFolder,
  useBrowserStore,
  type Selection,
} from "./store"
import { rowId, type EntryRow } from "./tree-model"

const get = useBrowserStore.getState
const set = useBrowserStore.setState

type SelectOptions = {
  /** Play the sound, when auto-preview is on. */
  preview?: boolean
  /** The selection is moving under a held key; wait for it to rest. */
  settle?: boolean
}

/**
 * Moves the selection to a row. This is the heart of auditioning: landing on
 * a sound shows it in the pane and, with auto-preview on, plays it.
 */
export function selectRow(row: EntryRow, options: SelectOptions = {}) {
  set({ selected: selectionOf(row) })
  if (row.kind !== "audio") {
    clearInfo()
    return
  }
  requestInfo(row.path, { settle: options.settle })
  if (options.preview && get().autoPreview) {
    requestPreview(row.path, { settle: options.settle })
  }
}

/** The selected row when it is a sound, otherwise null. */
export function selectedSound(): Selection | null {
  const selected = get().selected
  return selected?.kind === "audio" ? selected : null
}

export function selectedChannel(): Channel | null {
  const id = useUiStore.getState().selectedChannel
  if (id === null) return null
  return (
    useProjectStore
      .getState()
      .project.channels.find((channel) => channel.id === id) ?? null
  )
}

function applyResult(result: DispatchResult) {
  // The same patch also arrives as an event; the store ignores the repeat.
  receivePatch(result.patch)
}

/** Adds a new channel that plays the file, and selects it in the rack. */
export async function addToRack(path: string): Promise<void> {
  const result = await attempt(
    backend.addChannelFromFile(path),
    "Could not add the sound"
  )
  if (!result) return
  applyResult(result)
  const channels = useProjectStore.getState().project.channels
  const created = result.created.find((id) =>
    channels.some((channel) => channel.id === id)
  )
  const ui = useUiStore.getState()
  if (created !== undefined) ui.selectChannel(created)
  ui.showCenterTab("channelRack")
}

/** Makes the channel selected in the rack play the file instead. */
export async function replaceChannelSample(path: string): Promise<void> {
  const channel = selectedChannel()
  if (!channel) return
  const result = await attempt(
    backend.setChannelSampleFromFile(channel.id, path),
    `Could not replace the sample of ${channel.name}`
  )
  if (result) applyResult(result)
}

/** What Enter and a double-click do. Folders open on a single click instead. */
export function activateRow(row: EntryRow) {
  if (row.kind === "audio") void addToRack(row.path)
  else if (row.kind === "project") void openProjectPath(row.path)
  else if (row.kind === "folder") toggleFolder(row)
}

export function refresh(path: string) {
  forgetInfoUnder(path)
  refreshFolder(path)
}

/**
 * Refreshes the selected folder, or the folder the selected file is in. With
 * nothing selected it reads the roots and every open root again.
 */
export function refreshSelection() {
  const selected = get().selected
  if (selected === null) {
    void loadRoots().then(() => {
      const { roots, expanded } = get()
      for (const root of roots) {
        if (expanded.has(rowId(root.path, root.path))) refresh(root.path)
      }
    })
    return
  }
  const path = selected.kind === "folder" ? selected.path : selected.parentPath
  if (path !== null) refresh(path)
}

/** Asks for a folder and adds it to the browser, open and selected. */
export async function addFolder(): Promise<void> {
  let path: string | null
  try {
    path = await backend.pickFolder()
  } catch (error) {
    reportError(error, "Could not choose a folder")
    return
  }
  if (path === null) return

  let roots: BrowserRoot[]
  try {
    roots = await backend.browserAddRoot(path)
  } catch (error) {
    reportError(error, "Could not add the folder")
    return
  }

  const known = new Set(get().roots.map((root) => root.path))
  applyRoots(roots)
  const added = roots.find((root) => !known.has(root.path))
  if (!added) return

  const id = rowId(added.path, added.path)
  // A filter would hide the folder that was just added.
  setFilter("")
  expandFolder(id, added.path)
  // It may have been in the browser before, with a listing that is now old.
  refresh(added.path)
  set({
    selected: {
      id,
      path: added.path,
      name: added.name,
      kind: "folder",
      parentPath: null,
      root: added,
    },
  })
  clearInfo()
  revealRow(id)
}

/** Takes a folder the user added out of the browser. Its files stay on disk. */
export async function removeRoot(root: BrowserRoot): Promise<void> {
  if (root.kind === "factory") return
  try {
    applyRoots(await backend.browserRemoveRoot(root.path))
    if (get().selected === null) clearInfo()
  } catch (error) {
    reportError(error, `Could not remove ${root.name}`)
  }
}

export async function copyPath(path: string): Promise<void> {
  await navigator.clipboard.writeText(path)
  toast.success("Path copied", { description: path })
}
