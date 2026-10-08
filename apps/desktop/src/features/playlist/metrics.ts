import { logicalWheel } from "@/lib/ui-scale"
import { useSyncExternalStore } from "react"

import {
  clampViewport,
  DEFAULT_LIMITS,
  scrollByPx,
  yToRow,
  zoomTimeAt,
  type Viewport,
  type ViewportLimits,
} from "@/lib/canvas"
import { onProjectReplaced } from "@/lib/store/replaced"
import { clamp } from "@/lib/units"

import {
  DEFAULT_BAR_WIDTH,
  DEFAULT_ROW_HEIGHT,
  MAX_PX_PER_TICK,
  MAX_ROW_HEIGHT,
  MIN_PX_PER_TICK,
  MIN_ROW_HEIGHT,
  MIN_ROWS,
  MIN_SONG_BARS,
  TALL_ROW_HEIGHT,
} from "./layout"
import type { GridSurface } from "./surface"

const FOUR_FOUR_BAR = 3840

export const PLAYLIST_LIMITS: ViewportLimits = {
  ...DEFAULT_LIMITS,
  rowCount: MIN_ROWS,
  contentTicks: MIN_SONG_BARS * FOUR_FOUR_BAR,
  minPxPerTick: MIN_PX_PER_TICK,
  maxPxPerTick: MAX_PX_PER_TICK,
  minRowHeight: MIN_ROW_HEIGHT,
  maxRowHeight: MAX_ROW_HEIGHT,
}

type SavedView = Pick<
  Viewport,
  "scrollTick" | "scrollRow" | "pxPerTick" | "rowHeight"
>

const FRESH_VIEW: SavedView = {
  scrollTick: 0,
  scrollRow: 0,
  pxPerTick: DEFAULT_BAR_WIDTH / FOUR_FOUR_BAR,
  rowHeight: DEFAULT_ROW_HEIGHT,
}

// The panel is unmounted when another tab shows. This keeps the scroll and
// zoom for when it comes back.
let savedView: SavedView = FRESH_VIEW
// Goes up when the kept view is forgotten. A panel from before that has
// nothing to add to it any more.
let savedEpoch = 0

export function initialView(): SavedView {
  return savedView
}

/** Forgets the scroll and zoom kept from an earlier mount. */
export function resetSavedView(): void {
  savedView = FRESH_VIEW
  savedEpoch += 1
}

// Another song starts at its beginning, at the usual zoom.
onProjectReplaced(resetSavedView)

export type WheelInput = {
  /** CSS pixels from the grid's top left corner. */
  x: number
  y: number
  deltaX: number
  deltaY: number
  /** Ctrl, or Cmd on macOS. */
  mod: boolean
  shift: boolean
  alt: boolean
}

/** A wheel event's turn in pixels, whatever unit the browser reported it in. */
export function wheelInput(
  event: WheelEvent,
  point: { x: number; y: number }
): WheelInput {
  return {
    ...point,
    ...logicalWheel(event),
    mod: event.ctrlKey || event.metaKey,
    shift: event.shiftKey,
    alt: event.altKey,
  }
}

/**
 * The grid's viewport, for the parts around the canvas: the ruler, the
 * track headers and the scrollbars read it and scroll through it. Before
 * the canvas exists, and where it cannot be created at all, it holds a
 * viewport of its own, so those parts work the same.
 */
export class GridMetrics {
  private surface: GridSurface | null = null
  private current: Viewport
  private currentLimits: ViewportLimits = PLAYLIST_LIMITS
  private readonly listeners = new Set<() => void>()
  private readonly epoch = savedEpoch

  constructor() {
    this.current = clampViewport(
      { width: 800, height: 360, dpr: 1, ...savedView },
      this.currentLimits
    )
  }

  get viewport(): Viewport {
    return this.surface?.viewport ?? this.current
  }

  get limits(): ViewportLimits {
    return this.surface?.limits ?? this.currentLimits
  }

  /** Follows a canvas from now on. Returns a function that lets go of it. */
  attach(surface: GridSurface): () => void {
    this.surface = surface
    surface.setLimits(this.currentLimits)
    const stop = surface.onViewportChange(() => this.changed())
    this.changed()
    return () => {
      stop()
      if (this.surface !== surface) return
      this.current = surface.viewport
      this.currentLimits = surface.limits
      this.surface = null
    }
  }

  setViewport(next: Viewport): void {
    if (this.surface) {
      // The surface tells its listeners, and this is one of them.
      this.surface.setViewport(next)
      return
    }
    this.current = clampViewport(next, this.currentLimits)
    this.changed()
  }

