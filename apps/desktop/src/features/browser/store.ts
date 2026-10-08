import { create } from "zustand"

import type {
  BrowserEntry,
  BrowserEntryKind,
  BrowserRoot,
  SampleInfo,
  LibraryFileToken,
} from "@/bindings"
import { backend, errorMessage } from "@/lib/ipc"
import {
  invalidateLibrary,
  resetLibraryStore,
  useLibraryStore,
} from "./library-store"

import {
  dropPendingScrollTop,
  readPersisted,
  saveScrollTop,
  writePersisted,
} from "./persist"
import {
  isUnder,
  loadedEntries,
  parseRowId,
  pruneExpanded,
  rowId,
  type EntryRow,
  type Listing,
} from "./tree-model"

/** The row the user is on. Kept as facts, so actions need not find the row. */
export type Selection = {
  id: string
  path: string
  name: string
  kind: BrowserEntryKind
  /** Path of the folder it is in. Null for a top-level folder. */
  parentPath: string | null
  /** Set when it is a top-level folder. */
  root: BrowserRoot | null
  library?: LibraryFileToken
  /** Immutable even when the token lookup has not finished. */
  librarySelection?: Readonly<{ epoch: number; rootPath: string }>
}

export type InfoState =
  | { path: string; status: "loading" }
  | { path: string; status: "ready"; info: SampleInfo }
  | { path: string; status: "error"; message: string }

export type BrowserState = {
  rootsStatus: "idle" | "loading" | "ready" | "error"
  rootsError: string | null
  /** Factory content first, then the folders the user added. */
  roots: BrowserRoot[]
  /** What each folder holds, by path. Read once, then kept. */
  listings: Record<string, Listing>
  /** Row ids of open folders. */
  expanded: ReadonlySet<string>
  selected: Selection | null
  filter: string
  autoPreview: boolean
  /** True while the folders that were open last time are being read again. */
  restoring: boolean
  /** Facts and waveform of the sound the preview pane is showing. */
  info: InfoState | null
  /** The sound the engine was last told to play, and when it started. */
  playing: { path: string; startedAt: number } | null
  previewError: { path: string; message: string } | null
  /**
   * Requests to the components, which clear them once done. They wait here
   * while the panel is hidden, so "filter the browser" works from anywhere.
   */
  focusFilter: boolean
  focusTree: boolean
  /** Id of a row the tree should scroll into view. */
  reveal: string | null
}

// The panel can be hidden and shown again, so its state lives here and not
// in the components. `generation` lets a reset ignore calls still under way.
let generation = 0
let pendingLoads = 0
let firstRun = true
let hydrating = false
let scrollTop = 0
const loadTokens = new Map<string, number>()

function initialState(): BrowserState {
  const saved = readPersisted()
  firstRun = saved.expanded === null
  scrollTop = saved.scrollTop
  return {
    rootsStatus: "idle",
    rootsError: null,
    roots: [],
    listings: {},
    expanded: new Set(saved.expanded ?? []),
    selected: null,
    filter: "",
    autoPreview: saved.autoPreview,
    restoring: true,
    info: null,
    playing: null,
    previewError: null,
    focusFilter: false,
    focusTree: false,
    reveal: null,
  }
}

export const useBrowserStore = create<BrowserState>(initialState)

useBrowserStore.subscribe((state, previous) => {
  if (hydrating) return
  if (state.expanded !== previous.expanded) {
    writePersisted({ expanded: [...state.expanded] })
  }
  if (state.autoPreview !== previous.autoPreview) {
    writePersisted({ autoPreview: state.autoPreview })
  }
})

const get = useBrowserStore.getState
const set = useBrowserStore.setState

/**
 * Starts over from what is saved, as a restart of the app does. Tests call
 * it between cases.
 */
export function resetBrowserStore() {
  generation += 1
  pendingLoads = 0
  resetLibraryStore()
  loadTokens.clear()
  dropPendingScrollTop()
  hydrating = true
  set(initialState(), true)
  hydrating = false
}

export function savedScrollTop(): number {
  return scrollTop
}

export function rememberScrollTop(top: number) {
  scrollTop = top
  saveScrollTop(top)
}

function finishRestoreWhenIdle() {
  const { restoring, rootsStatus } = get()
  const rootsSettled = rootsStatus === "ready" || rootsStatus === "error"
  if (restoring && rootsSettled && pendingLoads === 0) set({ restoring: false })
}

function openPaths(expanded: ReadonlySet<string>): Set<string> {
  const paths = new Set<string>()
  for (const id of expanded) {
    const path = parseRowId(id)?.path
    if (path !== undefined) paths.add(path)
  }
  return paths
}

function applyListing(path: string, entries: BrowserEntry[], reload: boolean) {
  const state = get()
  const selected = state.selected
  const selectionGone =
    selected !== null &&
    selected.parentPath === path &&
    !entries.some((entry) => entry.path === selected.path)
  set({
    listings: { ...state.listings, [path]: { status: "ready", entries } },
    // Folders that were open last time but are gone now are dropped quietly.
    expanded: pruneExpanded(state.expanded, path, entries),
    ...(selectionGone ? { selected: null, info: null } : {}),
  })

  // Open folders inside this one are read next. This is how the folders that
  // were open last time come back, one level after another.
  const open = openPaths(get().expanded)
  for (const entry of entries) {
    if (entry.kind !== "folder" || !open.has(entry.path)) continue
    if (reload || get().listings[entry.path] === undefined) {
      void loadFolder(entry.path, { reload })
    }
  }
}

/**
 * Reads a folder. What was listed before stays on screen until the new
 * listing arrives. When the same folder is read twice at once, only the
 * later answer counts, in whichever order the two arrive.
 */
