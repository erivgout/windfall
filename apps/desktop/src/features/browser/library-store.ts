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
}

export const useLibraryStore = create<LibraryState>(() => ({
  favoritesOnly: false,
  tags: [],
  results: null,
  error: null,
  pending: true,
  revision: 0,
}))

export function resetLibraryStore() {
  useLibraryStore.setState({
    favoritesOnly: false,
    tags: [],
    results: null,
    error: null,
    pending: true,
    revision: 0,
  })
}

export function invalidateLibrary() {
  useLibraryStore.setState((s) => ({
    results: null,
    error: null,
    pending: true,
    revision: s.revision + 1,
  }))
}

export async function refreshLibrary() {
  await backend.libraryRefresh()
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
