import { describe, expect, it } from "vitest"

import { MAX_GAIN } from "@/lib/units"

import { FADER_PRESETS, nextFaderVolume } from "./fader-presets"

describe("fader volume presets", () => {
  it("lists Quiet, Unity, and Loud with their values", () => {
    expect(FADER_PRESETS).toEqual([
      { label: "Quiet", value: 0.5 },
      { label: "Unity", value: 1 },
      { label: "Loud", value: 1.5 },
    ])
    for (const preset of FADER_PRESETS) {
      expect(preset.value).toBeGreaterThan(0)
      expect(preset.value).toBeLessThan(MAX_GAIN)
    }
  })

  it("returns null when the fader is already there", () => {
    for (const preset of FADER_PRESETS) {
      expect(nextFaderVolume(preset.value, preset.value)).toBeNull()
    }
  })

  it("counts a difference smaller than 0.001 as a match", () => {
    for (const preset of FADER_PRESETS) {
      expect(nextFaderVolume(preset.value - 0.0009, preset.value)).toBeNull()
      expect(nextFaderVolume(preset.value + 0.0009, preset.value)).toBeNull()
    }
  })

  it("returns the preset when it differs", () => {
    for (const preset of FADER_PRESETS) {
      expect(nextFaderVolume(0, preset.value)).toBe(preset.value)
      expect(nextFaderVolume(preset.value - 0.002, preset.value)).toBe(
        preset.value
      )
      expect(nextFaderVolume(preset.value + 0.002, preset.value)).toBe(
        preset.value
      )
    }
  })
})
