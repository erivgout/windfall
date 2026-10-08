/**
 * The visible window of a time-grid editor. Horizontal units are ticks,
 * vertical units are rows (keys in the piano roll, tracks in the playlist).
 * Sizes are logical CSS pixels before application zoom. `dpr` is the
 * effective device density (monitor/browser DPR times application scale,
 * bounded by the common canvas allocation policy). Device coordinates
 * only appear in `DeviceTransform`.
 */
export interface Viewport {
  readonly width: number
  readonly height: number
  readonly dpr: number
  /** Tick at the left edge. May be fractional. */
  readonly scrollTick: number
  /** Row at the top edge. May be fractional. */
  readonly scrollRow: number
  /** CSS pixels per tick. */
  readonly pxPerTick: number
  /** CSS pixels per row. */
  readonly rowHeight: number
}

export interface ViewportLimits {
  readonly rowCount: number
  /** Length of the scrollable content in ticks. */
  readonly contentTicks: number
  readonly minPxPerTick: number
  readonly maxPxPerTick: number
  readonly minRowHeight: number
  readonly maxRowHeight: number
}

export interface TickRange {
  readonly start: number
  readonly end: number
}

export interface RowRange {
  /** First visible row, inclusive. */
  readonly first: number
  /** Last visible row, exclusive. */
  readonly last: number
}

export const DEFAULT_LIMITS: ViewportLimits = {
  rowCount: 128,
  contentTicks: 3840 * 64,
  minPxPerTick: 0.0005,
  maxPxPerTick: 2,
  minRowHeight: 4,
  maxRowHeight: 64,
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value))
}

export function tickToX(viewport: Viewport, tick: number): number {
  return (tick - viewport.scrollTick) * viewport.pxPerTick
}

export function xToTick(viewport: Viewport, x: number): number {
  return viewport.scrollTick + x / viewport.pxPerTick
}

export function rowToY(viewport: Viewport, row: number): number {
  return (row - viewport.scrollRow) * viewport.rowHeight
}

/** Fractional row under a y coordinate. Floor it for the row index. */
export function yToRow(viewport: Viewport, y: number): number {
  return viewport.scrollRow + y / viewport.rowHeight
}

export function visibleTicks(viewport: Viewport): TickRange {
  return {
    start: viewport.scrollTick,
    end: viewport.scrollTick + viewport.width / viewport.pxPerTick,
  }
}

export function visibleRows(viewport: Viewport, rowCount: number): RowRange {
  const first = clamp(Math.floor(viewport.scrollRow), 0, rowCount)
  const last = clamp(
    Math.ceil(viewport.scrollRow + viewport.height / viewport.rowHeight),
    first,
    rowCount
  )
  return { first, last }
}

export function clampViewport(
  viewport: Viewport,
  limits: ViewportLimits
): Viewport {
  const pxPerTick = clamp(
    viewport.pxPerTick,
    limits.minPxPerTick,
    limits.maxPxPerTick
  )
  const rowHeight = clamp(
    viewport.rowHeight,
    limits.minRowHeight,
    limits.maxRowHeight
  )
  const maxTick = Math.max(0, limits.contentTicks - viewport.width / pxPerTick)
  const maxRow = Math.max(0, limits.rowCount - viewport.height / rowHeight)
  return {
    ...viewport,
    pxPerTick,
    rowHeight,
    scrollTick: clamp(viewport.scrollTick, 0, maxTick),
    scrollRow: clamp(viewport.scrollRow, 0, maxRow),
  }
}

export function scrollByPx(
  viewport: Viewport,
  dx: number,
  dy: number,
  limits: ViewportLimits
): Viewport {
  return clampViewport(
    {
      ...viewport,
      scrollTick: viewport.scrollTick + dx / viewport.pxPerTick,
      scrollRow: viewport.scrollRow + dy / viewport.rowHeight,
    },
    limits
  )
}

/** Zooms time by `factor`, keeping the tick under `anchorX` in place. */
export function zoomTimeAt(
  viewport: Viewport,
  anchorX: number,
  factor: number,
  limits: ViewportLimits
): Viewport {
  const anchorTick = xToTick(viewport, anchorX)
  const pxPerTick = clamp(
    viewport.pxPerTick * factor,
    limits.minPxPerTick,
    limits.maxPxPerTick
  )
  return clampViewport(
    { ...viewport, pxPerTick, scrollTick: anchorTick - anchorX / pxPerTick },
    limits
  )
}

