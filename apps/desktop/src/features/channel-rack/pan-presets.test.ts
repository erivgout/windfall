import { describe, expect, it } from "vitest"

import { CHANNEL_PAN_PRESETS, nextChannelPan } from "./pan-presets"

describe("channel pan presets", () => {
  it("lists Hard left, Left, Center, Right, and Hard right with their values", () => {
    expect(CHANNEL_PAN_PRESETS).toEqual([
      { label: "Hard left", value: -1 },
      { label: "Left", value: -0.5 },
      { label: "Center", value: 0 },
      { label: "Right", value: 0.5 },
      { label: "Hard right", value: 1 },
    ])
  })

  it("returns null when the knob is already there", () => {
    for (const preset of CHANNEL_PAN_PRESETS) {
      expect(nextChannelPan(preset.value, preset.value)).toBeNull()
    }
  })

  it("counts a difference smaller than 0.001 as a match", () => {
    for (const preset of CHANNEL_PAN_PRESETS) {
      expect(nextChannelPan(preset.value - 0.0009, preset.value)).toBeNull()
      expect(nextChannelPan(preset.value + 0.0009, preset.value)).toBeNull()
    }
  })

  it("returns the preset when it differs", () => {
    for (const preset of CHANNEL_PAN_PRESETS) {
      expect(nextChannelPan(preset.value - 0.002, preset.value)).toBe(preset.value)
      expect(nextChannelPan(preset.value + 0.002, preset.value)).toBe(preset.value)
    }
    expect(nextChannelPan(0.001, 0)).toBe(0)
    expect(nextChannelPan(-0.001, 0)).toBe(0)
  })
})
