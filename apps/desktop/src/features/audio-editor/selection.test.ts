import { describe, expect, it } from "vitest"
import { frameAt, selectionBetween, validSelection } from "./selection"

describe("audio editor frame selection", () => {
  it("maps and clamps the pointer and sorts backward drags with exclusive ends", () => {
    expect(frameAt(35, 10, 100, 1000)).toBe(250)
    expect(frameAt(-1, 10, 100, 1000)).toBe(0)
    expect(frameAt(200, 10, 100, 1000)).toBe(1000)
    expect(frameAt(35, 10, 0, 1000)).toBe(0)
    expect(selectionBetween(900, 100, 1000)).toEqual({ start: 100, end: 900 })
    expect(selectionBetween(1000, 1000, 1000)).toEqual({
      start: 999,
      end: 1000,
    })
  })
  it("refuses empty, fractional, non-finite and out-of-bounds selections", () => {
    expect(validSelection({ start: 0, end: 1000 }, 1000)).toBe(true)
    for (const selection of [
      { start: 0, end: 0 },
      { start: -1, end: 20 },
      { start: 0, end: 1001 },
      { start: 0.5, end: 2 },
      { start: NaN, end: 2 },
    ]) {
      expect(validSelection(selection, 1000)).toBe(false)
    }
  })
})
