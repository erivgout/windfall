import { describe, expect, it } from "vitest"

import {
  DEFAULT_LIMITS,
  backingSize,
  clampViewport,
  deviceTransform,
  deviceX,
  deviceY,
  isBlackKey,
  keyToRow,
  rowToKey,
  rowToY,
  scrollByPx,
  snapTick,
  tickToX,
  visibleRows,
  visibleTicks,
  xToTick,
  yToRow,
  zoomRowsAt,
  zoomTimeAt,
  type Viewport,
  type ViewportLimits,
} from "./viewport"

const base: Viewport = {
  width: 1920,
  height: 1080,
  dpr: 1,
  scrollTick: 3840,
  scrollRow: 30,
  pxPerTick: 0.0625,
  rowHeight: 16,
}

const limits: ViewportLimits = {
  ...DEFAULT_LIMITS,
  contentTicks: 3840 * 200,
}

describe("coordinate conversion", () => {
  it("maps ticks to x and back", () => {
    expect(tickToX(base, 3840)).toBe(0)
    expect(tickToX(base, 3840 + 240)).toBe(15)
    expect(xToTick(base, 15)).toBe(3840 + 240)
    for (const tick of [0, 1, 959, 100_000, 767_999]) {
      expect(xToTick(base, tickToX(base, tick))).toBeCloseTo(tick, 6)
    }
  })

  it("maps rows to y and back", () => {
    expect(rowToY(base, 30)).toBe(0)
    expect(rowToY(base, 31)).toBe(16)
    expect(yToRow(base, 24)).toBe(31.5)
    expect(Math.floor(yToRow(base, 31.9))).toBe(31)
  })

  it("reports the visible tick and row ranges", () => {
    expect(visibleTicks(base)).toEqual({ start: 3840, end: 3840 + 30720 })
    // 1080 / 16 = 67.5 rows, so row 97 is partly visible.
    expect(visibleRows(base, 128)).toEqual({ first: 30, last: 98 })
    expect(visibleRows({ ...base, scrollRow: 100 }, 128)).toEqual({
      first: 100,
      last: 128,
    })
  })

  it("flips keys so the highest key is the top row", () => {
    expect(keyToRow(127)).toBe(0)
    expect(keyToRow(0)).toBe(127)
    expect(rowToKey(keyToRow(60))).toBe(60)
    expect(keyToRow(60, 88)).toBe(27)
  })

  it("knows the black keys", () => {
    const black = [1, 3, 6, 8, 10]
    for (let key = 0; key < 128; key++) {
      expect(isBlackKey(key)).toBe(black.includes(key % 12))
    }
  })

  it("snaps ticks to a grid", () => {
    expect(snapTick(119, 240)).toBe(0)
    expect(snapTick(120, 240)).toBe(240)
    expect(snapTick(-130, 240)).toBe(-240)
    expect(snapTick(12.4, 0)).toBe(12)
  })
})

describe("scrolling and zooming", () => {
  it("clamps scroll to the content", () => {
    const before = clampViewport(
      { ...base, scrollTick: -50, scrollRow: -3 },
      limits
    )
    expect(before.scrollTick).toBe(0)
    expect(before.scrollRow).toBe(0)

    const after = clampViewport(
      { ...base, scrollTick: 10_000_000, scrollRow: 500 },
      limits
    )
    expect(after.scrollTick).toBe(3840 * 200 - 30720)
    expect(after.scrollRow).toBe(128 - 67.5)
  })

  it("pins scroll at zero when the content is smaller than the view", () => {
    const tiny = clampViewport(
      { ...base, scrollTick: 500 },
      { ...limits, contentTicks: 1000 }
    )
    expect(tiny.scrollTick).toBe(0)
  })

  it("clamps zoom to the limits", () => {
    expect(clampViewport({ ...base, pxPerTick: 99 }, limits).pxPerTick).toBe(
      limits.maxPxPerTick
    )
    expect(clampViewport({ ...base, rowHeight: 1 }, limits).rowHeight).toBe(
      limits.minRowHeight
    )
  })

  it("scrolls by pixels", () => {
    const moved = scrollByPx(base, 150, -32, limits)
    expect(moved.scrollTick).toBe(3840 + 2400)
    expect(moved.scrollRow).toBe(28)
  })

  it("keeps the tick under the cursor fixed while zooming time", () => {
    const anchorX = 700
    const middle = { ...base, scrollTick: 100_000 }
    const tick = xToTick(middle, anchorX)
    for (const factor of [1.1, 2, 0.5, 0.9]) {
      const zoomed = zoomTimeAt(middle, anchorX, factor, limits)
      expect(zoomed.pxPerTick).toBeCloseTo(middle.pxPerTick * factor, 10)
      expect(tickToX(zoomed, tick)).toBeCloseTo(anchorX, 6)
    }
  })

  it("keeps the row under the cursor fixed while zooming rows", () => {
    const anchorY = 400
    const row = yToRow(base, anchorY)
    const zoomed = zoomRowsAt(base, anchorY, 1.5, limits)
    expect(zoomed.rowHeight).toBe(24)
    expect(rowToY(zoomed, row)).toBeCloseTo(anchorY, 6)
  })

  it("lets the anchor slide when zooming out hits the start of the content", () => {
    const atStart = { ...base, scrollTick: 0 }
    const zoomed = zoomTimeAt(atStart, 1900, 0.25, limits)
    expect(zoomed.scrollTick).toBe(0)
  })
})