  setLimits(limits: Partial<ViewportLimits>): void {
    if (this.surface) {
      this.surface.setLimits(limits)
    } else {
      this.currentLimits = { ...this.currentLimits, ...limits }
      this.current = clampViewport(this.current, this.currentLimits)
    }
    this.changed()
  }

  panBy(dx: number, dy: number): void {
    this.setViewport(scrollByPx(this.viewport, dx, dy, this.limits))
  }

  zoomTime(anchorX: number, factor: number): void {
    this.setViewport(zoomTimeAt(this.viewport, anchorX, factor, this.limits))
  }

  /** Fits absolute dragged ticks in the current logical CSS viewport. */
  fitRegion(range: { start: number; end: number }): void {
    if (
      !Number.isFinite(range.start) ||
      !Number.isFinite(range.end) ||
      range.start < 0 ||
      range.end <= range.start
    )
      return
    const viewport = this.viewport
    const padding = Math.min(24, viewport.width / 10)
    const pxPerTick = clamp(
      (viewport.width - padding * 2) / (range.end - range.start),
      this.limits.minPxPerTick,
      this.limits.maxPxPerTick
    )
    this.setLimits({
      contentTicks: Math.max(
        this.limits.contentTicks,
        range.end + padding / pxPerTick
      ),
    })
    this.setViewport({
      ...viewport,
      pxPerTick,
      scrollTick: Math.max(0, range.start - padding / pxPerTick),
    })
  }

  /**
   * Makes rows taller or shorter around the row under `anchorY`. Heights
   * stay whole pixels so the track headers, which the browser lays out,
   * line up with the rows the canvas draws.
   */
  zoomRows(anchorY: number, factor: number): void {
    const viewport = this.viewport
    const limits = this.limits
    const scaled = Math.round(viewport.rowHeight * factor)
    const stepped =
      scaled === viewport.rowHeight
        ? viewport.rowHeight + Math.sign(factor - 1)
        : scaled
    const rowHeight = clamp(stepped, limits.minRowHeight, limits.maxRowHeight)
    if (rowHeight === viewport.rowHeight) return
    const anchorRow = yToRow(viewport, anchorY)
    this.setViewport({
      ...viewport,
      rowHeight,
      scrollRow: anchorRow - anchorY / rowHeight,
    })
  }

  /** True when the rows are tall enough to draw automation curves in. */
  get tall(): boolean {
    return this.viewport.rowHeight >= TALL_ROW_HEIGHT
  }

  /**
   * Switches between the usual row height and the tall one, keeping the
   * row at the top of the view where it is.
   */
  toggleTall(): void {
    const viewport = this.viewport
    this.setViewport({
      ...viewport,
      rowHeight: this.tall ? DEFAULT_ROW_HEIGHT : TALL_ROW_HEIGHT,
      scrollRow: Math.floor(viewport.scrollRow),
    })
  }

  /**
   * The wheel, the same over the grid, the ruler and the track headers:
   * it scrolls tracks, Shift scrolls time, Ctrl zooms time around the
   * pointer, and Ctrl+Shift or Alt makes tracks taller or shorter.
   */
  wheel(input: WheelInput): void {
    const { x, y, deltaX, deltaY, mod, shift, alt } = input
    if ((mod && shift) || alt) {
      // Shift turns a vertical wheel into a horizontal one on some systems.
      this.zoomRows(y, Math.exp(-(deltaY || deltaX) * 0.002))
    } else if (mod) {
      this.zoomTime(x, Math.exp(-deltaY * 0.0015))
    } else if (shift) {
      this.panBy(deltaY + deltaX, 0)
    } else {
      this.panBy(deltaX, deltaY)
    }
  }

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener)
    return () => {
      this.listeners.delete(listener)
    }
  }

  private changed(): void {
    if (this.epoch === savedEpoch) {
      const { scrollTick, scrollRow, pxPerTick, rowHeight } = this.viewport
      savedView = { scrollTick, scrollRow, pxPerTick, rowHeight }
    }
    for (const listener of [...this.listeners]) listener()
  }
}

/**
 * A value worked out from the viewport. The component renders again only
 * when the value changes, so keep it a number, a string or a boolean.
 */
export function useViewportValue<T extends number | string | boolean>(
  metrics: GridMetrics,
  select: (viewport: Viewport, limits: ViewportLimits) => T
): T {
  const read = () => select(metrics.viewport, metrics.limits)
  return useSyncExternalStore(metrics.subscribe, read, read)
}
