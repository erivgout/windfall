import { create } from "zustand"

import type { LibraryMetadata, LibraryResults } from "@/bindings"
import { backend } from "@/lib/ipc"

type LibraryState = {
  favoritesOnly: boolean
  tags: string[]
  results: LibraryResults | null
  error: string | null
  pending: boolean
  revision: number
  /** Selection identity; query/metadata revisions do not change it. */
  epoch: number
}

export const useLibraryStore = create<LibraryState>(() => ({
  favoritesOnly: false,
  tags: [],
  results: null,
  error: null,
  pending: true,
  revision: 0,
  epoch: 0,
}))

export function resetLibraryStore() {
  const epoch = useLibraryStore.getState().epoch + 1
  useLibraryStore.setState({
    favoritesOnly: false,
    tags: [],
    results: null,
    error: null,
    pending: true,
    revision: 0,
    epoch,
  })
}

export function invalidateLibrary({ selections = true } = {}) {
  useLibraryStore.setState((s) => ({
    results: null,
    error: null,
    pending: true,
    revision: s.revision + 1,
    epoch: s.epoch + (selections ? 1 : 0),
  }))
}

export async function refreshLibrary() {
  // Invalidate before IPC can yield: no lookup may bridge this refresh.
  invalidateLibrary()
  await backend.libraryRefresh()
  // Selections made while native refresh was pending must also be reselected.
  invalidateLibrary()
}

export async function saveLibraryMetadata(
  path: string,
  metadata: LibraryMetadata
) {
  const saved = await backend.librarySetMetadata(path, metadata)
  useLibraryStore.setState((s) => ({ revision: s.revision + 1 }))
  return saved
}
