import { describe, expect, it } from "vitest"

import { MIX_PRESETS, nextMix } from "./mix-presets"

describe("mix presets", () => {
  it("lists Dry, 25%, Half, 75%, and Wet with their values", () => {
    expect(MIX_PRESETS).toEqual([
      { label: "Dry", value: 0 },
      { label: "25%", value: 0.25 },
      { label: "Half", value: 0.5 },
      { label: "75%", value: 0.75 },
      { label: "Wet", value: 1 },
    ])
  })

  it("returns null when the knob is already there", () => {
    for (const preset of MIX_PRESETS) {
      expect(nextMix(preset.value, preset.value)).toBeNull()
    }
  })

  it("counts a difference smaller than 0.001 as a match", () => {
    for (const preset of MIX_PRESETS) {
      expect(nextMix(preset.value - 0.0009, preset.value)).toBeNull()
      expect(nextMix(preset.value + 0.0009, preset.value)).toBeNull()
    }
  })

  it("returns the preset when it differs", () => {
    for (const preset of MIX_PRESETS) {
      expect(nextMix(preset.value - 0.002, preset.value)).toBe(preset.value)
      expect(nextMix(preset.value + 0.002, preset.value)).toBe(preset.value)
    }
    expect(nextMix(0.001, 0)).toBe(0)
    expect(nextMix(0, 0.001)).toBe(0.001)
  })
})
