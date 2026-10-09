import {
  deviceTransform,
  indexBatch,
  plainRows,
  RectBatch,
  RECT_FLAT,
  RECT_FULL_HEIGHT,
  RECT_FULL_WIDTH,
  rgbFromInt,
  withAlpha,
  writeGrid,
  type GridTheme,
  type HitOptions,
  type IndexedBatch,
  type Layer,
  type Marquee,
  type OverlayPainter,
  type RowStyle,
  type TimeGridSpec,
  type Viewport,
  type ViewportLimits,
} from "@/lib/canvas"

import { rowGeometry } from "./row-geometry"
import type { GridSurface } from "./surface"
import { trackRowsNow } from "./track-rows"

/** Keeps variable playlist rows out of the shared, uniform canvas row model.
 * The renderer receives integer CSS-pixel rows; editing retains row indices.
 */
export class TrackGridSurface implements GridSurface {
  private current: Viewport
  private currentLimits: ViewportLimits
  private rows: RowStyle = plainRows(0)
  private grid: TimeGridSpec = {
    ticksPerStep: 240,
    stepsPerBeat: 4,
    beatsPerBar: 4,
  }
  private items: IndexedBatch | null = null
  private dragRows = 0
  private rendered: {
    batch: RectBatch
    geometryVersion: number
    colorVersion: number
    flagsVersion: number
    geometry: ReturnType<typeof rowGeometry>
    dragRows: number
  } | null = null
  private readonly listeners = new Set<(viewport: Viewport) => void>()
  private readonly stop: () => void
  private updating = false

  private readonly canvas: GridSurface & {
    setUnderlay(items: IndexedBatch | null): void
  }

  constructor(
    canvas: GridSurface & { setUnderlay(items: IndexedBatch | null): void }
  ) {
    this.canvas = canvas
    this.current = canvas.viewport
    this.currentLimits = canvas.limits
    canvas.setRows(plainRows(0))
    canvas.setTimeGrid({ ...this.grid, segments: [] })
    this.stop = canvas.onViewportChange((raw) => {
      if (this.updating) return
      this.current = {
        ...raw,
        rowHeight: this.current.rowHeight,
        scrollRow: raw.scrollRow / this.current.rowHeight,
      }
      this.refresh()
      for (const listener of [...this.listeners]) listener(this.current)
    })
    this.setViewport(this.current)
  }

  destroy() {
    this.stop()
  }
  get viewport() {
    return this.current
  }
  get limits() {
    return this.currentLimits
  }
  get theme() {
    return this.canvas.theme
  }

  setViewport(next: Viewport) {
    this.updating = true
    try {
      const pixels = rowGeometry(next).top(this.currentLimits.rowCount)
      this.canvas.setLimits({
        ...this.currentLimits,
        rowCount: pixels,
        minRowHeight: 1,
        maxRowHeight: 1,
      })
      this.canvas.setViewport({
        ...next,
        rowHeight: 1,
        scrollRow: next.scrollRow * next.rowHeight,
      })
      const raw = this.canvas.viewport
      this.current = {
        ...raw,
        rowHeight: next.rowHeight,
        scrollRow: raw.scrollRow / next.rowHeight,
      }
    } finally {
      this.updating = false
    }
    // Height overrides can change while the canvas viewport remains identical.
    this.refresh()
    for (const listener of [...this.listeners]) listener(this.current)
  }

  setLimits(limits: Partial<ViewportLimits>) {
    this.currentLimits = { ...this.currentLimits, ...limits }
    this.setViewport(this.current)
  }
  setRows(rows: RowStyle) {
    this.rows = rows
    this.refresh()
  }
  setTimeGrid(grid: TimeGridSpec) {
    this.grid = grid
    this.refresh()
  }
  setItems(items: IndexedBatch | null) {
    this.items = items
    this.refreshItems()
  }

