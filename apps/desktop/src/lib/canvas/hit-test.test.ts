import { describe, expect, it } from "vitest"

import { rgba } from "./color"
import { hitTestPoint, hitTestRect } from "./hit-test"
import { RECT_SELECTED, RectBatch } from "./rect-batch"
import { indexBatch } from "./spatial-index"
import { keyToRow, type Viewport } from "./viewport"

const COLOR = rgba(200, 50, 120)

// 0.0625 px per tick: a step is 15 px, a beat 60 px. Row 60 starts at y = 0.
const viewport: Viewport = {
  width: 1920,
  height: 1080,
  dpr: 1,
  scrollTick: 3840,
  scrollRow: 60,
  pxPerTick: 0.0625,
  rowHeight: 16,
}

function items(
  rects: [
    id: number,
    start: number,
    length: number,
    row: number,
    flags?: number,
  ][]
) {
  const batch = new RectBatch()
  for (const [id, start, length, row, flags] of rects) {
    batch.push(id, start, length, row, 1, COLOR, flags ?? 0)
  }
  return indexBatch(batch)
}

describe("hitTestPoint", () => {
  // One beat long: x 60 to 120, y 32 to 48.
  const one = items([[7, 3840 + 960, 960, 62]])

  it("finds the rect under the pointer", () => {
    expect(hitTestPoint(viewport, one, 90, 40)).toEqual({
      index: 0,
      id: 7,
      part: "body",
    })
  })

  it("misses outside the rect", () => {
    expect(hitTestPoint(viewport, one, 90, 31)).toBeNull()
    expect(hitTestPoint(viewport, one, 90, 48.5)).toBeNull()
    expect(hitTestPoint(viewport, one, 50, 40)).toBeNull()
    expect(hitTestPoint(viewport, one, 130, 40)).toBeNull()
  })

  it("reports the resize handles at both ends", () => {
    expect(hitTestPoint(viewport, one, 61, 40)?.part).toBe("start-edge")
    expect(hitTestPoint(viewport, one, 65.9, 40)?.part).toBe("start-edge")
    expect(hitTestPoint(viewport, one, 66, 40)?.part).toBe("body")
    expect(hitTestPoint(viewport, one, 113.9, 40)?.part).toBe("body")
    expect(hitTestPoint(viewport, one, 114, 40)?.part).toBe("end-edge")
    expect(hitTestPoint(viewport, one, 119, 40)?.part).toBe("end-edge")
  })

  it("honors a custom handle width", () => {
    expect(hitTestPoint(viewport, one, 70, 40, { edgePx: 12 })?.part).toBe(
      "start-edge"
    )
    expect(hitTestPoint(viewport, one, 61, 40, { edgePx: 0 })?.part).toBe(
      "body"
    )
  })

  it("leaves a body on a short rect", () => {
    // A step is 15 px wide, so each handle shrinks to 5 px.
    const short = items([[1, 3840, 240, 60]])
    expect(hitTestPoint(viewport, short, 4, 8)?.part).toBe("start-edge")
    expect(hitTestPoint(viewport, short, 7, 8)?.part).toBe("body")
    expect(hitTestPoint(viewport, short, 11, 8)?.part).toBe("end-edge")
  })

  it("can still click a rect drawn narrower than a pixel", () => {
    const zoomedOut: Viewport = {
      ...viewport,
      scrollTick: 0,
      pxPerTick: 0.0025,
    }
    // 240 ticks is 0.6 px here, starting at x = 100.
    const tiny = items([[9, 40_000, 240, 60]])
    expect(hitTestPoint(zoomedOut, tiny, 101.5, 8)?.id).toBe(9)
    expect(hitTestPoint(zoomedOut, tiny, 99, 8)?.id).toBe(9)
    expect(hitTestPoint(zoomedOut, tiny, 104, 8)).toBeNull()
    expect(hitTestPoint(zoomedOut, tiny, 101.5, 20)).toBeNull()
    expect(hitTestPoint(zoomedOut, tiny, 101.5, 8, { slopPx: 0 })).toBeNull()
  })

  it("picks the selected rect where two overlap", () => {
    const stacked = items([
      [1, 3840, 960, 60, RECT_SELECTED],
      [2, 3840 + 240, 960, 60],
    ])
    expect(hitTestPoint(viewport, stacked, 40, 8)?.id).toBe(1)
    expect(hitTestPoint(viewport, stacked, 70, 8)?.id).toBe(2)
  })

  it("works through the key to row mapping", () => {
    const middleC = items([[1, 3840, 960, keyToRow(60)]])
    // Key 60 is row 67, seven rows below the top of this viewport.
    expect(hitTestPoint(viewport, middleC, 10, 7 * 16 + 1)?.id).toBe(1)
    expect(hitTestPoint(viewport, middleC, 10, 6 * 16 + 15)).toBeNull()
  })
})

describe("hitTestRect", () => {
  const grid = items([
    [1, 3840, 240, 60],
    [2, 3840 + 480, 240, 61],
    [3, 3840 + 960, 240, 62],
    [4, 3840 + 1440, 240, 63],
  ])
  const ids = (indices: number[]) => indices.map((i) => grid.batch.ids[i])

  it("selects every rect the marquee touches", () => {
    expect(ids(hitTestRect(viewport, grid, 0, 0, 200, 200))).toEqual([
      1, 2, 3, 4,
    ])
    expect(ids(hitTestRect(viewport, grid, 20, 20, 70, 40))).toEqual([2, 3])
  })

  it("accepts the corners in any order", () => {
    expect(ids(hitTestRect(viewport, grid, 70, 40, 20, 20))).toEqual([2, 3])
    expect(ids(hitTestRect(viewport, grid, 20, 40, 70, 20))).toEqual([2, 3])
  })

  it("does not select a rect it only borders", () => {
    // Rect 1 ends at x = 15 and at y = 16.
    expect(ids(hitTestRect(viewport, grid, 15, 0, 29, 16))).toEqual([])
    expect(ids(hitTestRect(viewport, grid, 14.9, 0, 29, 16))).toEqual([1])
  })

  it("selects nothing for a click without a drag", () => {
    expect(hitTestRect(viewport, grid, 5, 5, 5, 5)).toEqual([])
  })
})
