import { describe, expect, it } from "vitest"

import { nextClipGainPreset } from "./clip-gain-preset-step"
import { GAIN_PRESETS } from "./gain-presets"

describe("clip gain preset stepping", () => {
  it("starts at Quiet 0.5 and ends at Loud 1.5", () => {
    expect(GAIN_PRESETS[0].gain).toBe(0.5)
    expect(GAIN_PRESETS[GAIN_PRESETS.length - 1].gain).toBe(1.5)
  })

  it.each([
    { gain: 0.5, previous: null, next: 1 },
    { gain: 1, previous: 0.5, next: 1.5 },
    { gain: 1.5, previous: 1, next: null },
    { gain: 0.7, previous: 0.5, next: 1 },
    { gain: 0.2, previous: null, next: 0.5 },
    { gain: 2, previous: 1.5, next: null },
    { gain: 1.0004, previous: 0.5, next: 1.5 },
    { gain: 0.9996, previous: 0.5, next: 1.5 },
    { gain: 0.5004, previous: null, next: 1 },
    { gain: 1.4996, previous: 1, next: null },
    { gain: NaN, previous: null, next: null },
    { gain: Infinity, previous: null, next: null },
    { gain: -Infinity, previous: null, next: null },
  ])(
    "steps from $gain to previous $previous and next $next",
    ({ gain, previous, next }) => {
      expect(nextClipGainPreset(gain, "previous")).toBe(previous)
      expect(nextClipGainPreset(gain, "next")).toBe(next)
    }
  )
})
