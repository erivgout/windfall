import { describe, expect, it } from "vitest"

import { nextTempoPreset } from "./tempo-preset-step"
import { TEMPO_PRESETS } from "./tempo-presets"

describe("tempo preset stepping", () => {
  it("starts at 80 BPM and ends at 174 BPM", () => {
    expect(TEMPO_PRESETS[0]).toBe(80)
    expect(TEMPO_PRESETS[TEMPO_PRESETS.length - 1]).toBe(174)
  })

  it.each([
    { tempo: 80, previous: null, next: 100 },
    { tempo: 100, previous: 80, next: 120 },
    { tempo: 120, previous: 100, next: 128 },
    { tempo: 174, previous: 160, next: null },
    { tempo: 90, previous: 80, next: 100 },
    { tempo: 70, previous: null, next: 80 },
    { tempo: 200, previous: 174, next: null },
    { tempo: 120.0005, previous: 100, next: 128 },
    { tempo: 119.9995, previous: 100, next: 128 },
    { tempo: NaN, previous: null, next: null },
    { tempo: Infinity, previous: null, next: null },
    { tempo: -Infinity, previous: null, next: null },
  ])(
    "steps from $tempo to previous $previous and next $next",
    ({ tempo, previous, next }) => {
      expect(nextTempoPreset(tempo, "previous")).toBe(previous)
      expect(nextTempoPreset(tempo, "next")).toBe(next)
    }
  )
})
