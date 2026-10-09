import { describe, expect, it } from "vitest"

import { MAX_PATTERN_STEPS, TICKS_PER_STEP } from "@/lib/units"
import { nextWaveformStartScale, scaledWaveformStart } from "./waveform-start-scale"

const limit = MAX_PATTERN_STEPS * TICKS_PER_STEP

describe("waveform helper start scaling", () => {
  it.each([
    [0, "half", 0, null],
    [0, "double", 0, null],
    [limit, "double", limit, null],
    [-limit, "double", -limit, null],
    [1, "half", 0, 0],
    [-1, "half", 0, 0],
    [240, "half", 120, 120],
    [240, "double", 480, 480],
    [-240, "half", -120, -120],
    [-240, "double", -480, -480],
    [5, "half", 2, 2],
    [-5, "half", -2, -2],
    [limit - 100, "double", limit, limit],
    [-(limit - 100), "double", -limit, -limit],
  ] as const)(
    "scales %i ticks by %s to %i and returns %s as the next start",
    (start, factor, scaled, next) => {
      expect(scaledWaveformStart(start, factor)).toBe(scaled)
      expect(nextWaveformStartScale(start, factor)).toBe(next)
    }
  )
})
