import type {
  Clip,
  ClipContent,
  ClipId,
  ClipInit,
  ClipUpdate,
  Pattern,
  PlaylistTrack,
  PlaylistTrackId,
} from "@/bindings"
import { resizedSpan, snapTick } from "@/lib/canvas"
import { clamp, MAX_SONG_TICKS, TICKS_PER_STEP } from "@/lib/units"

/*
 * The arithmetic of editing clips, with no pointer, canvas or store in it.
 * Clips are placed by row here; rows become track ids only when a command is
 * built, because a row below the last track has no track yet.
 */

/** The shortest a clip can be made when snapping is off. */
export const MIN_CLIP_TICKS = 60
/** The last tick a clip may end on: the end of the longest song. */
export const MAX_TICK = MAX_SONG_TICKS

/** Whether a clip with this span ends inside what the timeline can count. */
export function spanFits(start: number, length: number): boolean {
  return start >= 0 && start + length <= MAX_TICK
}

/** A clip that is about to be created. */
export type NewClip = {
  row: number
  start: number
  length: number
  offset: number
  muted: boolean
  content: ClipContent
}

/** What a clip plays, as text: two clips with the same key play the same thing. */
export function contentKey(content: ClipContent): string {
  switch (content.type) {
    case "pattern":
      return `pattern:${content.pattern}`
    case "audio":
      return `audio:${content.sample}`
    case "automation":
      return `automation:${content.automation}`
    default: {
      const _exhaustive: never = content
      return _exhaustive
    }
  }
}

/** A change to an existing clip. Missing fields stay as they are. */
export type ClipChange = {
  id: ClipId
  row?: number
  start?: number
  length?: number
  offset?: number
  muted?: boolean
}

export type RowOf = (track: PlaylistTrackId) => number

/** `value` modulo `length`, always from 0 up to `length`. */
export function wrapTicks(value: number, length: number): number {
  if (length <= 0) return 0
  return ((value % length) + length) % length
}

export function patternTicks(pattern: Pick<Pattern, "lengthSteps">): number {
  return pattern.lengthSteps * TICKS_PER_STEP
}

/** Where the song ends: the end of the last clip, or 0 with no clips. */
export function songEnd(clips: readonly Clip[]): number {
  let end = 0
  for (const clip of clips) end = Math.max(end, clip.start + clip.length)
  return end
}

export function rowIndex(
  tracks: readonly PlaylistTrack[]
): Map<PlaylistTrackId, number> {
  return new Map(tracks.map((track, row) => [track.id, row]))
}

/** The nearest grid line, never before the start of the song. */
export function snapNearest(tick: number, snap: number): number {
  return Math.max(0, snapTick(tick, snap))
}

/** The grid line at or before a tick: the cell the pointer is in. */
export function snapCell(tick: number, snap: number): number {
  const cell = snap > 0 ? Math.floor(tick / snap) * snap : Math.round(tick)
  return Math.max(0, cell)
}

/**
 * How far a drag moves the selection. The grabbed clip's start lands on the
 * grid and everything else moves by the same amount. Nothing goes before the
 * start of the song or outside the rows.
 */
export function clampMove(
  items: readonly { start: number; row: number }[],
  anchorStart: number,
  rawTicks: number,
  rawRows: number,
  snap: number,
  rowCount: number
): { ticks: number; rows: number } {
  let minStart = Infinity
  let minRow = Infinity
  let maxRow = -Infinity
  for (const item of items) {
    minStart = Math.min(minStart, item.start)
    minRow = Math.min(minRow, item.row)
    maxRow = Math.max(maxRow, item.row)
  }
  if (items.length === 0) return { ticks: 0, rows: 0 }
  const target = snapNearest(anchorStart + rawTicks, snap)
  // "|| 0" turns a negative zero into a plain one.
  return {
    ticks: Math.max(target - anchorStart, -minStart) || 0,
    rows: clamp(Math.round(rawRows), -minRow, rowCount - 1 - maxRow) || 0,
  }
}

/** Like `clampMove` for an arrow key: a fixed step, not a snap to the grid. */
export function clampNudge(
  items: readonly { start: number; row: number }[],
  ticks: number,
  rows: number,
  rowCount: number
): { ticks: number; rows: number } {
  if (items.length === 0) return { ticks: 0, rows: 0 }
  let minStart = Infinity
  let minRow = Infinity
  let maxRow = -Infinity
  for (const item of items) {
    minStart = Math.min(minStart, item.start)
    minRow = Math.min(minRow, item.row)
    maxRow = Math.max(maxRow, item.row)
  }
  return {
    ticks: Math.max(ticks, -minStart) || 0,
    rows: clamp(rows, -minRow, rowCount - 1 - maxRow) || 0,
  }
}