describe("backingSize", () => {
  it("uses the observed device-pixel box when it matches the ratio", () => {
    // 1000.4 CSS px at 1.5x is 1500.6 device px, which the browser snaps
    // to 1501. Rounding the CSS size alone would give 1500.
    expect(
      backingSize(1000, 600, 1.5, { inlineSize: 1501, blockSize: 900 })
    ).toEqual({ width: 1501, height: 900 })
  })

  it("falls back to CSS size times ratio without a device-pixel box", () => {
    expect(backingSize(1920, 1080, 2)).toEqual({ width: 3840, height: 2160 })
    expect(backingSize(801, 601, 1.25)).toEqual({ width: 1001, height: 751 })
  })

  it("ignores a device-pixel box that contradicts the ratio", () => {
    // Device metrics emulation at 2x reports the unemulated box.
    expect(
      backingSize(1920, 1080, 2, { inlineSize: 1920, blockSize: 1080 })
    ).toEqual({ width: 3840, height: 2160 })
  })
})

describe("device transform", () => {
  it("puts ticks on whole device pixels", () => {
    const transform = deviceTransform({ ...base, scrollTick: 3841.37 })
    for (const tick of [3840, 4080, 5000, 34_560]) {
      expect(Number.isInteger(deviceX(transform, tick))).toBe(true)
    }
  })

  it("moves all content rigidly when scrolling", () => {
    // If rounding depended on the scroll position, lines would shimmer
    // against each other during a scroll.
    const ticks = [0, 240, 961, 3840, 7777, 30_000]
    const reference = deviceTransform({
      ...base,
      scrollTick: 0,
      pxPerTick: 0.0437,
    })
    const spacing = ticks.map(
      (t) => deviceX(reference, t) - deviceX(reference, 0)
    )
    for (const scrollTick of [0.4, 13.7, 999.99, 20_000.5, 123_456.789]) {
      const t = deviceTransform({ ...base, scrollTick, pxPerTick: 0.0437 })
      expect(ticks.map((tick) => deviceX(t, tick) - deviceX(t, 0))).toEqual(
        spacing
      )
    }
  })

  it("scales by the device pixel ratio", () => {
    const one = deviceTransform({ ...base, scrollTick: 0 })
    const two = deviceTransform({ ...base, scrollTick: 0, dpr: 2 })
    expect(deviceX(one, 240)).toBe(15)
    expect(deviceX(two, 240)).toBe(30)
    expect(two.widthDev).toBe(3840)
    expect(two.heightDev).toBe(2160)
    expect(one.lineWidth).toBe(1)
    expect(two.lineWidth).toBe(2)
    expect(deviceTransform({ ...base, dpr: 1.25 }).lineWidth).toBe(1)
    expect(deviceTransform({ ...base, dpr: 1.5 }).lineWidth).toBe(2)
  })

  it("puts row tops on whole device pixels at fractional zoom", () => {
    const transform = deviceTransform({
      ...base,
      dpr: 1.5,
      rowHeight: 12.27,
      scrollRow: 19.3,
    })
    let previous = deviceY(transform, 19)
    for (let row = 20; row < 110; row++) {
      const y = deviceY(transform, row)
      expect(Number.isInteger(y)).toBe(true)
      // Rows differ by at most one pixel, never collapse or overlap.
      expect(y - previous).toBeGreaterThanOrEqual(18)
      expect(y - previous).toBeLessThanOrEqual(19)
      previous = y
    }
  })

  it("stays exact in float32 far into a long project", () => {
    // The shaders do this arithmetic in float32. Subtracting the integer
    // scroll tick first is what keeps it exact ten million ticks in.
    const viewport = { ...base, scrollTick: 10_000_000.6, pxPerTick: 0.213 }
    const t = deviceTransform(viewport)
    for (const tick of [10_000_001, 10_000_240, 10_004_000, 10_008_999]) {
      const float32 = Math.floor(
        Math.fround(
          Math.fround(
            Math.fround(tick - t.scrollTick) * Math.fround(t.scaleX)
          ) - Math.fround(t.offsetX)
        ) + 0.5
      )
      expect(Math.abs(float32 - deviceX(t, tick))).toBeLessThanOrEqual(1)
      expect(
        Math.abs(deviceX(t, tick) - tickToX(viewport, tick))
      ).toBeLessThanOrEqual(1)
    }
  })
})
