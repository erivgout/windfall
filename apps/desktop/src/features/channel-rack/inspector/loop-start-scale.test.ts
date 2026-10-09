import { describe, expect, it } from "vitest"

import { nextLoopStartScale, scaledLoopStart } from "./loop-start-scale"

const cases: [number, number, "half" | "double", number, number | null][] = [
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
]

describe("loop start scale", () => {
  it.each(cases)(
    "scales start %s and end %s by %s to %s (next: %s)",
    (start, end, factor, scaled, next) => {
      expect(scaledLoopStart(start, end, factor)).toBeCloseTo(scaled, 10)
      if (next === null) {
        expect(nextLoopStartScale(start, end, factor)).toBeNull()
      } else {
        expect(nextLoopStartScale(start, end, factor)).toBeCloseTo(next, 10)
      }
    }
  )
})
