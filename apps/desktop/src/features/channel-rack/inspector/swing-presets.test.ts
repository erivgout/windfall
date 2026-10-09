import { describe, expect, it } from "vitest"

import { SWING_PRESETS, nextSwing } from "./swing-presets"

describe("swing mix presets", () => {
  it("lists Straight, Light, Half, and Full with their mix values", () => {
    expect(SWING_PRESETS).toEqual([
      { label: "Straight", value: 0 },
      { label: "Light", value: 0.25 },
      { label: "Half", value: 0.5 },
      { label: "Full", value: 1 },
    ])
  })

  it("returns null when the mix is already that amount or differs by less than 0.001", () => {
    for (const { value: preset } of SWING_PRESETS) {
      expect(nextSwing(preset, preset)).toBeNull()
      if (preset > 0) {
        expect(nextSwing(preset - 0.0009, preset)).toBeNull()
      }
      if (preset < 1) {
        expect(nextSwing(preset + 0.0009, preset)).toBeNull()
      }
    }
  })

  it("returns the preset when the mix differs", () => {
    for (const { value: preset } of SWING_PRESETS) {
      for (const current of [0, 0.25, 0.5, 0.75, 1]) {
        if (current !== preset) {
          expect(nextSwing(current, preset)).toBe(preset)
        }
      }
      if (preset > 0) {
        expect(nextSwing(preset - 0.002, preset)).toBe(preset)
      }
      if (preset < 1) {
        expect(nextSwing(preset + 0.002, preset)).toBe(preset)
      }
    }
    expect(nextSwing(0.001, 0)).toBe(0)
  })
})
