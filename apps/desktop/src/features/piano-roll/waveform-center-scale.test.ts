import { describe, expect, it } from "vitest"

import { DEFAULT_KEY } from "@/lib/units"
import { nextWaveformCenterScale, scaledWaveformCenter } from "./waveform-center-scale"

describe("waveform helper center key scaling", () => {
  it.each([
    [DEFAULT_KEY, "half", DEFAULT_KEY, null],
    [DEFAULT_KEY, "double", DEFAULT_KEY, null],
    [DEFAULT_KEY + 12, "half", 66, 66],
    [DEFAULT_KEY + 12, "double", DEFAULT_KEY + 24, DEFAULT_KEY + 24],
    [DEFAULT_KEY - 12, "half", DEFAULT_KEY - 6, DEFAULT_KEY - 6],
    [DEFAULT_KEY - 12, "double", 36, 36],
    [DEFAULT_KEY + 1, "half", DEFAULT_KEY, DEFAULT_KEY],
    [DEFAULT_KEY + 1, "double", DEFAULT_KEY + 2, DEFAULT_KEY + 2],
    [DEFAULT_KEY - 1, "half", DEFAULT_KEY, DEFAULT_KEY],
    [DEFAULT_KEY - 1, "double", DEFAULT_KEY - 2, DEFAULT_KEY - 2],
    [0, "double", 0, null],
    [127, "double", 127, null],
    [DEFAULT_KEY + 36, "double", 127, 127],
    [DEFAULT_KEY - 36, "double", 0, 0],
  ] as const)(
    "scales MIDI key %i by %s to %i and returns %s as the next center key",
    (key, factor, scaled, next) => {
      expect(scaledWaveformCenter(key, factor)).toBe(scaled)
      expect(nextWaveformCenterScale(key, factor)).toBe(next)
    }
  )
})
