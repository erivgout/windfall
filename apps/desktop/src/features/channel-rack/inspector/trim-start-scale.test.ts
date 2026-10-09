import { describe, expect, it } from "vitest"

import { nextTrimStartScale, scaledTrimStart } from "./trim-start-scale"

describe("trim start scale", () => {
  it.each([
    [0, 1, "half", 0.5, 0.5],
    [0, 1, "double", 0, null],
    [0.5, 1, "half", 0.75, 0.75],
    [0.5, 1, "double", 0, 0],
    [0.25, 0.75, "half", 0.5, 0.5],
    [0.25, 0.75, "double", 0, 0],
    [0.6, 0.8, "half", 0.7, 0.7],
    [0.6, 0.8, "double", 0.4, 0.4],
    [0.9, 1, "half", 0.95, 0.95],
    [0.9, 1, "double", 0.8, 0.8],
    [0.2, 0.2, "half", 0.2, null],
    [0.2, 0.2, "double", 0.2, null],
    [0, 0.001, "half", 0.0005, null],
    [0, 0.001, "double", 0, null],
  ] as const)(
    "scales start %s with end %s by %s",
    (start, end, factor, scaled, expected) => {
      expect(scaledTrimStart(start, end, factor)).toBeCloseTo(scaled, 12)
      const next = nextTrimStartScale(start, end, factor)
      if (expected === null) expect(next).toBeNull()
      else expect(next).toBeCloseTo(expected, 12)
    }
  )

  it("keeps changes equal to 0.001", () => {
    expect(nextTrimStartScale(0, 0.002, "half")).toBe(0.001)
    expect(nextTrimStartScale(0.001, 0.002, "double")).toBe(0)
  })

  it("does not round the scaled start", () => {
    const start = 0.3456789
    const end = 0.4567891
    const span = end - start
    expect(nextTrimStartScale(start, end, "half")).toBe(start + span / 2)
    expect(nextTrimStartScale(start, end, "double")).toBe(start - span)
  })
})
