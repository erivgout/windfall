import { rgbaToCss } from "./color"
import { createRenderer, type RendererChoice } from "./create-renderer"
import {
  DEFAULT_TIME_GRID,
  pianoRows,
  writeGrid,
  type RowStyle,
  type TimeGridSpec,
} from "./grid"
import { hitTestPoint, type Hit, type HitOptions } from "./hit-test"
import { RectBatch } from "./rect-batch"
import type { RectRenderer } from "./renderer"
import { createCanvas2DRenderer } from "./renderer-canvas2d"
import { visibleRange, type IndexedBatch } from "./spatial-index"
import {
  createCanvasColorParser,
  deriveGridTheme,
  observeTheme,
  readThemeTokens,
  type CssColorParser,
  type GridTheme,
  type ThemeTokens,
} from "./theme"
import {
  DEFAULT_LIMITS,
  backingSize,
  clampViewport,
  deviceTransform,
  deviceX,
  deviceY,
  scrollByPx,
  visibleTicks,
  zoomRowsAt,
  zoomTimeAt,
  type DeviceTransform,
  type Viewport,
  type ViewportLimits,
} from "./viewport"

export interface TimeGridViewOptions {
  /** Defaults to "auto": WebGL2, then Canvas 2D if that fails. */
  readonly renderer?: RendererChoice
  readonly limits?: Partial<ViewportLimits>
  readonly timeGrid?: TimeGridSpec
  /** Defaults to piano rows for `limits.rowCount`. */
  readonly rows?: RowStyle
  readonly initial?: Partial<
    Pick<Viewport, "scrollTick" | "scrollRow" | "pxPerTick" | "rowHeight">
  >
  /**
   * When true (the default) the view draws itself on the next animation
   * frame after anything changes. Turn it off to call `flush` yourself.
   */
  readonly autoRender?: boolean
}

/** A selection box in content space, so it stays put while the view scrolls. */
export interface Marquee {
  readonly tick0: number
  readonly row0: number
  readonly tick1: number
  readonly row1: number
}

export interface OverlayFrame {
  readonly viewport: Viewport
  readonly transform: DeviceTransform
  readonly theme: GridTheme
}

/**
 * Draws extra content on the overlay canvas, under the marquee and the
 * playhead. Coordinates are device pixels: use `deviceX` and `deviceY`.
 */
export type OverlayPainter = (
  ctx: CanvasRenderingContext2D,
  frame: OverlayFrame
) => void

export interface FrameStats {
  /** Whether the grid and items were redrawn. */
  readonly base: boolean
  /** Whether the marquee and playhead layer was redrawn. */
  readonly overlay: boolean
  /** Main-thread time spent issuing the draw, in milliseconds. */
  readonly cpuMs: number
  /** Size of the item index range sent to the renderer. */
  readonly itemsInRange: number
}

export type Layer = "base" | "overlay" | "all"

function styleCanvas(canvas: HTMLCanvasElement, interactive: boolean): void {
  canvas.style.position = "absolute"
  canvas.style.inset = "0"
  canvas.style.width = "100%"
  canvas.style.height = "100%"
  canvas.style.display = "block"
  if (!interactive) canvas.style.pointerEvents = "none"
}

/**
 * A scrollable, zoomable grid of rects over time: the drawing half of the
 * piano roll and the playlist. It owns two stacked canvases. The base one
 * holds the grid and the items and is drawn by the chosen renderer. The
 * overlay holds the marquee and the playhead, so those move without the
 * items being redrawn or any item data being touched.
 *
 * Overlay painters draw what belongs to the items: names, outlines, the
 * places a drag started from. So whatever changes the items or where they
 * are shown (`setItems`, `setDragOffset`, `setDragResize`, the viewport)
 * redraws both layers, and a painter needs no invalidation of its own for
 * those. Only the marquee and the playhead redraw the overlay alone.
 *
 * The view holds no project state. Give it a batch, a viewport and a
 * playhead position; it draws them.
 */
export class TimeGridView {
  /** Called after every drawn frame. */
  onFrame: ((stats: FrameStats) => void) | null = null

