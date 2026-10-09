import { describe, expect, it } from "vitest"

import { splitTune } from "../steps"
import { nextSamplerFinePreset } from "./fine-tune-preset-step"
import { FINE_TUNE_PRESETS } from "./fine-tune-presets"

describe("sampler fine-tune preset stepping", () => {
  it("reads the preset endpoints as -50 and 50 cents", () => {
    expect(FINE_TUNE_PRESETS[0].cents).toBe(-50)
    expect(FINE_TUNE_PRESETS[FINE_TUNE_PRESETS.length - 1].cents).toBe(50)
  })

  it.each([
    { tune: -0.5, coarse: 0, previous: null, next: -0.25 },
    { tune: -0.25, coarse: 0, previous: -0.5, next: 0 },
    { tune: 0, coarse: 0, previous: -0.25, next: 0.25 },
    { tune: 0.25, coarse: 0, previous: 0, next: 0.5 },
    { tune: 0.5, coarse: 0, previous: 0.25, next: null },
    { tune: 0.1, coarse: 0, previous: 0, next: 0.25 },
    { tune: -0.1, coarse: 0, previous: -0.25, next: 0 },
    { tune: -0.6, coarse: 0, previous: null, next: -0.5 },
    { tune: 0.6, coarse: 0, previous: 0.5, next: null },
    { tune: 48, coarse: 48, previous: 47.75, next: null },
  ])(
    "steps tune $tune around shown semitone $coarse",
    ({ tune, coarse, previous, next }) => {
      expect(nextSamplerFinePreset(tune, coarse, "previous")).toBe(previous)
      expect(nextSamplerFinePreset(tune, coarse, "next")).toBe(next)
    }
  )

  it.each([NaN, Infinity, -Infinity])(
    "returns null in both directions for non-finite tune %s",
    (tune) => {
      expect(nextSamplerFinePreset(tune, 0, "previous")).toBeNull()
      expect(nextSamplerFinePreset(tune, 0, "next")).toBeNull()
    }
  )

  it("keeps a nonzero shown semitone at both fine-tune endpoints", () => {
    expect(nextSamplerFinePreset(3.25, 3, "next")).toBe(3.5)
    expect(splitTune(3.5, 3)).toEqual({ semitones: 3, cents: 50 })
    expect(nextSamplerFinePreset(2.75, 3, "previous")).toBe(2.5)
    expect(splitTune(2.5, 3)).toEqual({ semitones: 3, cents: -50 })
  })

  it("rejects a clamped result that changes the shown semitone", () => {
    expect(nextSamplerFinePreset(48, 49, "next")).toBeNull()
  })
})
