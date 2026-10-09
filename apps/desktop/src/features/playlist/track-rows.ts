import { useMemo } from "react"

import type {
  Playlist,
  PlaylistTrack,
  PlaylistTrackId,
  TrackGroup,
} from "@/bindings"
import { useProjectStore } from "@/lib/store/project"

import { arrangementPlaylist } from "./arrangement-view"
import { playlist } from "./selectors"
import { usePlaylistStore } from "./store"

export type TrackRow =
  | { kind: "track"; track: PlaylistTrack; depth: number; index: number }
  | {
      kind: "group"
      group: TrackGroup
      depth: number
      members: readonly PlaylistTrack[]
    }

/** One projection shared by the headers, clip canvas and edit destinations. */
export function buildTrackRows(
  current: Playlist,
  collapsed: ReadonlySet<number>
) {
  const document = current
  current = arrangementPlaylist(current)
  const book = current.arrangementBook
  const groups = book?.trackGroups ?? []
  const byId = new Map(groups.map((group) => [group.id, group]))
  const children = new Map<number, TrackGroup[]>()
  const roots: TrackGroup[] = []
  for (const group of groups) {
    const parent = book?.groupParents[group.id]
    if (parent === undefined || !byId.has(parent)) roots.push(group)
    else {
      const siblings = children.get(parent) ?? []
      siblings.push(group)
      children.set(parent, siblings)
    }
  }
  const members = new Map<number, PlaylistTrack[]>()
  // Walk upward for each track so every ancestor includes nested members.
  current.tracks.forEach((track) => {
    const visited = new Set<number>()
    let parent = book?.trackParents[track.id]
    while (parent !== undefined && byId.has(parent) && !visited.has(parent)) {
      visited.add(parent)
      const descendants = members.get(parent) ?? []
      descendants.push(track)
      members.set(parent, descendants)
      parent = book?.groupParents[parent]
    }
  })
  const indices = new Map(
    document.tracks.map((track, index) => [track.id, index])
  )
  // An arrangement's order takes precedence over grouping. Emit each ancestor
  // at its first member, keeping collapse and document edit destinations intact.
  if (current !== document) {
    const rows: TrackRow[] = []
    const shown = new Set<number>()
    for (const track of current.tracks) {
      const ancestors: TrackGroup[] = []
      const visited = new Set<number>()
      let parent = book?.trackParents[track.id]
      while (parent !== undefined && byId.has(parent) && !visited.has(parent)) {
        visited.add(parent)
        ancestors.unshift(byId.get(parent)!)
        parent = book?.groupParents[parent]
      }
      let hidden = false
      for (const [depth, group] of ancestors.entries()) {
        if (!shown.has(group.id)) {
          shown.add(group.id)
          rows.push({
            kind: "group",
            group,
            depth,
            members: members.get(group.id) ?? [],
          })
        }
        if (collapsed.has(group.id)) {
          hidden = true
          break
        }
      }
      if (!hidden)
        rows.push({
          kind: "track",
          track,
          depth: ancestors.length,
          index: indices.get(track.id)!,
        })
    }
    const trackRows = new Map<PlaylistTrackId, number>()
    rows.forEach((row, index) => {
      if (row.kind === "track") trackRows.set(row.track.id, index)
    })
    return { rows, trackRows }
  }
  const anchor = (group: TrackGroup) => {
    const first = members.get(group.id)?.[0]
    return first ? indices.get(first.id)! : Infinity
  }
  const rows: TrackRow[] = []
  const visited = new Set<number>()
  const append = (
    parent: number | undefined,
    depth: number,
    siblings: TrackGroup[]
  ) => {
    const entries: { at: number; row: TrackRow }[] = siblings.map((group) => ({
      at: anchor(group),
      row: {
        kind: "group",
        group,
        depth,
        members: members.get(group.id) ?? [],
      },
    }))
    current.tracks.forEach((track, index) => {
      const owner = book?.trackParents[track.id]
      if (
        owner === parent ||
        (parent === undefined && (owner === undefined || !byId.has(owner)))
      ) {
        entries.push({ at: index, row: { kind: "track", track, depth, index } })
      }
    })
    entries.sort((a, b) => a.at - b.at)
    for (const { row } of entries) {
      if (row.kind === "track") rows.push(row)
      else if (!visited.has(row.group.id)) {
        visited.add(row.group.id)
        rows.push(row)
        if (!collapsed.has(row.group.id))
          append(row.group.id, depth + 1, children.get(row.group.id) ?? [])
      }
    }
  }
  append(undefined, 0, roots)
  const trackRows = new Map<PlaylistTrackId, number>()
  rows.forEach((row, index) => {
    if (row.kind === "track") trackRows.set(row.track.id, index)
  })
  return { rows, trackRows }
}

export const trackRowsNow = () =>
  buildTrackRows(playlist(), usePlaylistStore.getState().collapsedGroups)

export function useTrackRows() {
  const tracks = useProjectStore((state) => state.project.playlist.tracks)
  const arrangementBook = useProjectStore(
    (state) => state.project.playlist.arrangementBook
  )
  const collapsed = usePlaylistStore((state) => state.collapsedGroups)
  return useMemo(
    () => buildTrackRows({ tracks, clips: [], arrangementBook }, collapsed),
    [tracks, arrangementBook, collapsed]
  )
}

/** Group headers are not clip destinations; spare rows still create tracks. */
export function documentRowFor(
  row: number,
  layout = trackRowsNow()
): number | undefined {
  const { rows } = layout
  if (row >= rows.length) return playlist().tracks.length + row - rows.length
  const entry = rows[row]
  return entry?.kind === "track" ? entry.index : undefined
}