  private readonly container: HTMLElement
  /** Takes the pointer. The first renderer draws into it. */
  private readonly baseCanvas: HTMLCanvasElement
  /** What the renderer in use draws into. */
  private drawCanvas: HTMLCanvasElement
  private activeRenderer: RectRenderer
  private readonly overlayCanvas: HTMLCanvasElement
  private readonly overlayContext: CanvasRenderingContext2D | null
  private readonly gridBatch = new RectBatch(512)
  private readonly parseColor: CssColorParser
  private readonly resizeObserver: ResizeObserver
  private readonly stopThemeObserver: () => void
  private timeGrid: TimeGridSpec
  private readonly autoRender: boolean
  private rows: RowStyle
  private viewportLimits: ViewportLimits
  private currentViewport: Viewport
  private tokens: ThemeTokens
  private currentTheme: GridTheme
  private currentItems: IndexedBatch | null = null
  private currentUnderlay: IndexedBatch | null = null
  private dragTicks = 0
  private dragRows = 0
  private resizeStart = 0
  private resizeEnd = 0
  private resizeMinLength = 0
  private marquee: Marquee | null = null
  private playheadTick: number | null = null
  private overlayPainters: OverlayPainter[] = []
  private themeListeners: ((theme: GridTheme) => void)[] = []
  private viewportListeners: ((viewport: Viewport) => void)[] = []
  private baseDirty = true
  private overlayDirty = true
  private themeDirty = false
  private frameRequest = 0
  private destroyed = false

  static async create(
    container: HTMLElement,
    options: TimeGridViewOptions = {}
  ): Promise<TimeGridView> {
    const { renderer, canvas } = await createRenderer(
      options.renderer ?? "auto",
      () => document.createElement("canvas")
    )
    return new TimeGridView(container, renderer, canvas, options)
  }

