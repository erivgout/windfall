import type { PlaylistTrackId } from "@/bindings"
import type { DeviceTransform, Viewport } from "@/lib/canvas"

import { playlist } from "./selectors"
import { usePlaylistStore } from "./store"
import { buildTrackRows, type TrackRow } from "./track-rows"

/** Visible track/group spans in CSS pixels; spare rows use the global height. */
export function buildRowGeometry(
  rows: readonly TrackRow[],
  globalHeight: number,
  overrides: ReadonlyMap<PlaylistTrackId, number>
) {
  const tops = [0]
  for (const row of rows) {
    const height =
      row.kind === "track"
        ? (overrides.get(row.track.id) ?? globalHeight)
        : globalHeight
    tops.push(tops[tops.length - 1] + height)
  }
  const top = (row: number): number => {
    if (row < 0) return row * globalHeight
    const index = Math.floor(row)
    if (index >= rows.length)
      return tops[rows.length] + (row - rows.length) * globalHeight
    return tops[index] + (row - index) * (tops[index + 1] - tops[index])
  }
  const rowAt = (y: number): number => {
    if (y < 0) return y / globalHeight
    if (y >= tops[rows.length])
      return rows.length + (y - tops[rows.length]) / globalHeight
    let lo = 0
    let hi = rows.length
    while (lo + 1 < hi) {
      const mid = Math.floor((lo + hi) / 2)
      if (tops[mid] <= y) lo = mid
      else hi = mid
    }
    return lo + (y - tops[lo]) / (tops[lo + 1] - tops[lo])
  }
  return { top, rowAt, height: (row: number) => top(row + 1) - top(row) }
}

let cached:
  | {
      playlist: ReturnType<typeof playlist>
      collapsed: ReadonlySet<number>
      overrides: ReadonlyMap<PlaylistTrackId, number>
      height: number
      geometry: ReturnType<typeof buildRowGeometry>
    }
  | undefined

export function rowGeometry(viewport: Pick<Viewport, "rowHeight">) {
  const current = playlist()
  const { collapsedGroups, trackHeights } = usePlaylistStore.getState()
  if (
    !cached ||
    cached.playlist !== current ||
    cached.collapsed !== collapsedGroups ||
    cached.overrides !== trackHeights ||
    cached.height !== viewport.rowHeight
  ) {
    cached = {
      playlist: current,
      collapsed: collapsedGroups,
      overrides: trackHeights,
      height: viewport.rowHeight,
      geometry: buildRowGeometry(
        buildTrackRows(current, collapsedGroups).rows,
        viewport.rowHeight,
        trackHeights
      ),
    }
  }
  return cached.geometry
}

/** scrollRow remains a physical offset measured in global-height units. */
export function playlistYToRow(viewport: Viewport, y: number) {
  const origin =
    Math.round(viewport.scrollRow * viewport.rowHeight * viewport.dpr) /
    viewport.dpr
  return rowGeometry(viewport).rowAt(origin + y)
}

export function playlistRowToY(viewport: Viewport, row: number) {
  return (
    (Math.round(rowGeometry(viewport).top(row) * viewport.dpr) -
      Math.round(viewport.scrollRow * viewport.rowHeight * viewport.dpr)) /
    viewport.dpr
  )
}

export function playlistDeviceY(
  viewport: Viewport,
  transform: DeviceTransform,
  row: number
) {
  return (
    Math.round(rowGeometry(viewport).top(row) * viewport.dpr) -
    transform.offsetY
  )
}

export function physicalLimits(viewport: Viewport, rowCount: number) {
  return rowGeometry(viewport).top(rowCount) / viewport.rowHeight
}
