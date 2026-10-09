import { describe, expect, it } from "vitest"

import { METRONOME_GAINS, nextMetronomeGain } from "./metronome-gain"

describe("metronome gain presets", () => {
  it("lists Quiet, Medium, and Loud in order with their gain values", () => {
    expect(METRONOME_GAINS).toEqual([
      { label: "Quiet", value: 0.25 },
      { label: "Medium", value: 0.5 },
      { label: "Loud", value: 1 },
    ])
  })

  it("returns null when the current gain is within 0.001 of the preset", () => {
    for (const { value } of METRONOME_GAINS) {
      expect(nextMetronomeGain(value, value)).toBeNull()
      expect(nextMetronomeGain(value - 0.0009, value)).toBeNull()
      expect(nextMetronomeGain(value + 0.0009, value)).toBeNull()
    }
  })

  it("returns the preset when the current gain is farther away", () => {
    for (const { value } of METRONOME_GAINS) {
      expect(nextMetronomeGain(value - 0.0011, value)).toBe(value)
      expect(nextMetronomeGain(value + 0.0011, value)).toBe(value)
    }
    expect(nextMetronomeGain(0, 0.001)).toBe(0.001)
  })

  it("does not match Quiet when the current gain is zero", () => {
    expect(nextMetronomeGain(0, METRONOME_GAINS[0].value)).toBe(0.25)
  })
})
