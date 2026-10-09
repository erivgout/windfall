import { describe, expect, it } from "vitest"

import { nextNumeratorScale, scaledNumerator } from "./numerator-scale"

const CASES = [
  [1, "half", 1],
  [1, "double", 2],
  [16, "double", 16],
  [16, "half", 8],
  [4, "half", 2],
  [4, "double", 8],
  [3, "half", 1],
  [3, "double", 6],
  [5, "half", 2],
  [5, "double", 10],
  [7, "half", 3],
  [7, "double", 14],
  [6, "half", 3],
  [6, "double", 12],
  [9, "double", 16],
  [12, "half", 6],
  [12, "double", 16],
] as const

describe("scaledNumerator", () => {
  it.each(CASES)("scales %i by %s to %i", (numerator, factor, expected) => {
    expect(scaledNumerator(numerator, factor)).toBe(expected)
  })
})

describe("nextNumeratorScale", () => {
  it.each(CASES)(
    "scales %i by %s toward %i, returning null when unchanged",
    (numerator, factor, expected) => {
      expect(nextNumeratorScale(numerator, factor)).toBe(
        expected === numerator ? null : expected
      )
    }
  )
})