export async function loadFolder(
  path: string,
  options: { reload?: boolean } = {}
): Promise<void> {
  const mine = generation
  const token = (loadTokens.get(path) ?? 0) + 1
  loadTokens.set(path, token)
  set((state) => ({
    listings: {
      ...state.listings,
      [path]: {
        status: "loading",
        entries: loadedEntries(state.listings[path]),
      },
    },
  }))

  pendingLoads += 1
  let entries: BrowserEntry[] | null = null
  let failure = ""
  try {
    entries = await backend.browserList(path)
  } catch (error) {
    failure = errorMessage(error)
  }
  if (mine !== generation) return
  pendingLoads -= 1

  if (loadTokens.get(path) === token) {
    if (entries === null) {
      set((state) => ({
        listings: {
          ...state.listings,
          [path]: { status: "error", message: failure },
        },
      }))
    } else {
      applyListing(path, entries, options.reload ?? false)
    }
  }
  finishRestoreWhenIdle()
}

/** Takes a new list of roots from the backend and tidies up after it. */
export function applyRoots(roots: BrowserRoot[]) {
  const ordered = [
    ...roots.filter((root) => root.kind === "factory"),
    ...roots.filter((root) => root.kind !== "factory"),
  ]
  const state = get()
  const alive = new Set(ordered.map((root) => root.path))

  let expanded: ReadonlySet<string> = state.expanded
  if (firstRun) {
    // A new user sees the factory sounds without having to find the arrow.
    firstRun = false
    expanded = new Set(
      ordered
        .filter((root) => root.kind === "factory")
        .map((root) => rowId(root.path, root.path))
    )
  } else {
    const kept = [...expanded].filter((id) => {
      const root = parseRowId(id)?.root
      return root !== undefined && alive.has(root)
    })
    if (kept.length !== expanded.size) expanded = new Set(kept)
  }

  const selectedRoot =
    state.selected === null ? undefined : parseRowId(state.selected.id)?.root
  const selectionGone = selectedRoot !== undefined && !alive.has(selectedRoot)

  set({
    rootsStatus: "ready",
    rootsError: null,
    roots: ordered,
    expanded,
    ...(selectionGone ? { selected: null, info: null } : {}),
  })

  for (const root of ordered) {
    const open = expanded.has(rowId(root.path, root.path))
    if (open && get().listings[root.path] === undefined) {
      void loadFolder(root.path)
    }
  }
  invalidateLibrary()
  finishRestoreWhenIdle()
}

export async function loadRoots(): Promise<void> {
  const mine = generation
  set({ rootsStatus: "loading", rootsError: null })
  try {
    const roots = await backend.browserRoots()
    if (mine === generation) applyRoots(roots)
  } catch (error) {
    if (mine !== generation) return
    set({ rootsStatus: "error", rootsError: errorMessage(error) })
    finishRestoreWhenIdle()
  }
}

/** Reads the roots the first time the panel shows. */
export function ensureRootsLoaded() {
  if (get().rootsStatus === "idle") void loadRoots()
}

export function expandFolder(id: string, path: string) {
  const state = get()
  if (!state.expanded.has(id)) {
    set({ expanded: new Set(state.expanded).add(id) })
  }
  const listing = get().listings[path]
  if (listing === undefined || listing.status === "error") void loadFolder(path)
}

export function collapseFolder(id: string) {
  const state = get()
  if (!state.expanded.has(id)) return
  const expanded = new Set(state.expanded)
  expanded.delete(id)
  set({ expanded })
}

export function toggleFolder(row: EntryRow) {
  if (row.open && !row.forced) collapseFolder(row.id)
  else expandFolder(row.id, row.path)
}

/**
 * Reads a folder again, along with the open folders inside it. Closed
 * folders inside it are forgotten, so they are read when next opened.
 */
export function refreshFolder(path: string) {
  const state = get()
  const open = openPaths(state.expanded)
  const listings: Record<string, Listing> = {}
  for (const [key, listing] of Object.entries(state.listings)) {
    if (!isUnder(key, path) || open.has(key)) listings[key] = listing
  }
  set({ listings })
  void loadFolder(path, { reload: true })
}

export function setFilter(filter: string) {
  set({ filter })
  invalidateLibrary({ selections: false })
}

export function setAutoPreview(autoPreview: boolean) {
  set({ autoPreview })
}

export function requestFilterFocus() {
  set({ focusFilter: true, focusTree: false })
}

export function requestTreeFocus() {
  set({ focusTree: true, focusFilter: false })
}

export function revealRow(id: string) {
  set({ reveal: id })
}

/** The components call this once they have done what was asked. */
export function requestDone(request: "focusFilter" | "focusTree" | "reveal") {
  set(request === "reveal" ? { reveal: null } : { [request]: false })
}

export function selectionOf(row: EntryRow): Selection {
  const results = useLibraryStore.getState().results
  const indexed = results?.entries.find(
    (held) =>
      held.entry.path === row.path &&
      held.token.rootPath === parseRowId(row.id)?.root
  )?.token
  const library =
    row.library ??
    indexed ??
    (results && row.kind !== "folder"
      ? {
          path: row.path,
          rootPath: parseRowId(row.id)?.root ?? "",
          generation: results.generation,
          fingerprint: "",
        }
      : undefined)
  return {
    id: row.id,
    path: row.path,
    name: row.name,
    kind: row.kind,
    parentPath: row.parentPath,
    root: row.root,
    library,
    librarySelection:
      row.kind === "folder"
        ? undefined
        : Object.freeze({
            epoch: useLibraryStore.getState().epoch,
            rootPath: parseRowId(row.id)?.root ?? "",
          }),
  }
}
