import { describe, expect, it } from "vitest"

import { nextSampleGainPreset } from "./sample-gain-preset-step"
import { SAMPLE_GAIN_PRESETS } from "./sample-gain-presets"

describe("sample gain preset step", () => {
  it("starts at Quiet and ends at Loud", () => {
    expect(SAMPLE_GAIN_PRESETS[0].gain).toBe(0.5)
    expect(SAMPLE_GAIN_PRESETS[SAMPLE_GAIN_PRESETS.length - 1].gain).toBe(1.5)
  })

  it.each([
    { gain: 0.5, previous: null, next: 1 },
    { gain: 1, previous: 0.5, next: 1.5 },
    { gain: 1.5, previous: 1, next: null },
    { gain: 0.7, previous: 0.5, next: 1 },
    { gain: 0.2, previous: null, next: 0.5 },
    { gain: 2, previous: 1.5, next: null },
    { gain: 1.0004, previous: 0.5, next: 1.5 },
  ])("steps gain $gain to its neighboring presets", ({ gain, previous, next }) => {
    expect(nextSampleGainPreset(gain, "previous")).toBe(previous)
    expect(nextSampleGainPreset(gain, "next")).toBe(next)
  })

  it.each([NaN, Infinity, -Infinity])(
    "returns null in both directions for non-finite gain %s",
    (gain) => {
      expect(nextSampleGainPreset(gain, "previous")).toBeNull()
      expect(nextSampleGainPreset(gain, "next")).toBeNull()
    }
  )
})
