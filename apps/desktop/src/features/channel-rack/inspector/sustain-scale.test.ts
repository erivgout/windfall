import { describe, expect, it } from "vitest"

import { nextSustainScale } from "./sustain-scale"

describe("envelope sustain scaling", () => {
  it.each([
    [0.7, 0.5, 0.35],
    [0.7, 2, 1],
    [0.4, 2, 0.8],
    [0.6, 2, 1],
    [0.007, 0.5, 0.004],
    [0.234567, 2, 0.469],
  ] as const)(
    "scales sustain %s by %s to %s and preserves all three times",
    (sustain, factor, expected) => {
      const current = Object.freeze({
        attackMs: 8,
        decayMs: 400,
        sustain,
        releaseMs: 250,
      })

      expect(nextSustainScale(current, factor)).toEqual({
        attackMs: 8,
        decayMs: 400,
        sustain: expected,
        releaseMs: 250,
      })
      expect(current.sustain).toBe(sustain)
    }
  )

  it.each([
    [0, 0.5],
    [1, 2],
    [0.001, 0.5],
    [0.0015, 0.5],
    [0.002, 0.5],
    [0.999, 2],
  ] as const)(
    "returns null when scaling sustain %s by %s changes it by at most a thousandth",
    (sustain, factor) => {
      expect(
        nextSustainScale(
          { attackMs: 8, decayMs: 400, sustain, releaseMs: 250 },
          factor
        )
      ).toBeNull()
    }
  )

  it("returns the whole envelope when sustain changes by more than a thousandth", () => {
    expect(
      nextSustainScale(
        { attackMs: 8, decayMs: 400, sustain: 0.004, releaseMs: 250 },
        0.5
      )
    ).toEqual({ attackMs: 8, decayMs: 400, sustain: 0.002, releaseMs: 250 })
  })

  it("copies fractional and out-of-limit times exactly", () => {
    expect(
      nextSustainScale(
        {
          attackMs: 0.123456,
          decayMs: 6000.789,
          sustain: 0.4,
          releaseMs: 12000.123,
        },
        2
      )
    ).toEqual({
      attackMs: 0.123456,
      decayMs: 6000.789,
      sustain: 0.8,
      releaseMs: 12000.123,
    })
  })
})
