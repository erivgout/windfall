import { describe, expect, it } from "vitest"

import { rgba } from "./color"
import {
  DEFAULT_TIME_GRID,
  gridLevels,
  pianoRows,
  plainRows,
  writeGrid,
} from "./grid"
import {
  RECT_FULL_HEIGHT,
  RECT_FULL_WIDTH,
  RECT_HLINE,
  RECT_VLINE,
  RectBatch,
} from "./rect-batch"
import { deriveGridTheme } from "./theme"
import type { Viewport } from "./viewport"

const theme = deriveGridTheme({
  background: rgba(20, 20, 20),
  foreground: rgba(250, 250, 250),
  mutedForeground: rgba(160, 160, 160),
  brand: rgba(230, 60, 140),
  playhead: rgba(90, 160, 255),
  gridLine: rgba(255, 255, 255, 18),
  gridLineStrong: rgba(255, 255, 255, 51),
})

const viewport: Viewport = {
  width: 1920,
  height: 1080,
  dpr: 1,
  scrollTick: 3840,
  scrollRow: 30,
  pxPerTick: 0.0625,
  rowHeight: 16,
}

function verticalLines(batch: RectBatch): { tick: number; color: number }[] {
  const lines: { tick: number; color: number }[] = []
  for (let i = 0; i < batch.count; i++) {
    if (batch.flags[i] & RECT_VLINE) {
      lines.push({ tick: batch.start(i), color: batch.colors[i * 4 + 3] })
    }
  }
  return lines
}

describe("gridLevels", () => {
  it("shows steps, beats and bars when zoomed in", () => {
    expect(gridLevels(0.0625, DEFAULT_TIME_GRID)).toEqual({
      minor: 240,
      mid: 960,
      strong: 3840,
    })
  })

  it("drops steps once they are closer than the minimum spacing", () => {
    // A step is 6 px here, a beat 24 px.
    expect(gridLevels(0.025, DEFAULT_TIME_GRID)).toEqual({
      minor: 960,
      mid: 3840,
      strong: 15_360,
    })
  })

  it("keeps thinning out past bars", () => {
    // The whole 200-bar song in 1920 px: a bar is 9.6 px.
    expect(gridLevels(0.0025, DEFAULT_TIME_GRID)).toEqual({
      minor: 3840,
      mid: 15_360,
      strong: 61_440,
    })
    const far = gridLevels(0.0001, DEFAULT_TIME_GRID)
    expect(far.minor * 0.0001).toBeGreaterThanOrEqual(7)
  })

  it("follows the time signature", () => {
    const waltz = { ticksPerStep: 240, stepsPerBeat: 4, beatsPerBar: 3 }
    expect(gridLevels(0.0625, waltz).strong).toBe(2880)
  })

  it("terminates at a zoom of zero", () => {
    expect(gridLevels(0, DEFAULT_TIME_GRID).minor).toBeGreaterThan(0)
  })
})

describe("pianoRows", () => {
  it("shades black keys and marks octaves", () => {
    const rows = pianoRows(128)
    // Row 127 is key 0 (C), row 126 is key 1 (C sharp).
    expect(rows.shaded?.[127]).toBe(0)
    expect(rows.shaded?.[126]).toBe(1)
    // The line above B (key 11, row 116) separates it from the next C.
    expect(rows.strong?.[116]).toBe(1)
    expect(rows.strong?.[115]).toBe(0)
    expect(rows.shaded?.reduce((sum, v) => sum + v, 0)).toBe(53)
  })
})