  private refresh() {
    const viewport = this.current
    const geometry = rowGeometry(viewport)
    const first = Math.max(
      0,
      Math.floor(geometry.rowAt(viewport.scrollRow * viewport.rowHeight))
    )
    const last = Math.ceil(
      geometry.rowAt(viewport.scrollRow * viewport.rowHeight + viewport.height)
    )
    const grid = new RectBatch()
    writeGrid(
      grid,
      {
        ...viewport,
        scrollRow: first,
        height: (last - first + 1) * viewport.rowHeight,
      },
      this.grid,
      this.rows,
      this.theme
    )
    const { rows } = trackRowsNow()
    for (let row = first; row < Math.min(last + 1, rows.length); row++) {
      const entry = rows[row]
      if (entry.kind !== "track" || !entry.track.color) continue
      grid.push(
        -1,
        0,
        0,
        row,
        1,
        withAlpha(rgbFromInt(entry.track.color), 0.1),
        RECT_FLAT | RECT_FULL_WIDTH
      )
    }
    this.canvas.setUnderlay(indexBatch(this.toPixels(grid)))
    this.refreshItems()
  }

  private toPixels(source: RectBatch, move = false) {
    const geometry = rowGeometry(this.current)
    const result = new RectBatch(Math.max(1, source.count))
    for (let index = 0; index < source.count; index++) {
      const selected = move && source.isSelected(index)
      const row = source.row(index) + (selected ? this.dragRows : 0)
      const fullHeight = (source.flags[index] & RECT_FULL_HEIGHT) !== 0
      const color = index * 4
      result.push(
        source.ids[index],
        source.start(index),
        source.length(index),
        fullHeight ? 0 : geometry.top(row),
        fullHeight
          ? 0
          : geometry.top(row + source.rowSpan(index)) - geometry.top(row),
        {
          r: source.colors[color],
          g: source.colors[color + 1],
          b: source.colors[color + 2],
          a: source.colors[color + 3],
        },
        source.flags[index]
      )
    }
    return result
  }

  private refreshItems() {
    const batch = this.items?.batch
    if (!batch) {
      this.rendered = null
      this.canvas.setItems(null)
      return
    }
    const geometry = rowGeometry(this.current)
    const rendered = this.rendered
    if (
      rendered?.batch === batch &&
      rendered.geometryVersion === batch.geometryVersion &&
      rendered.colorVersion === batch.colorVersion &&
      rendered.flagsVersion === batch.flagsVersion &&
      rendered.geometry === geometry &&
      rendered.dragRows === this.dragRows
    )
      return
    this.rendered = {
      batch,
      geometryVersion: batch.geometryVersion,
      colorVersion: batch.colorVersion,
      flagsVersion: batch.flagsVersion,
      geometry,
      dragRows: this.dragRows,
    }
    this.canvas.setItems(indexBatch(this.toPixels(batch, true)))
  }
  setDragOffset(ticks: number, rows: number) {
    this.dragRows = rows
    this.canvas.setDragOffset(ticks, 0)
    this.refreshItems()
  }
  setDragResize(start: number, end: number, minLength?: number) {
    this.canvas.setDragResize(start, end, minLength)
  }
  setMarquee(marquee: Marquee | null) {
    const geometry = rowGeometry(this.current)
    this.canvas.setMarquee(
      marquee
        ? {
            ...marquee,
            row0: geometry.top(marquee.row0),
            row1: geometry.top(marquee.row1),
          }
        : null
    )
  }
  setPlayhead(tick: number | null) {
    this.canvas.setPlayhead(tick)
  }
  addOverlayPainter(painter: OverlayPainter) {
    return this.canvas.addOverlayPainter((ctx) =>
      painter(ctx, {
        viewport: this.current,
        transform: deviceTransform(this.current),
        theme: this.theme,
      })
    )
  }
  onThemeChange(listener: (theme: GridTheme) => void) {
    return this.canvas.onThemeChange((theme) => {
      this.refresh()
      listener(theme)
    })
  }
  onViewportChange(listener: (viewport: Viewport) => void) {
    this.listeners.add(listener)
    return () => {
      this.listeners.delete(listener)
    }
  }
  invalidate(layer?: Layer) {
    this.refreshItems()
    this.canvas.invalidate(layer)
  }
  hitTest(x: number, y: number, options?: HitOptions) {
    return this.canvas.hitTest(x, y, options)
  }
}
