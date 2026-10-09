import { describe, expect, it } from "vitest"

import { nextWaveformOpacityScale, scaledWaveformOpacity } from "./waveform-opacity-scale"

describe("waveform helper opacity scaling", () => {
  it.each([
    [0.18, "half", 0.09, 0.09],
    [0.18, "double", 0.36, 0.36],
    [0.2, "half", 0.1, 0.1],
    [0.2, "double", 0.4, 0.4],
    [0.4, "double", 0.6, 0.6],
    [0.03, "half", 0.015, 0.015],
    [0.01, "half", 0.01, null],
    [0.6, "double", 0.6, null],
    [0.0105, "half", 0.01, null],
    [0.5995, "double", 0.6, null],
  ] as const)(
    "scales opacity %s by %s to %s and returns %s as the next opacity",
    (opacity, factor, scaled, next) => {
      expect(scaledWaveformOpacity(opacity, factor)).toBe(scaled)
      expect(nextWaveformOpacityScale(opacity, factor)).toBe(next)
    }
  )
})
