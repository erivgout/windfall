import { describe, expect, it } from "vitest"

import { joinTune, splitTune } from "../steps"
import { nextSamplerFineScale } from "./fine-cents-scale"

describe("sampler fine cents scale", () => {
  it.each([
    { tune: 0, coarse: 0, factor: "half", expected: null },
    { tune: 0, coarse: 0, factor: "double", expected: null },
    { tune: 0.25, coarse: 0, factor: "half", expected: 0.12 },
    { tune: 0.25, coarse: 0, factor: "double", expected: 0.5 },
    { tune: -0.25, coarse: 0, factor: "half", expected: -0.12 },
    { tune: -0.25, coarse: 0, factor: "double", expected: -0.5 },
    { tune: 0.01, coarse: 0, factor: "half", expected: 0 },
    { tune: 0.01, coarse: 0, factor: "double", expected: 0.02 },
    { tune: -0.01, coarse: 0, factor: "half", expected: 0 },
    { tune: -0.01, coarse: 0, factor: "double", expected: -0.02 },
    { tune: 0.3, coarse: 0, factor: "double", expected: 0.5 },
    { tune: -0.3, coarse: 0, factor: "double", expected: -0.5 },
    { tune: 0.5, coarse: 0, factor: "double", expected: null },
    { tune: -0.5, coarse: 0, factor: "double", expected: null },
    { tune: 3.25, coarse: 3, factor: "half", expected: 3.12 },
    { tune: 3.25, coarse: 3, factor: "double", expected: 3.5 },
    { tune: -3.25, coarse: -3, factor: "half", expected: -3.12 },
    { tune: -3.25, coarse: -3, factor: "double", expected: -3.5 },
  ] as const)(
    "$factor of tune $tune with coarse $coarse returns $expected",
    ({ tune, coarse, factor, expected }) => {
      const next = nextSamplerFineScale(tune, coarse, factor)
      expect(next).toBe(expected)
      if (next !== null) {
        expect(splitTune(next, coarse).semitones).toBe(
          splitTune(tune, coarse).semitones
        )
      }
    }
  )

  it.each([
    { factor: "half", nextCents: 12 },
    { factor: "double", nextCents: 50 },
  ] as const)(
    "skips $factor at +48 semitones when clamping drops the cents",
    ({ factor, nextCents }) => {
      const tune = 48.25
      expect(splitTune(tune, 48)).toEqual({ semitones: 48, cents: 25 })
      const clamped = joinTune(48, nextCents, 48)
      expect(splitTune(clamped, 48)).toEqual({ semitones: 48, cents: 0 })
      expect(nextSamplerFineScale(tune, 48, factor)).toBeNull()
    }
  )
})
