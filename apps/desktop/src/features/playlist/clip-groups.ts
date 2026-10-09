import type {
  Clip,
  ClipGroup,
  ClipId,
  ClipUpdate,
  PlaylistTrack,
} from "@/bindings"

import { spanFits } from "./edit"

/** Original selection first, then touched groups in saved member order. */
export function expandClipSelection(
  selected: Iterable<ClipId>,
  groups: readonly ClipGroup[]
): ClipId[] {
  const original = new Set(selected)
  const expanded = new Set(original)
  for (const group of groups) {
    if (group.clips.some((id) => original.has(id))) {
      for (const id of group.clips) expanded.add(id)
    }
  }
  return [...expanded]
}

/** A shared delta in document track order; refuse the entire move on failure. */
export function clipGroupMove(
  clips: readonly Clip[],
  tracks: readonly PlaylistTrack[],
  ticks: number,
  rows: number
): ClipUpdate[] | null {
  const indices = new Map(tracks.map((track, index) => [track.id, index]))
  const updates: ClipUpdate[] = []
  for (const clip of clips) {
    const index = indices.get(clip.track)
    const track = index === undefined ? undefined : tracks[index + rows]
    const start = clip.start + ticks
    if (!track || clip.length === 0 || !spanFits(start, clip.length))
      return null
    updates.push({ id: clip.id, patch: { start, track: track.id } })
  }
  return updates
}
