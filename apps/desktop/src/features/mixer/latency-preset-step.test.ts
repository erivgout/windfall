import { describe, expect, it } from "vitest"

import { LATENCY_PRESETS } from "./latency-presets"
import { nextLatencyPreset } from "./latency-preset-step"

describe("latency preset stepping", () => {
  it("walks presets from None to 10 ms", () => {
    expect(LATENCY_PRESETS[0].value).toBe(0)
    expect(LATENCY_PRESETS[LATENCY_PRESETS.length - 1].value).toBe(10)
  })

  it.each([
    [0, null, 1],
    [1, 0, 5],
    [5, 1, 10],
    [10, 5, null],
    [0.0004, null, 1],
    [0.999, 0, 1],
    [3, 1, 5],
    [7, 5, 10],
    [-1, null, 0],
    [11, 10, null],
    [NaN, null, null],
    [Infinity, null, null],
    [-Infinity, null, null],
  ] as const)("steps %s ms previous to %s and next to %s", (offset, previous, next) => {
    expect(nextLatencyPreset(offset, "previous")).toBe(previous)
    expect(nextLatencyPreset(offset, "next")).toBe(next)
  })

  it.each(LATENCY_PRESETS)("counts corrections near $label as that preset", (preset) => {
    for (const offset of [preset.value - 0.0009, preset.value + 0.0009]) {
      expect(nextLatencyPreset(offset, "previous")).toBe(nextLatencyPreset(preset.value, "previous"))
      expect(nextLatencyPreset(offset, "next")).toBe(nextLatencyPreset(preset.value, "next"))
    }
  })
})
