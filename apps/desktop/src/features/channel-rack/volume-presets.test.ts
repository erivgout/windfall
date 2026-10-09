import { describe, expect, it } from "vitest"

import { MAX_GAIN } from "@/lib/units"

import { CHANNEL_VOLUME_PRESETS, nextChannelVolume } from "./volume-presets"

describe("channel volume presets", () => {
  it("lists Quiet, Default, Unity, and Loud with their values", () => {
    expect(CHANNEL_VOLUME_PRESETS).toEqual([
      { label: "Quiet", value: 0.5 },
      { label: "Default", value: 0.8 },
      { label: "Unity", value: 1 },
      { label: "Loud", value: 1.5 },
    ])
    for (const preset of CHANNEL_VOLUME_PRESETS) {
      expect(preset.value).toBeGreaterThan(0)
      expect(preset.value).toBeLessThan(MAX_GAIN)
    }
  })

  it("returns null when the channel is already that loud", () => {
    for (const preset of CHANNEL_VOLUME_PRESETS) {
      expect(nextChannelVolume(preset.value, preset.value)).toBeNull()
    }
  })

  it("counts a difference smaller than 0.001 as a match", () => {
    for (const preset of CHANNEL_VOLUME_PRESETS) {
      expect(nextChannelVolume(preset.value - 0.0009, preset.value)).toBeNull()
      expect(nextChannelVolume(preset.value + 0.0009, preset.value)).toBeNull()
    }
  })

  it("returns the preset when it differs", () => {
    for (const preset of CHANNEL_VOLUME_PRESETS) {
      expect(nextChannelVolume(0, preset.value)).toBe(preset.value)
      expect(nextChannelVolume(preset.value - 0.002, preset.value)).toBe(
        preset.value
      )
      expect(nextChannelVolume(preset.value + 0.002, preset.value)).toBe(
        preset.value
      )
    }
  })
})
