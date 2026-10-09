import { describe, expect, it } from "vitest"

import { METRONOME_GAINS } from "./metronome-gain"
import { nextMetronomePreset } from "./metronome-preset-step"

describe("metronome preset stepping", () => {
  it("starts at Quiet gain 0.25 and ends at Loud gain 1", () => {
    expect(METRONOME_GAINS[0].value).toBe(0.25)
    expect(METRONOME_GAINS[METRONOME_GAINS.length - 1].value).toBe(1)
  })

  it.each([
    { gain: 0.25, previous: null, next: 0.5 },
    { gain: 0.5, previous: 0.25, next: 1 },
    { gain: 1, previous: 0.5, next: null },
    { gain: 0.3, previous: 0.25, next: 0.5 },
    { gain: 0.1, previous: null, next: 0.25 },
    { gain: 1.5, previous: 1, next: null },
    { gain: 0.5004, previous: 0.25, next: 1 },
    { gain: 0.4996, previous: 0.25, next: 1 },
    { gain: 0.2496, previous: null, next: 0.5 },
    { gain: 1.0004, previous: 0.5, next: null },
    { gain: NaN, previous: null, next: null },
    { gain: Infinity, previous: null, next: null },
    { gain: -Infinity, previous: null, next: null },
  ])(
    "steps from $gain to previous $previous and next $next",
    ({ gain, previous, next }) => {
      expect(nextMetronomePreset(gain, "previous")).toBe(previous)
      expect(nextMetronomePreset(gain, "next")).toBe(next)
    }
  )
})
