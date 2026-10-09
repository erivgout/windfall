import { describe, expect, it } from "vitest"

import { PAN_PRESETS, nextPan } from "./pan-presets"

describe("pan presets", () => {
  it("lists Hard left, Left, Center, Right, and Hard right with their values", () => {
    expect(PAN_PRESETS).toEqual([
      { label: "Hard left", value: -1 },
      { label: "Left", value: -0.5 },
      { label: "Center", value: 0 },
      { label: "Right", value: 0.5 },
      { label: "Hard right", value: 1 },
    ])
  })

  it("returns null when the knob is already there", () => {
    for (const preset of PAN_PRESETS) {
      expect(nextPan(preset.value, preset.value)).toBeNull()
    }
  })

  it("counts a difference smaller than 0.001 as a match", () => {
    for (const preset of PAN_PRESETS) {
      expect(nextPan(preset.value - 0.0009, preset.value)).toBeNull()
      expect(nextPan(preset.value + 0.0009, preset.value)).toBeNull()
    }
  })

  it("returns the preset when it differs", () => {
    for (const preset of PAN_PRESETS) {
      expect(nextPan(preset.value - 0.002, preset.value)).toBe(preset.value)
      expect(nextPan(preset.value + 0.002, preset.value)).toBe(preset.value)
    }
    expect(nextPan(0.001, 0)).toBe(0)
    expect(nextPan(-0.001, 0)).toBe(0)
  })
})
