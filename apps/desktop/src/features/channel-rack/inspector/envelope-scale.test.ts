import { describe, expect, it } from "vitest"

import { nextEnvelopeScale } from "./envelope-scale"

describe("envelope time scaling", () => {
  it("halves all three times and preserves sustain without mutating the input", () => {
    const current = Object.freeze({
      attackMs: 8,
      decayMs: 400,
      sustain: 0.7,
      releaseMs: 250,
    })

    expect(nextEnvelopeScale(current, 0.5)).toEqual({
      attackMs: 4,
      decayMs: 200,
      sustain: 0.7,
      releaseMs: 125,
    })
    expect(current).toEqual({
      attackMs: 8,
      decayMs: 400,
      sustain: 0.7,
      releaseMs: 250,
    })
  })

  it("doubles all three times and preserves sustain", () => {
    expect(
      nextEnvelopeScale(
        { attackMs: 8, decayMs: 400, sustain: 0.7, releaseMs: 250 },
        2
      )
    ).toEqual({ attackMs: 16, decayMs: 800, sustain: 0.7, releaseMs: 500 })
  })

  it("halves a short envelope while keeping a zero decay at zero", () => {
    expect(
      nextEnvelopeScale(
        { attackMs: 1, decayMs: 0, sustain: 1, releaseMs: 30 },
        0.5
      )
    ).toEqual({ attackMs: 0.5, decayMs: 0, sustain: 1, releaseMs: 15 })
  })

  it("snaps half of a 0.3 ms attack to 0.2 ms", () => {
    expect(
      nextEnvelopeScale(
        { attackMs: 0.3, decayMs: 0, sustain: 0.7, releaseMs: 0 },
        0.5
      )
    ).toEqual({ attackMs: 0.2, decayMs: 0, sustain: 0.7, releaseMs: 0 })
  })

  it.each(["attackMs", "decayMs", "releaseMs"] as const)(
    "keeps half of a 0.1 ms %s at 0.1 ms",
    (field) => {
      const current = {
        attackMs: 8,
        decayMs: 400,
        sustain: 0.7,
        releaseMs: 250,
        [field]: 0.1,
      }

      expect(nextEnvelopeScale(current, 0.5)).toEqual({
        attackMs: 4,
        decayMs: 200,
        sustain: 0.7,
        releaseMs: 125,
        [field]: 0.1,
      })
    }
  )

  it.each([
    ["attackMs", 5000],
    ["decayMs", 5000],
    ["releaseMs", 10000],
  ] as const)("keeps a doubled %s at its %s ms limit", (field, limit) => {
    const current = {
      attackMs: 8,
      decayMs: 400,
      sustain: 0.7,
      releaseMs: 250,
      [field]: limit,
    }

    expect(nextEnvelopeScale(current, 2)).toEqual({
      attackMs: 16,
      decayMs: 800,
      sustain: 0.7,
      releaseMs: 500,
      [field]: limit,
    })
  })

  it.each([
    ["attackMs", 3000, 5000],
    ["releaseMs", 6000, 10000],
  ] as const)("caps doubling a %s of %s ms at %s ms", (field, time, limit) => {
    expect(
      nextEnvelopeScale(
        { attackMs: 0, decayMs: 0, sustain: 0.7, releaseMs: 0, [field]: time },
        2
      )
    ).toEqual({
      attackMs: 0,
      decayMs: 0,
      sustain: 0.7,
      releaseMs: 0,
      [field]: limit,
    })
  })

  it("returns null when halving three zero times", () => {
    expect(
      nextEnvelopeScale(
        { attackMs: 0, decayMs: 0, sustain: 0.4, releaseMs: 0 },
        0.5
      )
    ).toBeNull()
  })

  it("returns null when doubling three times already at their limits", () => {
    expect(
      nextEnvelopeScale(
        { attackMs: 5000, decayMs: 5000, sustain: 0.2, releaseMs: 10000 },
        2
      )
    ).toBeNull()
  })

  it.each(["attackMs", "decayMs", "releaseMs"] as const)(
    "returns the whole envelope with exact sustain when only %s changes",
    (field) => {
      const current = {
        attackMs: 0.1,
        decayMs: 0.1,
        sustain: 0.234567,
        releaseMs: 0.1,
        [field]: 2,
      }

      expect(nextEnvelopeScale(current, 0.5)).toEqual({
        attackMs: 0.1,
        decayMs: 0.1,
        sustain: current.sustain,
        releaseMs: 0.1,
        [field]: 1,
      })
    }
  )
})
