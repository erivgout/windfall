import type {
  Clip,
  ClipId,
  Pattern,
  PatternId,
  PlaylistTrack,
  PlaylistTrackId,
} from "@/bindings"
import {
  indexBatch,
  type Hit,
  type IndexedBatch,
  type Marquee,
  type RowStyle,
} from "@/lib/canvas"
import { useProjectStore } from "@/lib/store/project"
import { ticksPerBar } from "@/lib/time"

import { buildClipBatch } from "./clip-batch"
import { patternTicks, rowIndex, songEnd, type NewClip } from "./edit"
import { contentTicksFor, EDGE_PX, rowCountFor } from "./layout"
import type { GridMetrics } from "./metrics"
import {
  ClipPainter,
  NO_DRAG,
  type DragState,
  type PaintSource,
} from "./painter"
import { project } from "./selectors"
import { gridSpecFor } from "./snap"
import { usePlaylistStore } from "./store"
import type { GridSurface } from "./surface"

const ui = () => usePlaylistStore.getState()
const NOTHING: ReadonlySet<ClipId> = new Set()
/** A clip drawn narrower than this can be pressed from just beside it. */
const SLIVER_PX = 4

/**
 * What the grid shows: the clips of the project as a batch on the canvas,
 * the rows and grid lines that fit them, and the preview of a gesture in
 * progress. It follows the stores by itself and rebuilds the batch once per
 * edit. A change of selection only sets flags on the batch.
 */
export class PlaylistScene implements PaintSource {
  readonly painter: ClipPainter
  /** Called when an edit changed the clips, before the canvas shows them. */
  onClipsChanged: () => void = () => {}

  items: IndexedBatch | null = null
  clips: readonly Clip[] = []
  clipById: ReadonlyMap<ClipId, Clip> = new Map()
  patterns: ReadonlyMap<PatternId, Pattern> = new Map()
  drag: DragState = NO_DRAG
  ghosts: readonly NewClip[] = []
  marked: ReadonlySet<ClipId> = NOTHING
  cloneHint = false
  songEnd = 0

  private readonly surface: GridSurface
  private readonly metrics: GridMetrics
  private readonly stops: (() => void)[]
  private tracks: readonly PlaylistTrack[] = []
  private patternList: readonly Pattern[] = []
  private rows = new Map<PlaylistTrackId, number>()
  private mutedTracks = new Set<PlaylistTrackId>()
  private rowCount = 0

  constructor(surface: GridSurface, metrics: GridMetrics) {
    this.surface = surface
    this.metrics = metrics
    this.painter = new ClipPainter(this)
    this.stops = [
      surface.addOverlayPainter(this.painter.paint),
      surface.onThemeChange(() => this.rebuild()),
      surface.onViewportChange(() => this.fitRows()),
      useProjectStore.subscribe((state, previous) => {
        if (
          state.project.playlist !== previous.project.playlist ||
          state.project.patterns !== previous.project.patterns ||
          state.project.settings.timeSignature !==
            previous.project.settings.timeSignature
        ) {
          this.sync()
        }
      }),
      usePlaylistStore.subscribe((state, previous) => {
        if (state.selection !== previous.selection) this.showSelection()
        if (state.snap !== previous.snap) this.showGrid()
      }),
    ]
    this.sync()
  }

  destroy(): void {
    for (const stop of this.stops) stop()
  }

  isMuted = (clip: Clip): boolean =>
    clip.muted || this.mutedTracks.has(clip.track)

  rowOf = (track: PlaylistTrackId): number => this.rows.get(track) ?? 0

  /** One pass of the pattern a clip plays, in ticks. */
  passTicks = (clip: Clip): number => {
    const pattern = this.patterns.get(clip.content.pattern)
    return pattern ? patternTicks(pattern) : clip.length
  }

  /**
   * The clip under a point given in CSS pixels. The grid lets a press land
   * a couple of pixels outside a rect so that slivers can be clicked; that
   * is kept for slivers only, or a press just past a clip's end would grab
   * the clip instead of placing the next one.
   */
  hitAt(x: number, y: number): Hit | null {
    const exact = this.surface.hitTest(x, y, { edgePx: EDGE_PX, slopPx: 0 })
    if (exact || !this.items) return exact
    const near = this.surface.hitTest(x, y, { edgePx: EDGE_PX })
    if (!near) return null
    const width =
      this.items.batch.length(near.index) * this.surface.viewport.pxPerTick
    return width < SLIVER_PX ? near : null
  }

