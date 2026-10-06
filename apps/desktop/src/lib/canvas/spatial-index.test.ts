import { describe, expect, it } from "vitest"

import { rgba } from "./color"
import { RECT_SELECTED, RectBatch } from "./rect-batch"
import {
  buildTimeIndex,
  indexBatch,
  lowerBoundStart,
  queryPoint,
  queryRect,
  visibleRange,
  type IndexedBatch,
} from "./spatial-index"

const COLOR = rgba(200, 50, 120)

function random(seed: number): () => number {
  let state = seed >>> 0
  return () => {
    state = (state + 0x6d2b79f5) >>> 0
    let t = state
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

function randomItems(
  count: number,
  seed: number,
  maxLength = 1920
): IndexedBatch {
  const next = random(seed)
  const batch = new RectBatch(count)
  for (let id = 0; id < count; id++) {
    batch.push(
      id,
      Math.floor(next() * 768_000),
      1 + Math.floor(next() * maxLength),
      Math.floor(next() * 128),
      1,
      COLOR
    )
  }
  return indexBatch(batch)
}

function overlapping(
  items: IndexedBatch,
  tick0: number,
  tick1: number,
  row0: number,
  row1: number
): number[] {
  const { batch } = items
  const out: number[] = []
  for (let i = 0; i < batch.count; i++) {
    if (batch.start(i) >= tick1 || batch.end(i) <= tick0) continue
    if (batch.row(i) >= row1 || batch.row(i) + batch.rowSpan(i) <= row0)
      continue
    out.push(i)
  }
  return out
}

describe("indexBatch", () => {
  it("sorts by start and keeps ids attached to their rects", () => {
    const batch = new RectBatch()
    batch.push(10, 500, 100, 3, 1, COLOR)
    batch.push(11, 100, 50, 7, 1, COLOR, RECT_SELECTED)
    batch.push(12, 300, 10, 9, 1, COLOR)
    const items = indexBatch(batch)
    expect([...items.batch.ids.subarray(0, 3)]).toEqual([11, 12, 10])
    expect(items.batch.start(0)).toBe(100)
    expect(items.batch.row(0)).toBe(7)
    expect(items.batch.isSelected(0)).toBe(true)
    expect(items.batch.indexOfId(10)).toBe(2)
  })

  it("handles an empty batch", () => {
    const items = indexBatch(new RectBatch())
    expect(visibleRange(items, 0, 1000)).toEqual({ first: 0, last: 0 })
    expect(queryPoint(items, 5, 5)).toBe(-1)
    expect(queryRect(items, 0, 100, 0, 100)).toEqual([])
  })
})

describe("lowerBoundStart", () => {
  it("finds the first rect starting at or after a tick", () => {
    const batch = new RectBatch()
    for (const start of [0, 10, 10, 20, 40])
      batch.push(start, start, 5, 0, 1, COLOR)
    expect(lowerBoundStart(batch, -5)).toBe(0)
    expect(lowerBoundStart(batch, 10)).toBe(1)
    expect(lowerBoundStart(batch, 11)).toBe(3)
    expect(lowerBoundStart(batch, 41)).toBe(5)
  })
})

describe("visibleRange", () => {
  const items = randomItems(5000, 1)

  it("never misses a rect that overlaps the window", () => {
    const next = random(99)
    for (let trial = 0; trial < 200; trial++) {
      const tick0 = Math.floor(next() * 800_000) - 10_000
      const tick1 = tick0 + 1 + Math.floor(next() * 60_000)
      const range = visibleRange(items, tick0, tick1)
      for (const i of overlapping(items, tick0, tick1, 0, 128)) {
        expect(i).toBeGreaterThanOrEqual(range.first)
        expect(i).toBeLessThan(range.last)
      }
    }
  })

  it("culls: a narrow window covers a small part of the batch", () => {
    const range = visibleRange(items, 380_000, 380_000 + 30_720)
    const exact = overlapping(items, 380_000, 380_000 + 30_720, 0, 128).length
    expect(range.last - range.first).toBeGreaterThanOrEqual(exact)
    // 30,720 of 768,000 ticks is 4% of the song.
    expect(range.last - range.first).toBeLessThan(5000 * 0.06)
  })

  it("is empty outside the content", () => {
    const before = visibleRange(items, -50_000, -10_000)
    expect(before.last - before.first).toBe(0)
    const after = visibleRange(items, 900_000, 950_000)
    expect(after.last - after.first).toBe(0)
  })

  it("includes one long rect that starts far to the left", () => {
    const batch = new RectBatch()
    batch.push(1, 0, 700_000, 5, 1, COLOR)
    for (let i = 0; i < 2000; i++)
      batch.push(2 + i, 1000 + i * 300, 240, 10, 1, COLOR)
    const long = indexBatch(batch)
    const range = visibleRange(long, 500_000, 510_000)
    expect(range.first).toBe(0)
    expect(range.last).toBe(lowerBoundStart(long.batch, 510_000))
  })
})

describe("queryPoint", () => {
  it("agrees with a brute-force search", () => {
    const items = randomItems(3000, 7)
    const next = random(8)
    let hits = 0
    for (let trial = 0; trial < 2000; trial++) {
      const tick = next() * 770_000
      const row = next() * 128
      const expected = overlapping(items, tick, tick + 1e-9, row, row + 1e-9)
      const found = queryPoint(items, tick, row)
      if (expected.length === 0) {
        expect(found).toBe(-1)
      } else {
        hits++
        expect(found).toBe(expected[expected.length - 1])
      }
    }
    expect(hits).toBeGreaterThan(10)
  })

  it("treats start as inside and end as outside", () => {
    const batch = new RectBatch()
    batch.push(1, 240, 240, 60, 1, COLOR)
    const items = indexBatch(batch)
    expect(queryPoint(items, 240, 60)).toBe(0)
    expect(queryPoint(items, 479.9, 60.99)).toBe(0)
    expect(queryPoint(items, 480, 60)).toBe(-1)
    expect(queryPoint(items, 239.9, 60)).toBe(-1)
    expect(queryPoint(items, 300, 61)).toBe(-1)
    expect(queryPoint(items, 300, 59.99)).toBe(-1)
  })

  it("prefers a selected rect when rects overlap", () => {
    const batch = new RectBatch()
    batch.push(1, 0, 960, 60, 1, COLOR, RECT_SELECTED)
    batch.push(2, 100, 960, 60, 1, COLOR)
    const items = indexBatch(batch)
    expect(items.batch.ids[queryPoint(items, 500, 60.5)]).toBe(1)
    items.batch.clearSelection()
    // With nothing selected the one drawn last (on top) wins.
    expect(items.batch.ids[queryPoint(items, 500, 60.5)]).toBe(2)
  })
})

describe("queryRect", () => {
  it("agrees with a brute-force search and reports each rect once", () => {
    const items = randomItems(4000, 21, 20_000)
    const next = random(22)
    for (let trial = 0; trial < 150; trial++) {
      const tick0 = next() * 760_000 - 5000
      const tick1 = tick0 + next() * 90_000 + 1
      const row0 = next() * 120
      const row1 = row0 + next() * 30 + 0.01
      expect(queryRect(items, tick0, tick1, row0, row1)).toEqual(
        overlapping(items, tick0, tick1, row0, row1)
      )
    }
  })

  it("returns nothing for an empty box", () => {
    const items = randomItems(100, 3)
    expect(queryRect(items, 500, 500, 0, 128)).toEqual([])
    expect(queryRect(items, 0, 768_000, 64, 64)).toEqual([])
  })
})

describe("at 50,000 rects", () => {
  const items = randomItems(50_000, 5, 960)

  it("keeps buckets small, so a query reads a few dozen entries", () => {
    const { index } = items
    let largest = 0
    for (let b = 0; b < index.bucketCount; b++) {
      largest = Math.max(largest, index.offsets[b + 1] - index.offsets[b])
    }
    expect(largest).toBeLessThan(120)
    expect(index.entries.length).toBeLessThan(50_000 * 4)
  })

  it("answers point and box queries correctly", () => {
    const next = random(51)
    for (let trial = 0; trial < 50; trial++) {
      const tick = next() * 768_000
      const row = next() * 128
      const expected = overlapping(items, tick, tick + 1e-9, row, row + 1e-9)
      expect(queryPoint(items, tick, row)).toBe(
        expected.length > 0 ? expected[expected.length - 1] : -1
      )
      expect(queryRect(items, tick, tick + 4000, row, row + 6)).toEqual(
        overlapping(items, tick, tick + 4000, row, row + 6)
      )
    }
  })

  it("lets the caller pick the bucket size", () => {
    const coarse = buildTimeIndex(items.batch, 3840)
    expect(coarse.bucketTicks).toBe(3840)
    expect(coarse.bucketCount).toBeLessThanOrEqual(201)
  })
})