  private constructor(
    container: HTMLElement,
    renderer: RectRenderer,
    baseCanvas: HTMLCanvasElement,
    options: TimeGridViewOptions
  ) {
    this.container = container
    this.activeRenderer = renderer
    this.baseCanvas = baseCanvas
    this.drawCanvas = baseCanvas
    this.overlayCanvas = document.createElement("canvas")
    this.overlayContext = this.overlayCanvas.getContext("2d")
    this.timeGrid = options.timeGrid ?? DEFAULT_TIME_GRID
    this.autoRender = options.autoRender ?? true
    this.viewportLimits = { ...DEFAULT_LIMITS, ...options.limits }
    this.rows = options.rows ?? pianoRows(this.viewportLimits.rowCount)

    if (getComputedStyle(container).position === "static") {
      container.style.position = "relative"
    }
    styleCanvas(baseCanvas, true)
    styleCanvas(this.overlayCanvas, false)
    container.append(baseCanvas, this.overlayCanvas)

    this.parseColor = createCanvasColorParser()
    this.tokens = readThemeTokens(container, this.parseColor)
    this.currentTheme = deriveGridTheme(this.tokens)
    this.adopt(renderer)

    const dpr = window.devicePixelRatio || 1
    this.currentViewport = clampViewport(
      {
        width: container.clientWidth,
        height: container.clientHeight,
        dpr,
        scrollTick: options.initial?.scrollTick ?? 0,
        scrollRow: options.initial?.scrollRow ?? 0,
        pxPerTick: options.initial?.pxPerTick ?? 0.0625,
        rowHeight: options.initial?.rowHeight ?? 16,
      },
      this.viewportLimits
    )
    this.resizeCanvases(
      Math.round(container.clientWidth * dpr),
      Math.round(container.clientHeight * dpr)
    )

    this.resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[entries.length - 1]
      if (entry) this.handleResize(entry)
    })
    try {
      this.resizeObserver.observe(container, {
        box: "device-pixel-content-box",
      })
    } catch {
      // Safari has no device-pixel-content-box.
      this.resizeObserver.observe(container)
    }
    this.stopThemeObserver = observeTheme(() => {
      this.themeDirty = true
      this.invalidate("all")
    })
    this.invalidate("all")
  }

  get viewport(): Viewport {
    return this.currentViewport
  }

  get limits(): ViewportLimits {
    return this.viewportLimits
  }

  get theme(): GridTheme {
    return this.currentTheme
  }

  get items(): IndexedBatch | null {
    return this.currentItems
  }

  /**
   * The renderer in use. It changes to a Canvas 2D one if a GPU renderer
   * loses its context for good, so read it when it is needed.
   */
  get renderer(): RectRenderer {
    return this.activeRenderer
  }

  /**
   * The canvas that receives pointer and wheel events. It stays the same
   * element for the life of the view.
   */
  get element(): HTMLCanvasElement {
    return this.baseCanvas
  }

  get transform(): DeviceTransform {
    return {
      ...deviceTransform(this.currentViewport),
      widthDev: this.drawCanvas.width,
      heightDev: this.drawCanvas.height,
    }
  }

  setViewport(next: Viewport): void {
    const clamped = clampViewport(next, this.viewportLimits)
    const previous = this.currentViewport
    if (
      clamped.scrollTick === previous.scrollTick &&
      clamped.scrollRow === previous.scrollRow &&
      clamped.pxPerTick === previous.pxPerTick &&
      clamped.rowHeight === previous.rowHeight &&
      clamped.width === previous.width &&
      clamped.height === previous.height &&
      clamped.dpr === previous.dpr
    ) {
      return
    }
    this.currentViewport = clamped
    this.invalidate("all")
    for (const listener of this.viewportListeners) listener(clamped)
  }

  setLimits(limits: Partial<ViewportLimits>): void {
    this.viewportLimits = { ...this.viewportLimits, ...limits }
    this.setViewport(this.currentViewport)
  }

  setRows(rows: RowStyle): void {
    this.rows = rows
    this.invalidate("base")
  }

  /** Changes which time lines are drawn: the snap, or the time signature. */
  setTimeGrid(spec: TimeGridSpec): void {
    const current = this.timeGrid
    if (
      spec.ticksPerStep === current.ticksPerStep &&
      spec.stepsPerBeat === current.stepsPerBeat &&
      spec.beatsPerBar === current.beatsPerBar
    ) {
      return
    }
    this.timeGrid = spec
    this.invalidate("base")
  }

  panBy(dxPx: number, dyPx: number): void {
    this.setViewport(
      scrollByPx(this.currentViewport, dxPx, dyPx, this.viewportLimits)
    )
  }

  zoomTime(anchorX: number, factor: number): void {
    this.setViewport(
      zoomTimeAt(this.currentViewport, anchorX, factor, this.viewportLimits)
    )
  }

  zoomRows(anchorY: number, factor: number): void {
    this.setViewport(
      zoomRowsAt(this.currentViewport, anchorY, factor, this.viewportLimits)
    )
  }

  /**
   * Replaces the items. The previous batch's GPU buffers are freed, so do
   * not hand the old batch back later.
   */
  setItems(items: IndexedBatch | null): void {
    const previous = this.currentItems
    if (previous && previous.batch !== items?.batch) {
      this.activeRenderer.release(previous.batch)
    }
    this.currentItems = items
    this.invalidate("all")
  }

  /**
   * A second batch drawn behind the items, such as the notes of other
   * channels in a piano roll. It is never selected, dragged or hit-tested.
   * The previous underlay's GPU buffers are freed.
   */
  setUnderlay(items: IndexedBatch | null): void {
    const previous = this.currentUnderlay
    if (previous && previous.batch !== items?.batch) {
      this.activeRenderer.release(previous.batch)
    }
    this.currentUnderlay = items
    this.invalidate("base")
  }

  /**
   * Shows selected items moved by this much without changing the batch.
   * Commit the move to the project when the drag ends, then pass (0, 0).
   */
  setDragOffset(ticks: number, rows: number): void {
    if (ticks === this.dragTicks && rows === this.dragRows) return
    this.dragTicks = ticks
    this.dragRows = rows
    this.invalidate("all")
  }

  /**
   * Shows selected items with an edge moved by this many ticks without
   * changing the batch. Each item becomes what `resizedSpan` returns for
   * it. Commit the resize when the drag ends, then pass (0, 0).
   */
  setDragResize(startTicks: number, endTicks: number, minLength = 1): void {
    if (
      startTicks === this.resizeStart &&
      endTicks === this.resizeEnd &&
      minLength === this.resizeMinLength
    ) {
      return
    }
    this.resizeStart = startTicks
    this.resizeEnd = endTicks
    this.resizeMinLength = minLength
    this.invalidate("all")
  }

  setMarquee(marquee: Marquee | null): void {
    this.marquee = marquee
    this.invalidate("overlay")
  }

  /** Call from the realtime feed. Redraws the overlay only. */
  setPlayhead(tick: number | null): void {
    if (tick === this.playheadTick) return
    this.playheadTick = tick
    this.invalidate("overlay")
  }

  addOverlayPainter(painter: OverlayPainter): () => void {
    this.overlayPainters.push(painter)
    this.invalidate("overlay")
    return () => {
      this.overlayPainters = this.overlayPainters.filter((p) => p !== painter)
      this.invalidate("overlay")
    }
  }

  /** Item colors come from the theme, so rebuild them in the listener. */
  onThemeChange(listener: (theme: GridTheme) => void): () => void {
    this.themeListeners.push(listener)
    return () => {
      this.themeListeners = this.themeListeners.filter((l) => l !== listener)
    }
  }

  onViewportChange(listener: (viewport: Viewport) => void): () => void {
    this.viewportListeners.push(listener)
    return () => {
      this.viewportListeners = this.viewportListeners.filter(
        (l) => l !== listener
      )
    }
  }

  /**
   * Marks a layer for redraw, both by default. Call it after mutating the
   * item batch in place, or after a change to something only an overlay
   * painter knows about.
   */
  invalidate(layer: Layer = "all"): void {
    if (layer !== "overlay") this.baseDirty = true
    if (layer !== "base") this.overlayDirty = true
    if (this.autoRender && this.frameRequest === 0 && !this.destroyed) {
      this.frameRequest = requestAnimationFrame(() => {
        this.frameRequest = 0
        this.flush()
      })
    }
  }

  /** Draws whatever is dirty right now. Returns null when nothing was. */
  flush(): FrameStats | null {
    if (this.destroyed) return null
    if (this.themeDirty) this.refreshTheme()
    if (!this.baseDirty && !this.overlayDirty) return null
    const started = performance.now()
    const base = this.baseDirty
    const overlay = this.overlayDirty
    this.baseDirty = false
    this.overlayDirty = false
    const transform = this.transform
    const itemsInRange = base ? this.drawBase(transform) : 0
    if (overlay) this.drawOverlay(transform)
    const stats: FrameStats = {
      base,
      overlay,
      cpuMs: performance.now() - started,
      itemsInRange,
    }
    this.onFrame?.(stats)
    return stats
  }

  /** Converts a pointer event position to CSS pixels inside the view. */
  localPoint(event: { clientX: number; clientY: number }): {
    x: number
    y: number
  } {
    const bounds = this.container.getBoundingClientRect()
    return {
      x: event.clientX - bounds.left - this.container.clientLeft,
      y: event.clientY - bounds.top - this.container.clientTop,
    }
  }

  hitTest(x: number, y: number, options?: HitOptions): Hit | null {
    return this.currentItems
      ? hitTestPoint(this.currentViewport, this.currentItems, x, y, options)
      : null
  }

  destroy(): void {
    if (this.destroyed) return
    this.destroyed = true
    if (this.frameRequest !== 0) cancelAnimationFrame(this.frameRequest)
    this.resizeObserver.disconnect()
    this.stopThemeObserver()
    this.activeRenderer.onRestored = null
    this.activeRenderer.onLost = null
    this.activeRenderer.dispose()
    this.baseCanvas.remove()
    this.drawCanvas.remove()
    this.overlayCanvas.remove()
    this.themeListeners = []
    this.viewportListeners = []
    this.overlayPainters = []
  }

  private refreshTheme(): void {
    this.themeDirty = false
    const tokens = readThemeTokens(this.container, this.parseColor)
    // Any class change on the root element lands here; most are not a
    // theme change.
    if (JSON.stringify(tokens) === JSON.stringify(this.tokens)) return
    this.tokens = tokens
    this.currentTheme = deriveGridTheme(tokens)
    this.activeRenderer.setTheme(this.currentTheme)
    this.baseDirty = true
    this.overlayDirty = true
    for (const listener of this.themeListeners) listener(this.currentTheme)
  }

  private handleResize(entry: ResizeObserverEntry): void {
    const dpr = window.devicePixelRatio || 1
    const width = this.container.clientWidth
    const height = this.container.clientHeight
    const size = backingSize(
      width,
      height,
      dpr,
      entry.devicePixelContentBoxSize?.[0]
    )
    const resized = this.resizeCanvases(size.width, size.height)
    this.setViewport({ ...this.currentViewport, width, height, dpr })
    if (resized) {
      this.invalidate("all")
      // Resizing clears a canvas. Drawing before the browser paints keeps
      // a drag-resize from flickering.
      this.flush()
    }
  }

  private resizeCanvases(widthDev: number, heightDev: number): boolean {
    const width = Math.max(1, widthDev)
    const height = Math.max(1, heightDev)
    if (
      this.drawCanvas.width === width &&
      this.drawCanvas.height === height &&
      this.overlayCanvas.width === width &&
      this.overlayCanvas.height === height
    ) {
      return false
    }
    this.activeRenderer.resize(width, height)
    this.overlayCanvas.width = width
    this.overlayCanvas.height = height
    return true
  }

  private adopt(renderer: RectRenderer): void {
    renderer.setTheme(this.currentTheme)
    renderer.onRestored = () => this.invalidate("all")
    renderer.onLost = () => this.replaceLostRenderer()
  }

  /**
   * Carries on in Canvas 2D after a GPU renderer lost its context for good.
   * The canvas that takes the pointer stays in place, so listeners on
   * `element` keep working. It turns see-through and the new canvas goes
   * under it.
   */
  private replaceLostRenderer(): void {
    const lost = this.activeRenderer
    if (this.destroyed || lost.info.kind === "canvas2d") return
    const canvas = document.createElement("canvas")
    let renderer: RectRenderer
    try {
      renderer = createCanvas2DRenderer(canvas)
    } catch {
      // No 2D context either. The view stays blank, as it already is.
      return
    }
    renderer.resize(this.drawCanvas.width, this.drawCanvas.height)
    styleCanvas(canvas, false)
    this.baseCanvas.before(canvas)
    this.baseCanvas.style.opacity = "0"
    lost.onRestored = null
    lost.onLost = null
    lost.dispose()
    this.drawCanvas = canvas
    this.activeRenderer = renderer
    this.adopt(renderer)
    this.invalidate("all")
  }

  private drawBase(transform: DeviceTransform): number {
    const viewport = this.currentViewport
    const renderer = this.activeRenderer
    writeGrid(
      this.gridBatch,
      viewport,
      this.timeGrid,
      this.rows,
      this.currentTheme
    )
    renderer.beginFrame(transform)
    renderer.drawBatch(this.gridBatch)
    const ticks = visibleTicks(viewport)
    const underlay = this.currentUnderlay
    if (underlay && underlay.batch.count > 0) {
      const range = visibleRange(underlay, ticks.start, ticks.end)
      renderer.drawBatch(underlay.batch, range)
    }
    let itemsInRange = 0
    const items = this.currentItems
    if (items && items.batch.count > 0) {
      const dragging = items.batch.selectedCount > 0
      const dragTicks = dragging ? this.dragTicks : 0
      const resizeStart = dragging ? this.resizeStart : 0
      const resizeEnd = dragging ? this.resizeEnd : 0
      // A dragged item is drawn at start + dragTicks, and a resized one
      // reaches further than it is stored, so the range has to take in the
      // items that are being dragged or stretched into view.
      const range = visibleRange(
        items,
        Math.min(ticks.start, ticks.start - dragTicks - Math.max(0, resizeEnd)),
        Math.max(ticks.end, ticks.end - dragTicks - Math.min(0, resizeStart))
      )
      itemsInRange = range.last - range.first
      renderer.drawBatch(items.batch, {
        first: range.first,
        last: range.last,
        dragTicks,
        dragRows: dragging ? this.dragRows : 0,
        resizeStart,
        resizeEnd,
        minLength: this.resizeMinLength,
      })
    }
    renderer.endFrame()
    return itemsInRange
  }

  private drawOverlay(transform: DeviceTransform): void {
    const ctx = this.overlayContext
    if (!ctx) return
    const width = this.overlayCanvas.width
    const height = this.overlayCanvas.height
    const theme = this.currentTheme
    ctx.setTransform(1, 0, 0, 1, 0, 0)
    ctx.clearRect(0, 0, width, height)

    if (this.overlayPainters.length > 0) {
      const frame: OverlayFrame = {
        viewport: this.currentViewport,
        transform,
        theme,
      }
      for (const painter of this.overlayPainters) {
        ctx.save()
        painter(ctx, frame)
        ctx.restore()
      }
    }

    const marquee = this.marquee
    if (marquee) {
      const lw = transform.lineWidth
      const x0 = deviceX(transform, Math.min(marquee.tick0, marquee.tick1))
      const x1 = deviceX(transform, Math.max(marquee.tick0, marquee.tick1))
      const y0 = deviceY(transform, Math.min(marquee.row0, marquee.row1))
      const y1 = deviceY(transform, Math.max(marquee.row0, marquee.row1))
      const w = Math.max(lw, x1 - x0)
      const h = Math.max(lw, y1 - y0)
      ctx.fillStyle = rgbaToCss(theme.marqueeFill)
      ctx.fillRect(x0, y0, w, h)
      ctx.fillStyle = rgbaToCss(theme.marqueeStroke)
      ctx.fillRect(x0, y0, w, lw)
      ctx.fillRect(x0, y0 + h - lw, w, lw)
      ctx.fillRect(x0, y0, lw, h)
      ctx.fillRect(x0 + w - lw, y0, lw, h)
    }

    if (this.playheadTick !== null) {
      const thickness = Math.max(1, Math.round(1.5 * this.currentViewport.dpr))
      const x = deviceX(transform, this.playheadTick)
      if (x + thickness > 0 && x < width) {
        ctx.fillStyle = rgbaToCss(theme.playhead)
        ctx.fillRect(x, 0, thickness, height)
      }
    }
  }
}
