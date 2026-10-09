import { describe, expect, it } from "vitest"

import { nextSampleGain, SAMPLE_GAIN_PRESETS } from "./sample-gain-presets"

describe("sample gain presets", () => {
  it("lists the three presets as linear gains", () => {
    expect(SAMPLE_GAIN_PRESETS).toEqual([
      { label: "Quiet", gain: 0.5 },
      { label: "Unity", gain: 1 },
      { label: "Loud", gain: 1.5 },
    ])
  })

  it("returns null when the gain already matches", () => {
    for (const { gain } of SAMPLE_GAIN_PRESETS) {
      expect(nextSampleGain(gain, gain)).toBeNull()
    }
  })

  it("counts a difference smaller than 0.001 as a match", () => {
    for (const { gain } of SAMPLE_GAIN_PRESETS) {
      for (const difference of [-0.0009, 0.0009]) {
        expect(nextSampleGain(gain + difference, gain)).toBeNull()
      }
    }
  })

  it("returns the preset when the gain differs", () => {
    for (const { gain } of SAMPLE_GAIN_PRESETS) {
      for (const current of [0, 2, gain - 0.0011, gain + 0.0011]) {
        expect(nextSampleGain(current, gain)).toBe(gain)
      }
    }
    expect(nextSampleGain(0, 0.001)).toBe(0.001)
  })
})
