import { describe, expect, it } from "vitest"

import { rgba } from "./color"
import { RECT_FLAT, RECT_SELECTED, RectBatch } from "./rect-batch"

const RED = rgba(255, 0, 0)
const BLUE = rgba(0, 0, 255, 128)

describe("RectBatch", () => {
  it("stores rects and grows past its first capacity", () => {
    const batch = new RectBatch(2)
    for (let i = 0; i < 100; i++) {
      batch.push(1000 + i, i * 10, 5, i % 7, 1, i % 2 ? RED : BLUE)
    }
    expect(batch.count).toBe(100)
    expect(batch.start(99)).toBe(990)
    expect(batch.end(99)).toBe(995)
    expect(batch.row(99)).toBe(1)
    expect(batch.ids[99]).toBe(1099)
    expect([...batch.colors.subarray(0, 8)]).toEqual([
      0, 0, 255, 128, 255, 0, 0, 255,
    ])
  })

  it("bumps a version for every kind of change", () => {
    const batch = new RectBatch()
    batch.push(1, 0, 10, 0, 1, RED)
    const { geometryVersion, colorVersion, flagsVersion } = batch

    batch.setSelected(0, true)
    expect(batch.flagsVersion).toBeGreaterThan(flagsVersion)
    expect(batch.geometryVersion).toBe(geometryVersion)

    batch.setColor(0, BLUE)
    expect(batch.colorVersion).toBeGreaterThan(colorVersion)
    expect(batch.geometryVersion).toBe(geometryVersion)

    batch.translateSelected(240, -1)
    expect(batch.geometryVersion).toBeGreaterThan(geometryVersion)
  })

  it("does not bump the flags version when selection does not change", () => {
    const batch = new RectBatch()
    batch.push(1, 0, 10, 0, 1, RED)
    const before = batch.flagsVersion
    batch.setSelected(0, false)
    batch.clearSelection()
    expect(batch.flagsVersion).toBe(before)
  })

  it("tracks the selected count", () => {
    const batch = new RectBatch()
    for (let i = 0; i < 10; i++) batch.push(i, i, 1, 0, 1, RED)
    batch.setSelection([1, 3, 3, 5, 99])
    expect(batch.selectedCount).toBe(3)
    expect(batch.selectedIndices()).toEqual([1, 3, 5])
    batch.setSelected(3, false)
    batch.setSelected(7, true)
    batch.setSelected(7, true)
    expect(batch.selectedCount).toBe(3)
    expect(batch.selectedIndices()).toEqual([1, 5, 7])
    batch.setSelection([2])
    expect(batch.selectedIndices()).toEqual([2])
    batch.clearSelection()
    expect(batch.selectedCount).toBe(0)
  })

  it("keeps other flag bits when selection changes", () => {
    const batch = new RectBatch()
    batch.push(1, 0, 10, 0, 1, RED, RECT_FLAT | RECT_SELECTED)
    expect(batch.selectedCount).toBe(1)
    batch.clearSelection()
    expect(batch.flags[0]).toBe(RECT_FLAT)
  })

  it("moves only selected rects", () => {
    const batch = new RectBatch()
    batch.push(1, 100, 10, 5, 1, RED)
    batch.push(2, 200, 10, 6, 1, RED, RECT_SELECTED)
    batch.translateSelected(-50, 2)
    expect([batch.start(0), batch.row(0)]).toEqual([100, 5])
    expect([batch.start(1), batch.row(1)]).toEqual([150, 8])
  })

  it("sorts by start and keeps equal starts in their original order", () => {
    const batch = new RectBatch()
    const starts = [300, 100, 300, 0, 100, 300]
    starts.forEach((start, i) =>
      batch.push(i, start, 1, i, 1, i === 2 ? BLUE : RED)
    )
    expect(batch.isSortedByStart()).toBe(false)
    batch.sortByStart()
    expect(batch.isSortedByStart()).toBe(true)
    expect([...batch.ids.subarray(0, 6)]).toEqual([3, 1, 4, 0, 2, 5])
    expect(batch.row(4)).toBe(2)
    expect(batch.colors[4 * 4 + 2]).toBe(255)
    expect(batch.indexOfId(5)).toBe(5)
    expect(batch.indexOfId(42)).toBe(-1)
  })

  it("sorts negative starts", () => {
    const batch = new RectBatch()
    for (const start of [5, -960, 0, -1]) batch.push(start, start, 1, 0, 1, RED)
    batch.sortByStart()
    expect([...batch.ids.subarray(0, 4)]).toEqual([-960, -1, 0, 5])
  })

  it("clears", () => {
    const batch = new RectBatch()
    batch.push(1, 0, 1, 0, 1, RED, RECT_SELECTED)
    batch.clear()
    expect(batch.count).toBe(0)
    expect(batch.selectedCount).toBe(0)
    expect(batch.indexOfId(1)).toBe(-1)
  })
})
