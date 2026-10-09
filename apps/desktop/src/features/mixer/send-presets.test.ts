import { describe, expect, it } from "vitest"

import { MAX_GAIN } from "@/lib/units"

import { nextSendGain, SEND_PRESETS } from "./send-presets"

describe("send level presets", () => {
  it("lists Off, Quiet, Unity, and Loud with their values", () => {
    expect(SEND_PRESETS).toEqual([
      { label: "Off", value: 0 },
      { label: "Quiet", value: 0.5 },
      { label: "Unity", value: 1 },
      { label: "Loud", value: 1.5 },
    ])
    for (const preset of SEND_PRESETS) {
      expect(preset.value).toBeGreaterThanOrEqual(0)
      expect(preset.value).toBeLessThan(MAX_GAIN)
    }
  })

  it("returns null when the send is already there", () => {
    for (const preset of SEND_PRESETS) {
      expect(nextSendGain(preset.value, preset.value)).toBeNull()
    }
  })

  it("counts a difference smaller than 0.001 as a match", () => {
    for (const preset of SEND_PRESETS) {
      expect(nextSendGain(preset.value - 0.0009, preset.value)).toBeNull()
      expect(nextSendGain(preset.value + 0.0009, preset.value)).toBeNull()
    }
  })

  it("returns the preset when it differs", () => {
    for (const preset of SEND_PRESETS) {
      expect(nextSendGain(MAX_GAIN, preset.value)).toBe(preset.value)
      expect(nextSendGain(preset.value - 0.002, preset.value)).toBe(
        preset.value
      )
      expect(nextSendGain(preset.value + 0.002, preset.value)).toBe(
        preset.value
      )
    }
    expect(nextSendGain(0.001, 0)).toBe(0)
  })
})
