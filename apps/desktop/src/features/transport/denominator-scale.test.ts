import { describe, expect, it } from "vitest"

import { nextDenominatorScale, scaledDenominator } from "./denominator-scale"

const CASES = [
  [2, "half", 2],
  [2, "double", 4],
  [4, "half", 2],
  [4, "double", 8],
  [8, "half", 4],
  [8, "double", 16],
  [16, "half", 8],
  [16, "double", 16],
  [3, "half", null],
  [3, "double", null],
] as const

describe("scaledDenominator", () => {
  it.each(CASES)("scales %i by %s to %s", (denominator, factor, expected) => {
    expect(scaledDenominator(denominator, factor)).toBe(expected)
  })
})

describe("nextDenominatorScale", () => {
  it.each(CASES)(
    "scales %i by %s toward %s, returning null when unchanged or unsupported",
    (denominator, factor, expected) => {
      expect(nextDenominatorScale(denominator, factor)).toBe(
        expected === denominator ? null : expected
      )
    }
  )
})