export function moveChanges(
  clips: readonly Clip[],
  rowOf: RowOf,
  ticks: number,
  rows: number
): ClipChange[] {
  if (ticks === 0 && rows === 0) return []
  return clips.map((clip) => {
    const change: ClipChange = { id: clip.id }
    if (ticks !== 0) change.start = clip.start + ticks
    if (rows !== 0) change.row = rowOf(clip.track) + rows
    return change
  })
}

/** The shortest a resize may leave a clip: one grid cell. */
export function minResizeLength(snap: number): number {
  return snap > 0 ? snap : MIN_CLIP_TICKS
}

/** Ticks to add to the end of the grabbed clip so it meets the pointer. */
export function endResizeDelta(
  anchor: Clip,
  pointerTick: number,
  snap: number
): number {
  return snapNearest(pointerTick, snap) - (anchor.start + anchor.length)
}

/**
 * Ticks to add to the start of the grabbed clip so it meets the pointer.
 * Stops where the earliest selected clip would cross the start of the song,
 * and where a clip that does not loop would reach the start of what it
 * plays: an audio clip has no audio before its first frame.
 */
export function startTrimDelta(
  clips: readonly Clip[],
  anchor: Clip,
  pointerTick: number,
  snap: number,
  passTicks: (clip: Clip) => number = () => 1
): number {
  let limit = -anchor.start
  for (const clip of [anchor, ...clips]) {
    limit = Math.max(limit, -clip.start)
    if (passTicks(clip) <= 0) limit = Math.max(limit, -clip.offset)
  }
  // "|| 0" turns a negative zero into a plain one.
  return Math.max(snapNearest(pointerTick, snap) - anchor.start, limit) || 0
}

/**
 * What an edge drag does to each clip. Moving the start edge changes
 * `offset` by the same amount, so what is inside stays where it was on the
 * timeline: trimming hides the beginning. A pattern clip loops, so pulling
 * its edge left uncovers the end of the previous pass; `passTicks` is the
 * length of that loop, and 0 for a clip of audio or automation, whose
 * offset simply stops at 0.
 */
export function resizeChanges(
  clips: readonly Clip[],
  startDelta: number,
  endDelta: number,
  minLength: number,
  passTicks: (clip: Clip) => number
): ClipChange[] {
  const changes: ClipChange[] = []
  for (const clip of clips) {
    const span = resizedSpan(
      clip.start,
      clip.length,
      startDelta,
      endDelta,
      minLength
    )
    const shift = span.start - clip.start
    if (shift === 0 && span.length === clip.length) continue
    const change: ClipChange = { id: clip.id }
    if (shift !== 0) {
      change.start = span.start
      const pass = passTicks(clip)
      change.offset =
        pass > 0
          ? wrapTicks(clip.offset + shift, pass)
          : Math.max(0, clip.offset + shift)
    }
    if (span.length !== clip.length) change.length = span.length
    changes.push(change)
  }
  return changes
}

/**
 * Starts of the clips a paint stroke lays down: one pass of the brush
 * after another from where the stroke began to where the pointer is, in
 * either direction.
 */
export function paintStarts(
  anchor: number,
  pointerTick: number,
  passTicks: number | ((tick: number) => number)
): number[] {
  if (typeof passTicks === "function") {
    if (passTicks(anchor) <= 0) return []
    const starts = [anchor]
    // Each new clip uses the meter at its own start. Looking just before
    // the prior start keeps a leftward stroke in the preceding segment.
    if (pointerTick < anchor) {
      let start = anchor
      while (start > pointerTick) {
        const pass = passTicks(Math.max(0, start - 1))
        if (pass <= 0) break
        start -= pass
        if (start < 0) break
        starts.unshift(start)
      }
    } else {
      let start = anchor
      while (true) {
        const pass = passTicks(start)
        if (pass <= 0) break
        start += pass
        if (start > pointerTick) break
        starts.push(start)
      }
    }
    return starts
  }
  if (passTicks <= 0) return []
  const reach = Math.floor((pointerTick - anchor) / passTicks)
  const starts: number[] = []
  for (let k = Math.min(0, reach); k <= Math.max(0, reach); k++) {
    const start = anchor + k * passTicks
    if (start >= 0) starts.push(start)
  }
  return starts
}

/**
 * Drops new clips that would sit exactly on top of a clip that already
 * plays the same thing there, which would only double its volume unseen.
 */
export function withoutStacked(
  candidates: readonly NewClip[],
  existing: readonly Clip[],
  rowOf: RowOf
): NewClip[] {
  const taken = new Set(
    existing.map(
      (clip) =>
        `${rowOf(clip.track)}:${clip.start}:${clip.length}:${contentKey(clip.content)}`
    )
  )
  return candidates.filter(
    (clip) =>
      !taken.has(
        `${clip.row}:${clip.start}:${clip.length}:${contentKey(clip.content)}`
      )
  )
}

