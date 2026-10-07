import { useShallow } from "zustand/react/shallow"

import type {
  Clip,
  Pattern,
  PatternId,
  PlaylistTrack,
  Project,
} from "@/bindings"
import { useProjectStore } from "@/lib/store/project"
import { selectedPatternId } from "@/lib/store/selectors"
import { useTransportStore } from "@/lib/store/transport"

import { usePlaylistStore } from "./store"

/* Reads of the project the playlist needs, as hooks and as plain calls. */

export const project = (): Project => useProjectStore.getState().project
export const playlist = () => project().playlist

export function usePlaylistTracks(): PlaylistTrack[] {
  return useProjectStore((state) => state.project.playlist.tracks)
}

export function useClipCount(): number {
  return useProjectStore((state) => state.project.playlist.clips.length)
}

/** How many clips play each pattern, in the order of `ids`. */
export function useClipCounts(ids: readonly PatternId[]): number[] {
  return useProjectStore(
    useShallow((state) => {
      const counts = new Map<PatternId, number>()
      for (const clip of state.project.playlist.clips) {
        const pattern = clip.content.pattern
        counts.set(pattern, (counts.get(pattern) ?? 0) + 1)
      }
      return ids.map((id) => counts.get(id) ?? 0)
    })
  )
}

/** The pattern the Draw and Paint tools place: the one selected app-wide. */
export function brushPattern(): Pattern | undefined {
  const current = project()
  const id = selectedPatternId(current, useTransportStore.getState().pattern)
  return current.patterns.find((pattern) => pattern.id === id)
}

/** The selected clips that still exist, in timeline order. */
export function selectedClips(): Clip[] {
  const { selection } = usePlaylistStore.getState()
  if (selection.size === 0) return []
  return playlist().clips.filter((clip) => selection.has(clip.id))
}

export function useSelectionCount(): number {
  return usePlaylistStore((state) => state.selection.size)
}
