import { describe, expect, it } from "vitest"

import { MAX_SONG_TICKS } from "@/lib/units"
import { nextWaveformLengthScale, scaledWaveformLength } from "./waveform-length-scale"

describe("waveform helper length scaling", () => {
  it.each([
    [3840, "half", 1920, 1920],
    [3840, "double", 7680, 7680],
    [5, "half", 2, 2],
    [3, "half", 1, 1],
    [MAX_SONG_TICKS - 100, "double", MAX_SONG_TICKS, MAX_SONG_TICKS],
    [1, "half", 1, null],
    [MAX_SONG_TICKS, "double", MAX_SONG_TICKS, null],
  ] as const)(
    "scales %i ticks by %s to %i and returns %s as the next length",
    (length, factor, scaled, next) => {
      expect(scaledWaveformLength(length, factor)).toBe(scaled)
      expect(nextWaveformLengthScale(length, factor)).toBe(next)
    }
  )
})
