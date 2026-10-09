import { describe, expect, it } from "vitest"

import { nextSwing, SWING_PRESETS } from "./swing-presets"

describe("swing presets", () => {
  it("lists Straight, Light, Medium, Heavy, and Full with their values", () => {
    expect(SWING_PRESETS).toEqual([
      { label: "Straight", value: 0 },
      { label: "Light", value: 0.25 },
      { label: "Medium", value: 0.5 },
      { label: "Heavy", value: 0.75 },
      { label: "Full", value: 1 },
    ])
  })

  it("returns null when the current value is within 0.001 of the preset", () => {
    for (const preset of SWING_PRESETS) {
      expect(nextSwing(preset.value, preset)).toBeNull()
      if (preset.value > 0) {
        expect(nextSwing(preset.value - 0.0009, preset)).toBeNull()
      }
      if (preset.value < 1) {
        expect(nextSwing(preset.value + 0.0009, preset)).toBeNull()
      }
    }
  })

  it("returns the preset when the current value is farther away", () => {
    for (const preset of SWING_PRESETS) {
      if (preset.value > 0) {
        expect(nextSwing(preset.value - 0.002, preset)).toBe(preset.value)
      }
      if (preset.value < 1) {
        expect(nextSwing(preset.value + 0.002, preset)).toBe(preset.value)
      }
    }
    expect(nextSwing(0.001, SWING_PRESETS[0])).toBe(0)
  })

  it("does not match Light when the current value is zero", () => {
    expect(nextSwing(0, SWING_PRESETS[1])).toBe(0.25)
  })
})
