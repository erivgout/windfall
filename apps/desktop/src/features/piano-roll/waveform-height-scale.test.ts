import { describe, expect, it } from "vitest"

import { nextWaveformHeightScale, scaledWaveformHeight } from "./waveform-height-scale"

describe("waveform helper height scaling", () => {
  it.each([
    [24, "half", 12, 12],
    [24, "double", 48, 48],
    [5, "half", 2, 2],
    [3, "half", 2, 2],
    [65, "double", 128, 128],
    [2, "half", 2, null],
    [128, "double", 128, null],
  ] as const)(
    "scales %i rows by %s to %i and returns %s as the next height",
    (height, factor, scaled, next) => {
      expect(scaledWaveformHeight(height, factor)).toBe(scaled)
      expect(nextWaveformHeightScale(height, factor)).toBe(next)
    }
  )
})
