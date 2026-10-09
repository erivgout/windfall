import { describe, expect, it } from "vitest"

import { nextLoopLengthScale, scaledLoopEnd } from "./loop-length-scale"

const cases: [number, number, "half" | "double", number, number | null][] = [
  [0, 1, "half", 0.5, 0.5],
  [0, 1, "double", 1, null],
  [0, 0.5, "half", 0.25, 0.25],
  [0, 0.5, "double", 1, 1],
  [0.25, 0.75, "half", 0.5, 0.5],
  [0.25, 0.75, "double", 1, 1],
  [0.5, 1, "half", 0.75, 0.75],
  [0.5, 1, "double", 1, null],
  [0.9, 1, "half", 0.95, 0.95],
  [0.9, 1, "double", 1, null],
  [0.2, 0.2, "half", 0.2, null],
  [0.2, 0.2, "double", 0.2, null],
  [0, 0.001, "half", 0.0005, null],
  [0, 0.001, "double", 0.002, 0.002],
]

describe("loop length scale", () => {
  it.each(cases)(
    "scales start %s and end %s by %s to %s (next: %s)",
    (start, end, factor, scaled, next) => {
      expect(scaledLoopEnd(start, end, factor)).toBeCloseTo(scaled, 10)
      if (next === null) {
        expect(nextLoopLengthScale(start, end, factor)).toBeNull()
      } else {
        expect(nextLoopLengthScale(start, end, factor)).toBeCloseTo(next, 10)
      }
    }
  )
})
