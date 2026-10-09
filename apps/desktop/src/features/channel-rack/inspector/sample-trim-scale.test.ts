import { describe, expect, it } from "vitest"

import { nextSampleTrimScale, scaledTrimEnd } from "./sample-trim-scale"

describe("sample trim scale", () => {
  it.each([
    { start: 0, end: 1, half: 0.5, double: null },
    { start: 0, end: 0.5, half: 0.25, double: 1 },
    { start: 0.25, end: 0.75, half: 0.5, double: 1 },
    { start: 0.5, end: 1, half: 0.75, double: null },
    { start: 0.9, end: 1, half: 0.95, double: null },
    { start: 0.2, end: 0.2, half: null, double: null },
    { start: 0, end: 0.001, half: null, double: 0.002 },
  ])(
    "scales the trim from $start to $end while keeping the start",
    ({ start, end, half, double }) => {
      expect(nextSampleTrimScale(start, end, "half")).toBe(half)
      expect(nextSampleTrimScale(start, end, "double")).toBe(double)
    }
  )

  it("scales the fractional span without rounding", () => {
    const start = 0.1234567
    const end = 0.3456789
    expect(scaledTrimEnd(start, end, "half")).toBe(start + (end - start) / 2)
    expect(scaledTrimEnd(start, end, "double")).toBe(start + (end - start) * 2)
  })

  it("caps the doubled end at the end of the sample", () => {
    expect(scaledTrimEnd(0.25, 0.75, "double")).toBe(1)
    expect(scaledTrimEnd(0, 1, "double")).toBe(1)
  })
})
