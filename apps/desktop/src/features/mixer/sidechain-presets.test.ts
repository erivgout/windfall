import { describe, expect, it } from "vitest"

import { MAX_GAIN } from "@/lib/units"

import { nextSidechainGain, SIDECHAIN_PRESETS } from "./sidechain-presets"

describe("sidechain level presets", () => {
  it("lists Off, Quiet, Unity, and Loud with their values", () => {
    expect(SIDECHAIN_PRESETS).toEqual([
      { label: "Off", value: 0 },
      { label: "Quiet", value: 0.5 },
      { label: "Unity", value: 1 },
      { label: "Loud", value: 1.5 },
    ])
    for (const preset of SIDECHAIN_PRESETS) {
      expect(preset.value).toBeGreaterThanOrEqual(0)
      expect(preset.value).toBeLessThan(MAX_GAIN)
    }
  })

  it("returns null when the sidechain is already there", () => {
    for (const preset of SIDECHAIN_PRESETS) {
      expect(nextSidechainGain(preset.value, preset.value)).toBeNull()
    }
  })

  it("counts a difference smaller than 0.001 as a match", () => {
    for (const preset of SIDECHAIN_PRESETS) {
      expect(nextSidechainGain(preset.value - 0.0009, preset.value)).toBeNull()
      expect(nextSidechainGain(preset.value + 0.0009, preset.value)).toBeNull()
    }
  })

  it("returns the preset when it differs", () => {
    for (const preset of SIDECHAIN_PRESETS) {
      expect(nextSidechainGain(MAX_GAIN, preset.value)).toBe(preset.value)
      expect(nextSidechainGain(preset.value - 0.002, preset.value)).toBe(
        preset.value
      )
      expect(nextSidechainGain(preset.value + 0.002, preset.value)).toBe(
        preset.value
      )
    }
    expect(nextSidechainGain(0.001, 0)).toBe(0)
  })
})
