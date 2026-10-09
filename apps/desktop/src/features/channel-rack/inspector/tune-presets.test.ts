import { describe, expect, it } from "vitest"

import { nextSamplerTune, TUNE_PRESETS } from "./tune-presets"

describe("sampler tune presets", () => {
  it("lists the five presets in semitones", () => {
    expect(TUNE_PRESETS).toEqual([
      { label: "Octave down", semitones: -12 },
      { label: "Fifth down", semitones: -7 },
      { label: "Unison", semitones: 0 },
      { label: "Fifth up", semitones: 7 },
      { label: "Octave up", semitones: 12 },
    ])
  })

  it("keeps 25 cents when moving from 0 to 12 semitones", () => {
    expect(nextSamplerTune(0.25, 25, 12)).toBe(12.25)
  })

  it("returns null when the joined tune already matches", () => {
    for (const { semitones } of TUNE_PRESETS) {
      expect(nextSamplerTune(semitones + 0.25, 25, semitones)).toBeNull()
    }
  })

  it("counts a difference smaller than 0.001 as a match", () => {
    for (const difference of [-0.0009, 0.0009]) {
      expect(nextSamplerTune(12.25 + difference, 25, 12)).toBeNull()
    }
  })

  it("keeps Fine at either end of its range", () => {
    for (const cents of [-50, 50]) {
      for (const { semitones } of TUNE_PRESETS) {
        expect(nextSamplerTune(3 + cents / 100, cents, semitones)).toBe(
          semitones + cents / 100
        )
      }
    }
  })
})
