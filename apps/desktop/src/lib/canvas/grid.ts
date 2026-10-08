import {
  RECT_FLAT,
  RECT_FULL_HEIGHT,
  RECT_FULL_WIDTH,
  RECT_HLINE,
  RECT_VLINE,
  type RectBatch,
} from "./rect-batch"
import type { GridTheme } from "./theme"
import {
  isBlackKey,
  rowToKey,
  visibleRows,
  visibleTicks,
  type Viewport,
} from "./viewport"

export interface TimeGridSpec {
  readonly ticksPerStep: number
  readonly stepsPerBeat: number
  readonly beatsPerBar: number
  readonly segments?: readonly (Omit<TimeGridSpec, "segments"> & { start: number; end: number })[]
}

/** 4/4 at 960 PPQ with sixteenth-note steps. */
export const DEFAULT_TIME_GRID: TimeGridSpec = {
  ticksPerStep: 240,
  stepsPerBeat: 4,
  beatsPerBar: 4,
}

export interface RowStyle {
  readonly rowCount: number
  /** 1 where the row is set apart by shading (the darker rows). */
  readonly shaded: Uint8Array | null
  /** 1 where the line above the row is a group boundary. */
  readonly strong: Uint8Array | null
}

/** Tick spacing of the three line weights drawn at the current zoom. */
export interface GridLevels {
  readonly minor: number
  readonly mid: number
  readonly strong: number
}

/** Lines closer than this many CSS pixels are dropped. */
export const MIN_LINE_SPACING_PX = 7
const MIN_ROW_LINE_HEIGHT_PX = 6

/**
 * Picks which lines to draw. Zoomed in, that is steps, beats and bars.
 * Zooming out drops the finest level and promotes the rest, continuing
 * past bars in groups of four, so the grid never turns into a solid fill.
 */
export function gridLevels(
  pxPerTick: number,
  spec: TimeGridSpec,
  minSpacingPx = MIN_LINE_SPACING_PX
): GridLevels {
  let minor = spec.ticksPerStep
  let mid = minor * spec.stepsPerBeat
  let strong = mid * spec.beatsPerBar
  // The bound ends the loop at a zoom of zero.
  while (minor * pxPerTick < minSpacingPx && minor < 2 ** 40) {
    minor = mid
    mid = strong
    strong *= 4
  }
  return { minor, mid, strong }
}

/** Rows for a piano roll: black keys shaded, a strong line above each C. */
export function pianoRows(rowCount = 128): RowStyle {
  const shaded = new Uint8Array(rowCount)
  const strong = new Uint8Array(rowCount)
  for (let row = 0; row < rowCount; row++) {
    const key = rowToKey(row, rowCount)
    shaded[row] = isBlackKey(key) ? 1 : 0
    strong[row] = key % 12 === 11 ? 1 : 0
  }
  return { rowCount, shaded, strong }
}

/** Plain rows with no shading, for a playlist. */
export function plainRows(rowCount: number): RowStyle {
  return { rowCount, shaded: null, strong: null }
}

/**
 * Rewrites `out` with the grid for one frame: row shading, row lines, then
 * time lines. Only what is on screen is emitted, a few hundred rects at
 * most, so rebuilding it every frame costs little.
 */
export function writeGrid(
  out: RectBatch,
  viewport: Viewport,
  spec: TimeGridSpec,
  rows: RowStyle,
  theme: GridTheme
): void {
  out.clear()
  const rowRange = visibleRows(viewport, rows.rowCount)
  const flatRow = RECT_FLAT | RECT_FULL_WIDTH

  if (rows.shaded) {
    const shadeMarked = !theme.rowShadeOnUnmarked
    for (let row = rowRange.first; row < rowRange.last; row++) {
      if ((rows.shaded[row] === 1) === shadeMarked) {
        out.push(-1, 0, 0, row, 1, theme.rowShade, flatRow)
      }
    }
  }

  const drawRowLines = viewport.rowHeight >= MIN_ROW_LINE_HEIGHT_PX
  // One past the last row closes the grid at the bottom.
  for (let row = rowRange.first; row <= rowRange.last; row++) {
    const strong = rows.strong !== null && rows.strong[row] === 1
    if (!strong && !drawRowLines) continue
    out.push(
      -1,
      0,
      0,
      row,
      0,
      strong ? theme.rowLineStrong : theme.rowLine,
      flatRow | RECT_HLINE
    )
  }

  const ticks = visibleTicks(viewport)
  const flatColumn = RECT_FLAT | RECT_FULL_HEIGHT | RECT_VLINE
  for (const segment of spec.segments ?? [{ ...spec, start: 0, end: Infinity }]) {
    const levels = gridLevels(viewport.pxPerTick, segment)
    const first = segment.start + Math.max(0, Math.ceil((ticks.start - segment.start) / levels.minor)) * levels.minor
    for (let tick = first; tick <= ticks.end && tick < segment.end; tick += levels.minor) {
      const offset = tick - segment.start
      let color = theme.gridMinor
      if (offset % levels.strong === 0) color = theme.gridBar
      else if (offset % levels.mid === 0) color = theme.gridBeat
      out.push(-1, tick, 0, 0, 0, color, flatColumn)
    }
  }
}