/** Zooms rows by `factor`, keeping the row under `anchorY` in place. */
export function zoomRowsAt(
  viewport: Viewport,
  anchorY: number,
  factor: number,
  limits: ViewportLimits
): Viewport {
  const anchorRow = yToRow(viewport, anchorY)
  const rowHeight = clamp(
    viewport.rowHeight * factor,
    limits.minRowHeight,
    limits.maxRowHeight
  )
  return clampViewport(
    { ...viewport, rowHeight, scrollRow: anchorRow - anchorY / rowHeight },
    limits
  )
}

/**
 * The viewport in device pixels, in the form every renderer consumes.
 *
 * The scroll origin is rounded to a whole device pixel once, so all content
 * moves rigidly when scrolling and a tick always lands on the same pixel
 * column relative to its neighbors. `scrollTick` is split off as an integer
 * so a GPU can subtract it from integer tick attributes exactly, before any
 * float32 multiply.
 */
export interface DeviceTransform {
  readonly widthDev: number
  readonly heightDev: number
  /** Integer tick subtracted from every tick before scaling. */
  readonly scrollTick: number
  /** Device pixels per tick. */
  readonly scaleX: number
  /** Device pixels subtracted after scaling. Small, so float32 holds it. */
  readonly offsetX: number
  /** Device pixels per row. */
  readonly scaleY: number
  readonly offsetY: number
  /** Thickness of a 1 CSS pixel line, in whole device pixels. */
  readonly lineWidth: number
}

export function deviceTransform(viewport: Viewport): DeviceTransform {
  const scaleX = viewport.pxPerTick * viewport.dpr
  const scaleY = viewport.rowHeight * viewport.dpr
  const scrollTick = Math.floor(viewport.scrollTick)
  const originX = Math.round(viewport.scrollTick * scaleX)
  return {
    widthDev: Math.round(viewport.width * viewport.dpr),
    heightDev: Math.round(viewport.height * viewport.dpr),
    scrollTick,
    scaleX,
    offsetX: originX - scrollTick * scaleX,
    scaleY,
    offsetY: Math.round(viewport.scrollRow * scaleY),
    lineWidth: Math.max(1, Math.round(viewport.dpr)),
  }
}

/** Device pixel column of a tick. Matches the shader's rounding. */
export function deviceX(transform: DeviceTransform, tick: number): number {
  return Math.floor(
    (tick - transform.scrollTick) * transform.scaleX - transform.offsetX + 0.5
  )
}

/** Device pixel row of the top of a grid row. */
export function deviceY(transform: DeviceTransform, row: number): number {
  return Math.floor(row * transform.scaleY - transform.offsetY + 0.5)
}

/**
 * Canvas backing size in device pixels.
 *
 * The device-pixel box a ResizeObserver reports is exact, where CSS pixels
 * times the ratio can be one pixel off at fractional scale factors and blur
 * every line. Under device metrics emulation (DevTools device mode,
 * Playwright) the box is the unemulated one and disagrees with the ratio by
 * far more than a pixel. Then the CSS size wins, so the backing store and
 * the transform still use the same ratio.
 */
export function backingSize(
  cssWidth: number,
  cssHeight: number,
  dpr: number,
  deviceBox?: { readonly inlineSize: number; readonly blockSize: number }
): { width: number; height: number } {
  const width = Math.round(cssWidth * dpr)
  const height = Math.round(cssHeight * dpr)
  // CSS sizes arrive rounded to whole pixels, so allow half a CSS pixel of
  // error on top of one device pixel of snapping.
  const tolerance = 1 + dpr / 2
  if (
    deviceBox &&
    Math.abs(deviceBox.inlineSize - width) <= tolerance &&
    Math.abs(deviceBox.blockSize - height) <= tolerance
  ) {
    return { width: deviceBox.inlineSize, height: deviceBox.blockSize }
  }
  return { width, height }
}

/** Rounds a tick to the nearest multiple of `grid` ticks. */
export function snapTick(tick: number, grid: number): number {
  return grid > 0 ? Math.round(tick / grid) * grid : Math.round(tick)
}

/** The piano roll puts the highest key on the top row. */
export function keyToRow(key: number, rowCount = 128): number {
  return rowCount - 1 - key
}

export function rowToKey(row: number, rowCount = 128): number {
  return rowCount - 1 - row
}

const BLACK_KEYS = [0, 1, 0, 1, 0, 0, 1, 0, 1, 0, 1, 0]

export function isBlackKey(key: number): boolean {
  return BLACK_KEYS[((key % 12) + 12) % 12] === 1
}