function asNewClip(clip: Clip, rowOf: RowOf): NewClip {
  return {
    row: rowOf(clip.track),
    start: clip.start,
    length: clip.length,
    offset: clip.offset,
    muted: clip.muted,
    content: clip.content,
  }
}

/** Copies of clips moved by a drag, for Ctrl+drag. */
export function cloneMoved(
  clips: readonly Clip[],
  rowOf: RowOf,
  ticks: number,
  rows: number
): NewClip[] {
  return clips.map((clip) => {
    const copy = asNewClip(clip, rowOf)
    return { ...copy, start: copy.start + ticks, row: copy.row + rows }
  })
}

/**
 * Copies of clips placed right after them. The copies start a whole number
 * of grid cells later, so a selection that sat on the grid stays on it.
 */
export function duplicateRight(
  clips: readonly Clip[],
  rowOf: RowOf,
  snap: number
): NewClip[] {
  if (clips.length === 0) return []
  let first = Infinity
  let last = 0
  for (const clip of clips) {
    first = Math.min(first, clip.start)
    last = Math.max(last, clip.start + clip.length)
  }
  const span = last - first
  const shift = snap > 0 ? Math.max(snap, Math.ceil(span / snap) * snap) : span
  return cloneMoved(clips, rowOf, shift, 0)
}

/** What copy keeps: the clips with times counted from the earliest one. */
export function clipboardFrom(clips: readonly Clip[], rowOf: RowOf): NewClip[] {
  let first = Infinity
  for (const clip of clips) first = Math.min(first, clip.start)
  return clips.map((clip) => {
    const copy = asNewClip(clip, rowOf)
    return { ...copy, start: copy.start - first }
  })
}

/**
 * Where pasted clips go: on the rows they were copied from, with the
 * earliest one starting at `tick` snapped to the grid. Clips whose pattern,
 * sample or automation has been deleted since are left out.
 */
export function pasteAt(
  copied: readonly NewClip[],
  tick: number,
  snap: number,
  exists: (content: ClipContent) => boolean
): NewClip[] {
  const at = snapNearest(tick, snap)
  return copied
    .filter((clip) => exists(clip.content))
    .map((clip) => ({ ...clip, start: at + clip.start }))
}

/** How many tracks must be added before these rows all have one. */
export function tracksNeeded(
  trackCount: number,
  rows: Iterable<number>
): number {
  let highest = -1
  for (const row of rows) highest = Math.max(highest, row)
  return Math.max(0, highest + 1 - trackCount)
}

const wholeTick = (value: number, min: number) =>
  clamp(Math.round(value), min, MAX_TICK)

/** `addClips` entries for new clips, once every row has a track. */
export function clipInits(
  clips: readonly NewClip[],
  tracks: readonly PlaylistTrackId[]
): ClipInit[] {
  return clips.map((clip) => ({
    track: tracks[clip.row],
    start: wholeTick(clip.start, 0),
    length: wholeTick(clip.length, 1),
    ...(clip.offset !== 0 && { offset: wholeTick(clip.offset, 0) }),
    ...(clip.muted && { muted: true }),
    content: clip.content,
  }))
}

/** `updateClips` entries for changes, once every row has a track. */
export function clipUpdates(
  changes: readonly ClipChange[],
  tracks: readonly PlaylistTrackId[]
): ClipUpdate[] {
  return changes.map((change) => ({
    id: change.id,
    patch: {
      ...(change.row !== undefined && { track: tracks[change.row] }),
      ...(change.start !== undefined && { start: wholeTick(change.start, 0) }),
      ...(change.length !== undefined && {
        length: wholeTick(change.length, 1),
      }),
      ...(change.offset !== undefined && {
        offset: wholeTick(change.offset, 0),
      }),
      ...(change.muted !== undefined && { muted: change.muted }),
    },
  }))
}

/**
 * Where a track dragged from row `from` and dropped into gap `gap` ends up,
 * as the index `movePlaylistTrack` takes, or null when the drop changes
 * nothing. Gap 0 is above the first track and gap `n` below the last of `n`.
 */
export function trackDropIndex(
  from: number,
  gap: number,
  trackCount: number
): number | null {
  if (from < 0 || from >= trackCount) return null
  // The track leaves its place first, so the gaps below it move up one.
  const to = clamp(gap > from ? gap - 1 : gap, 0, trackCount - 1)
  return to === from ? null : to
}

/** Every clip a pointer stroke from one point to another passes over. */
export function strokeBox(
  from: { tick: number; row: number },
  to: { tick: number; row: number }
): { tick0: number; tick1: number; row0: number; row1: number } {
  // A box with no area finds nothing, so a still pointer gets one tick and
  // a sliver of a row.
  return {
    tick0: Math.min(from.tick, to.tick),
    tick1: Math.max(from.tick, to.tick) + 1,
    row0: Math.min(from.row, to.row),
    row1: Math.max(from.row, to.row) + 0.001,
  }
}
