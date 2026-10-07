import type {
  Clip,
  ClipId,
  ClipInit,
  ClipUpdate,
  Command,
  Pattern,
  PatternId,
  Playlist,
  PlaylistTrack,
  PlaylistTrackId,
} from "@/bindings"
import { resizedSpan, snapTick } from "@/lib/canvas"
import { clamp, TICKS_PER_STEP } from "@/lib/units"

/*
 * The arithmetic of editing clips, with no pointer, canvas or store in it.
 * Clips are placed by row here; rows become track ids only when a command is
 * built, because a row below the last track has no track yet.
 */

/** The shortest a clip can be made when snapping is off. */
export const MIN_CLIP_TICKS = 60
/** Clip geometry is drawn from 32-bit integers. */
export const MAX_TICK = 0x7fffffff

/** A clip that is about to be created. */
export type NewClip = {
  row: number
  start: number
  length: number
  offset: number
  muted: boolean
  pattern: PatternId
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
 * Stops where the earliest selected clip would cross the start of the song.
 */
export function startTrimDelta(
  clips: readonly Clip[],
  anchor: Clip,
  pointerTick: number,
  snap: number
): number {
  let minStart = anchor.start
  for (const clip of clips) minStart = Math.min(minStart, clip.start)
  return Math.max(snapNearest(pointerTick, snap) - anchor.start, -minStart)
}

/**
 * What an edge drag does to each clip. Moving the start edge changes
 * `offset` by the same amount, so the notes inside stay where they were on
 * the timeline: trimming hides the beginning, and pulling the edge left
 * uncovers the end of the previous pass of the loop.
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
      change.offset = wrapTicks(clip.offset + shift, passTicks(clip))
    }
    if (span.length !== clip.length) change.length = span.length
    changes.push(change)
  }
  return changes
}

/**
 * Starts of the clips a paint stroke lays down: one pass of the pattern
 * after another from where the stroke began to where the pointer is, in
 * either direction.
 */
export function paintStarts(
  anchor: number,
  pointerTick: number,
  passTicks: number
): number[] {
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
        `${rowOf(clip.track)}:${clip.start}:${clip.length}:${clip.content.pattern}`
    )
  )
  return candidates.filter(
    (clip) =>
      !taken.has(`${clip.row}:${clip.start}:${clip.length}:${clip.pattern}`)
  )
}

function asNewClip(clip: Clip, rowOf: RowOf): NewClip {
  return {
    row: rowOf(clip.track),
    start: clip.start,
    length: clip.length,
    offset: clip.offset,
    muted: clip.muted,
    pattern: clip.content.pattern,
  }
}

/** Copies of clips moved by a drag, for Shift+drag. */
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
 * earliest one starting at `tick` snapped to the grid. Clips whose pattern
 * has been deleted since are left out.
 */
export function pasteAt(
  copied: readonly NewClip[],
  tick: number,
  snap: number,
  hasPattern: (pattern: PatternId) => boolean
): NewClip[] {
  const at = snapNearest(tick, snap)
  return copied
    .filter((clip) => hasPattern(clip.pattern))
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
    content: { type: "pattern", pattern: clip.pattern },
  }))
}

/**
 * `addClips` cannot set an offset or mute a clip, so copies of clips that
 * have either need a second command once their ids are known.
 */
export function clipFollowUps(
  clips: readonly NewClip[],
  created: readonly ClipId[]
): ClipUpdate[] {
  const updates: ClipUpdate[] = []
  clips.forEach((clip, index) => {
    const id = created[index]
    if (id === undefined || (clip.offset === 0 && !clip.muted)) return
    updates.push({
      id,
      patch: {
        ...(clip.offset !== 0 && { offset: wholeTick(clip.offset, 0) }),
        ...(clip.muted && { muted: true }),
      },
    })
  })
  return updates
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
 * The model can only add a track at the bottom. To put an empty track at
 * `index`, a track is added at the bottom and this moves everything from
 * `index` on down one row: clips, names and mute states. `added` is the new
 * bottom track, and `playlist` is the playlist before it was added.
 */
export function insertTrackCommands(
  playlist: Playlist,
  index: number,
  added: PlaylistTrack
): Command[] {
  const before = playlist.tracks
  if (index < 0 || index >= before.length) return []
  const after = [...before, added]
  const nextTrack = new Map<PlaylistTrackId, PlaylistTrackId>()
  for (let row = index; row < before.length; row++) {
    nextTrack.set(before[row].id, after[row + 1].id)
  }

  const commands: Command[] = []
  const updates: ClipUpdate[] = []
  for (const clip of playlist.clips) {
    const track = nextTrack.get(clip.track)
    if (track !== undefined) updates.push({ id: clip.id, patch: { track } })
  }
  if (updates.length > 0) commands.push({ type: "updateClips", updates })

  for (let row = before.length; row > index; row--) {
    const from = before[row - 1]
    const to = after[row]
    if (from.name === to.name && from.muted === to.muted) continue
    commands.push({
      type: "updatePlaylistTrack",
      id: to.id,
      patch: { name: from.name, muted: from.muted },
    })
  }
  const emptied = before[index]
  if (emptied.name !== added.name || emptied.muted) {
    commands.push({
      type: "updatePlaylistTrack",
      id: emptied.id,
      patch: { name: added.name, muted: false },
    })
  }
  return commands
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