  /** Reads the project again and redraws. Runs after every edit. */
  private sync(): void {
    const { playlist, patterns } = project()
    const clipsChanged = playlist.clips !== this.clips
    this.tracks = playlist.tracks
    this.clips = playlist.clips
    this.rows = rowIndex(playlist.tracks)
    this.mutedTracks = new Set(
      playlist.tracks.filter((track) => track.muted).map((track) => track.id)
    )
    this.clipById = new Map(playlist.clips.map((clip) => [clip.id, clip]))
    if (patterns !== this.patternList) {
      this.patternList = patterns
      this.patterns = new Map(patterns.map((pattern) => [pattern.id, pattern]))
    }
    this.songEnd = songEnd(playlist.clips)
    if (clipsChanged) this.onClipsChanged()

    const { selection } = ui()
    if ([...selection].some((id) => !this.clipById.has(id))) {
      // Clips can go away under the selection: undo, or a deleted pattern.
      ui().select([...selection].filter((id) => this.clipById.has(id)))
    }
    this.showGrid()
    this.fitRows(true)
    this.rebuild()
  }

  private rebuild(): void {
    this.items = indexBatch(
      buildClipBatch(
        this.clips,
        this.rowOf,
        this.patterns,
        this.isMuted,
        ui().selection,
        this.surface.theme
      )
    )
    this.surface.setItems(this.items)
    this.painter.invalidate()
  }

  /** Marks these clips selected on the canvas. The store is not told. */
  showSelected(ids: Iterable<ClipId>): void {
    const items = this.items
    if (!items) return
    const indices: number[] = []
    for (const id of ids) {
      const index = items.batch.indexOfId(id)
      if (index >= 0) indices.push(index)
    }
    items.batch.setSelection(indices)
    this.redraw()
  }

  /** Shows the selection the store holds. */
  showSelection(): void {
    this.showSelected(ui().selection)
  }

  private showGrid(): void {
    const signature = project().settings.timeSignature
    this.surface.setTimeGrid(gridSpecFor(ui().snap, signature))
  }

  /**
   * Sets how many rows and how much timeline the grid has. Rows below the
   * last track and the rows of muted tracks are the darker ones.
   */
  private fitRows(force = false): void {
    const viewport = this.surface.viewport
    const rowCount = rowCountFor(
      this.tracks.length,
      viewport.height / viewport.rowHeight
    )
    const contentTicks = contentTicksFor(
      this.songEnd,
      ticksPerBar(project().settings.timeSignature)
    )
    const limits = this.surface.limits
    if (
      !force &&
      rowCount === this.rowCount &&
      contentTicks === limits.contentTicks
    ) {
      return
    }
    this.rowCount = rowCount
    const shaded = new Uint8Array(rowCount)
    const strong = new Uint8Array(rowCount + 1)
    this.tracks.forEach((track, row) => {
      if (track.muted) shaded[row] = 1
    })
    for (let row = this.tracks.length; row < rowCount; row++) shaded[row] = 1
    if (this.tracks.length > 0) strong[this.tracks.length] = 1
    const style: RowStyle = { rowCount, shaded, strong }
    this.surface.setRows(style)
    if (rowCount !== limits.rowCount || contentTicks !== limits.contentTicks) {
      this.metrics.setLimits({ rowCount, contentTicks })
    }
  }

  // The preview of a gesture. None of it touches the batch's geometry.

  redraw(): void {
    this.painter.invalidate()
    this.surface.invalidate("all")
  }

  /** Shows the selected clips moved or resized by this much. */
  setDrag(drag: DragState): void {
    this.drag = drag
    this.surface.setDragOffset(drag.ticks, drag.rows)
    this.surface.setDragResize(drag.resizeStart, drag.resizeEnd, drag.minLength)
    this.redraw()
  }

  setGhosts(ghosts: readonly NewClip[]): void {
    this.ghosts = ghosts
    this.redraw()
  }

  setMarked(marked: ReadonlySet<ClipId>): void {
    this.marked = marked
    this.redraw()
  }

  setMarquee(marquee: Marquee | null): void {
    this.surface.setMarquee(marquee)
  }

  clearPreview(): void {
    this.ghosts = []
    this.marked = NOTHING
    this.cloneHint = false
    this.surface.setMarquee(null)
    this.setDrag(NO_DRAG)
  }
}
