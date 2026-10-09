import { describe, expect, it } from "vitest"

import { nextSendPreset } from "./send-preset-step"
import { SEND_PRESETS } from "./send-presets"

describe("send preset steps", () => {
  it("starts at Off and ends at Loud", () => {
    expect(SEND_PRESETS[0].value).toBe(0)
    expect(SEND_PRESETS[SEND_PRESETS.length - 1].value).toBe(1.5)
  })

  it.each([
    { gain: 0, previous: null, next: 0.5 },
    { gain: 0.5, previous: 0, next: 1 },
    { gain: 1, previous: 0.5, next: 1.5 },
    { gain: 1.5, previous: 1, next: null },
    { gain: 0.7, previous: 0.5, next: 1 },
    { gain: -0.1, previous: null, next: 0 },
    { gain: 2, previous: 1.5, next: null },
    { gain: 0.5004, previous: 0, next: 1 },
  ])("steps from $gain without wrapping", ({ gain, previous, next }) => {
    expect(nextSendPreset(gain, "previous")).toBe(previous)
    expect(nextSendPreset(gain, "next")).toBe(next)
  })

  it.each(SEND_PRESETS)(
    "counts levels within tolerance on both sides of $label as that preset",
    ({ value }) => {
      for (const direction of ["previous", "next"] as const) {
        const expected = nextSendPreset(value, direction)
        expect(nextSendPreset(value - 0.0009, direction)).toBe(expected)
        expect(nextSendPreset(value + 0.0009, direction)).toBe(expected)
      }
    }
  )

  it.each([NaN, Infinity, -Infinity])(
    "returns null in both directions for %s",
    (gain) => {
      expect(nextSendPreset(gain, "previous")).toBeNull()
      expect(nextSendPreset(gain, "next")).toBeNull()
    }
  )
})
