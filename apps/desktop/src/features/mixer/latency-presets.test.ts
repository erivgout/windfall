import { describe, expect, it } from "vitest"

import { LATENCY_PRESETS, nextLatency } from "./latency-presets"

describe("latency presets", () => {
  it("lists None, 1 ms, 5 ms, and 10 ms in milliseconds", () => {
    expect(LATENCY_PRESETS).toEqual([
      { label: "None", value: 0 },
      { label: "1 ms", value: 1 },
      { label: "5 ms", value: 5 },
      { label: "10 ms", value: 10 },
    ])
  })

  it("returns null when the correction is already that amount", () => {
    for (const preset of LATENCY_PRESETS) {
      expect(nextLatency(preset.value, preset.value)).toBeNull()
    }
  })

  it("counts a difference smaller than 0.001 as a match", () => {
    for (const preset of LATENCY_PRESETS) {
      expect(nextLatency(preset.value - 0.0009, preset.value)).toBeNull()
      expect(nextLatency(preset.value + 0.0009, preset.value)).toBeNull()
    }
  })

  it("returns the preset when it differs", () => {
    for (const preset of LATENCY_PRESETS) {
      expect(nextLatency(preset.value - 0.002, preset.value)).toBe(preset.value)
      expect(nextLatency(preset.value + 0.002, preset.value)).toBe(preset.value)
      expect(nextLatency(-1000, preset.value)).toBe(preset.value)
      expect(nextLatency(1000, preset.value)).toBe(preset.value)
    }
    expect(nextLatency(0.001, 0)).toBe(0)
    expect(nextLatency(-0.001, 0)).toBe(0)
  })
})