describe("writeGrid", () => {
  it("emits time lines only inside the visible range", () => {
    const batch = new RectBatch()
    writeGrid(batch, viewport, DEFAULT_TIME_GRID, pianoRows(128), theme)
    const lines = verticalLines(batch)
    // 30,720 visible ticks at one line per step, both ends included.
    expect(lines).toHaveLength(129)
    expect(lines[0].tick).toBe(3840)
    expect(lines[lines.length - 1].tick).toBe(3840 + 30_720)
  })

  it("gives bars, beats and steps different weights", () => {
    const batch = new RectBatch()
    writeGrid(batch, viewport, DEFAULT_TIME_GRID, pianoRows(128), theme)
    const alpha = new Map(verticalLines(batch).map((l) => [l.tick, l.color]))
    expect(alpha.get(7680)).toBe(theme.gridBar.a)
    expect(alpha.get(7680 + 960)).toBe(theme.gridBeat.a)
    expect(alpha.get(7680 + 240)).toBe(theme.gridMinor.a)
    expect(theme.gridBar.a).toBeGreaterThan(theme.gridBeat.a)
    expect(theme.gridBeat.a).toBeGreaterThan(theme.gridMinor.a)
  })

  it("starts on a grid line when the scroll position is between lines", () => {
    const batch = new RectBatch()
    writeGrid(
      batch,
      { ...viewport, scrollTick: 3841.5 },
      DEFAULT_TIME_GRID,
      pianoRows(128),
      theme
    )
    expect(verticalLines(batch)[0].tick).toBe(4080)
  })

  it("emits shading and lines only for visible rows", () => {
    const batch = new RectBatch()
    writeGrid(batch, viewport, DEFAULT_TIME_GRID, pianoRows(128), theme)
    let shaded = 0
    let rowLines = 0
    for (let i = 0; i < batch.count; i++) {
      const flags = batch.flags[i]
      if (flags & RECT_VLINE) {
        expect(flags & RECT_FULL_HEIGHT).toBeTruthy()
        continue
      }
      expect(flags & RECT_FULL_WIDTH).toBeTruthy()
      expect(batch.row(i)).toBeGreaterThanOrEqual(30)
      expect(batch.row(i)).toBeLessThanOrEqual(98)
      if (flags & RECT_HLINE) rowLines++
      else shaded++
    }
    // Rows 30 to 97 are visible: 68 rows, 69 lines counting the closing one.
    expect(rowLines).toBe(69)
    // 29 of those rows are black keys. The dark theme's shade is lighter
    // than its background, so it goes on the other 39.
    expect(theme.rowShadeOnUnmarked).toBe(true)
    expect(shaded).toBe(39)
  })

  it("keeps black-key rows the darker ones in the light theme too", () => {
    const light = deriveGridTheme({
      background: rgba(255, 255, 255),
      foreground: rgba(20, 20, 20),
      mutedForeground: rgba(120, 120, 120),
      brand: rgba(214, 51, 132),
      playhead: rgba(40, 110, 220),
      gridLine: rgba(0, 0, 0, 20),
      gridLineStrong: rgba(0, 0, 0, 56),
    })
    expect(light.rowShadeOnUnmarked).toBe(false)
    const batch = new RectBatch()
    writeGrid(batch, viewport, DEFAULT_TIME_GRID, pianoRows(128), light)
    let shaded = 0
    for (let i = 0; i < batch.count; i++) {
      const flags = batch.flags[i]
      if (flags & (RECT_VLINE | RECT_HLINE)) continue
      shaded++
      // Row 126 is C sharp 0; every shaded row must be a black key.
      expect([1, 3, 6, 8, 10]).toContain((127 - batch.row(i)) % 12)
    }
    expect(shaded).toBe(29)
  })

  it("stays a few hundred rects at any zoom", () => {
    const batch = new RectBatch()
    for (const pxPerTick of [0.0005, 0.0025, 0.01, 0.0625, 0.5, 2]) {
      writeGrid(
        batch,
        { ...viewport, scrollTick: 0, scrollRow: 0, rowHeight: 8.5, pxPerTick },
        DEFAULT_TIME_GRID,
        pianoRows(128),
        theme
      )
      expect(batch.count).toBeLessThan(128 + 129 + 1920 / 7 + 2)
    }
  })

  it("draws only strong row lines when rows are too short for all of them", () => {
    const batch = new RectBatch()
    writeGrid(
      batch,
      { ...viewport, scrollRow: 0, rowHeight: 4 },
      DEFAULT_TIME_GRID,
      pianoRows(128),
      theme
    )
    let rowLines = 0
    for (let i = 0; i < batch.count; i++) {
      if (batch.flags[i] & RECT_HLINE) rowLines++
    }
    expect(rowLines).toBe(10)
  })

  it("draws plain rows without shading", () => {
    const batch = new RectBatch()
    writeGrid(batch, viewport, DEFAULT_TIME_GRID, plainRows(128), theme)
    for (let i = 0; i < batch.count; i++) {
      const flags = batch.flags[i]
      expect((flags & RECT_VLINE) !== 0 || (flags & RECT_HLINE) !== 0).toBe(
        true
      )
    }
  })

  it("reuses the batch between frames", () => {
    const batch = new RectBatch()
    writeGrid(batch, viewport, DEFAULT_TIME_GRID, pianoRows(128), theme)
    const first = batch.count
    const version = batch.geometryVersion
    writeGrid(batch, viewport, DEFAULT_TIME_GRID, pianoRows(128), theme)
    expect(batch.count).toBe(first)
    expect(batch.geometryVersion).toBeGreaterThan(version)
  })
})
