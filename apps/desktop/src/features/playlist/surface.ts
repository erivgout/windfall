import type {
  GridTheme,
  Hit,
  HitOptions,
  IndexedBatch,
  Layer,
  Marquee,
  OverlayPainter,
  RowStyle,
  TimeGridSpec,
  Viewport,
  ViewportLimits,
} from "@/lib/canvas"

/**
 * The part of `TimeGridView` the playlist uses. The pointer handling is
 * written against this, so tests can run it on a stand-in with no canvas.
 */
export interface GridSurface {
  readonly viewport: Viewport
  readonly limits: ViewportLimits
  readonly theme: GridTheme
  setViewport(next: Viewport): void
  setLimits(limits: Partial<ViewportLimits>): void
  setRows(rows: RowStyle): void
  setTimeGrid(spec: TimeGridSpec): void
  setItems(items: IndexedBatch | null): void
  setDragOffset(ticks: number, rows: number): void
  setDragResize(startTicks: number, endTicks: number, minLength?: number): void
  setMarquee(marquee: Marquee | null): void
  setPlayhead(tick: number | null): void
  addOverlayPainter(painter: OverlayPainter): () => void
  onThemeChange(listener: (theme: GridTheme) => void): () => void
  onViewportChange(listener: (viewport: Viewport) => void): () => void
  invalidate(layer?: Layer): void
  hitTest(x: number, y: number, options?: HitOptions): Hit | null
}
