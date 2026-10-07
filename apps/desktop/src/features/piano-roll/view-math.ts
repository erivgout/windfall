import type { Clip, Note, PatternId, PlaylistTrack } from "@/bindings"
import {
  clampViewport,
  keyToRow,
  type Viewport,
  type ViewportLimits,
} from "@/lib/canvas"
import { MAX_PATTERN_STEPS, TICKS_PER_STEP } from "@/lib/units"

import { HOME_KEY, notesExtent, type Extent } from "./edit-math"

/*
 * Where the view looks: the scrollable extent, fitting notes on screen,
 * scrollbar geometry and where the song's playhead is inside a pattern.
 */

export const ROW_COUNT = 128
export const MIN_ROW_HEIGHT = 6
export const MAX_ROW_HEIGHT = 40
export const DEFAULT_ROW_HEIGHT = 16
export const DEFAULT_PX_PER_TICK = 0.08
/** Zoomed all the way out, the longest pattern is still a few hundred pixels. */
export const MIN_PX_PER_TICK = 0.0012
export const MAX_PX_PER_TICK = 2

/** Row heights are whole pixels so the keyboard and the grid stay in step. */
export function clampRowHeight(height: number): number {
  return Math.min(MAX_ROW_HEIGHT, Math.max(MIN_ROW_HEIGHT, Math.round(height)))
}

/** The next row height in a zoom direction: always at least a pixel away. */
export function steppedRowHeight(height: number, factor: number): number {
  const scaled = height * factor
  const stepped =
    factor >= 1
      ? Math.max(height + 1, Math.round(scaled))
      : Math.min(height - 1, Math.round(scaled))
  return clampRowHeight(stepped)
}

/**
 * How far the view can scroll: past the pattern and past the last note by
 * a few bars, so there is always room to draw further right.
 */
export function contentTicks(
  lengthSteps: number,
  lastNoteEnd: number,
  barTicks: number
): number {
  const used = Math.max(lengthSteps * TICKS_PER_STEP, lastNoteEnd)
  return Math.max(
    used + 8 * barTicks,
    Math.min(MAX_PATTERN_STEPS * TICKS_PER_STEP, 16 * barTicks)
  )
}

const FIT_PAD_PX = 24
/** Fitting never zooms rows in further than this: a few notes stay note-sized. */
const MAX_FIT_ROW_HEIGHT = 22

/** A viewport that shows a box of notes with a little air around it. */
export function fitViewport(
  viewport: Viewport,
  extent: Extent,
  limits: ViewportLimits
): Viewport {
  const span = Math.max(1, extent.end - extent.start)
  const usable = Math.max(40, viewport.width - 2 * FIT_PAD_PX)
  const pxPerTick = Math.min(
    limits.maxPxPerTick,
    Math.max(limits.minPxPerTick, usable / span)
  )
  const rows = extent.highKey - extent.lowKey + 3
  const rowHeight = Math.min(
    MAX_FIT_ROW_HEIGHT,
    clampRowHeight(Math.floor(viewport.height / rows))
  )
  return clampViewport(
    {
      ...viewport,
      pxPerTick,
      rowHeight,
      scrollTick: extent.start - FIT_PAD_PX / pxPerTick,
      scrollRow: centeredScrollRow(
        viewport.height,
        rowHeight,
        extent.lowKey,
        extent.highKey
      ),
    },
    limits
  )
}

function centeredScrollRow(
  height: number,
  rowHeight: number,
  lowKey: number,
  highKey: number
): number {
  const middle = (keyToRow(highKey) + keyToRow(lowKey) + 1) / 2
  return middle - height / rowHeight / 2
}

/** Scrolls up or down so a range of keys sits in the middle. Zoom stays. */
export function centerOnKeys(
  viewport: Viewport,
  lowKey: number,
  highKey: number,
  limits: ViewportLimits
): Viewport {
  return clampViewport(
    {
      ...viewport,
      scrollRow: centeredScrollRow(
        viewport.height,
        viewport.rowHeight,
        lowKey,
        highKey
      ),
    },
    limits
  )
}

/** Where a lane opens: on its notes, or around C5 when it has none. */
export function openingViewport(
  viewport: Viewport,
  notes: readonly Note[],
  limits: ViewportLimits
): Viewport {
  const extent = notesExtent(notes)
  const start = { ...viewport, scrollTick: 0 }
  if (!extent) return centerOnKeys(start, HOME_KEY, HOME_KEY, limits)
  const centered = centerOnKeys(start, extent.lowKey, extent.highKey, limits)
  const visibleRows = viewport.height / viewport.rowHeight
  if (extent.highKey - extent.lowKey + 1 <= visibleRows) return centered
  // Too tall to show whole: start from the top note.
  return clampViewport(
    { ...start, scrollRow: keyToRow(extent.highKey) - 1 },
    limits
  )
}

export type ThumbGeometry = { offset: number; size: number }

/**
 * A scrollbar thumb for a visible range given as fractions of the content.
 * The thumb never gets smaller than `minSize`, so it can always be grabbed.
 */
export function thumbGeometry(
  start: number,
  end: number,
  track: number,
  minSize: number
): ThumbGeometry {
  const visible = Math.min(1, Math.max(0, end - start))
  const size = Math.min(track, Math.max(minSize, visible * track))
  const travel = track - size
  const scrollable = 1 - visible
  const offset = scrollable <= 0 ? 0 : (start / scrollable) * travel
  return { offset: Math.min(travel, Math.max(0, offset)), size }
}

/** The start fraction a thumb dragged to `offset` pixels stands for. */
export function startAtThumbOffset(
  offset: number,
  visible: number,
  track: number,
  minSize: number
): number {
  const size = Math.min(track, Math.max(minSize, visible * track))
  const travel = track - size
  if (travel <= 0) return 0
  const share = Math.min(1, Math.max(0, offset / travel))
  return share * (1 - visible)
}

/**
 * Where the playhead is inside a pattern while the song plays: the first
 * audible clip of the pattern under the song position decides. Null when
 * no such clip is playing, and then the piano roll shows no playhead.
 */
export function patternTickInSong(
  songTick: number,
  clips: readonly Clip[],
  tracks: readonly PlaylistTrack[],
  pattern: PatternId,
  patternTicks: number
): number | null {
  if (patternTicks <= 0) return null
  for (const clip of clips) {
    if (clip.content.type !== "pattern" || clip.content.pattern !== pattern) {
      continue
    }
    if (songTick < clip.start || songTick >= clip.start + clip.length) continue
    if (clip.muted) continue
    if (tracks.find((track) => track.id === clip.track)?.muted) continue
    return (songTick - clip.start + clip.offset) % patternTicks
  }
  return null
}
