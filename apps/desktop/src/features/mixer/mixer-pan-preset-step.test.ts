import { describe, expect, it } from "vitest"

import { nextMixerPanPreset } from "./mixer-pan-preset-step"
import { PAN_PRESETS } from "./pan-presets"

describe("mixer pan preset stepping", () => {
  it("starts at hard left -1 and ends at hard right 1", () => {
    expect(PAN_PRESETS[0].value).toBe(-1)
    expect(PAN_PRESETS[PAN_PRESETS.length - 1].value).toBe(1)
  })

  it.each([
    { pan: -1, previous: null, next: -0.5 },
    { pan: -0.5, previous: -1, next: 0 },
    { pan: 0, previous: -0.5, next: 0.5 },
    { pan: 0.5, previous: 0, next: 1 },
    { pan: 1, previous: 0.5, next: null },
    { pan: -0.25, previous: -0.5, next: 0 },
    { pan: -1.5, previous: null, next: -1 },
    { pan: 1.5, previous: 1, next: null },
    { pan: 0.0004, previous: -0.5, next: 0.5 },
    { pan: -0.0004, previous: -0.5, next: 0.5 },
    { pan: NaN, previous: null, next: null },
    { pan: Infinity, previous: null, next: null },
    { pan: -Infinity, previous: null, next: null },
  ])(
    "steps from $pan to previous $previous and next $next",
    ({ pan, previous, next }) => {
      expect(nextMixerPanPreset(pan, "previous")).toBe(previous)
      expect(nextMixerPanPreset(pan, "next")).toBe(next)
    }
  )
})
