import { describe, expect, it } from "vitest"

import { nextFaderPreset } from "./fader-preset-step"
import { FADER_PRESETS } from "./fader-presets"

describe("fader preset stepping", () => {
  it("starts at Quiet 0.5 and ends at Loud 1.5", () => {
    expect(FADER_PRESETS[0].value).toBe(0.5)
    expect(FADER_PRESETS[FADER_PRESETS.length - 1].value).toBe(1.5)
  })

  it.each([
    { volume: 0.5, previous: null, next: 1 },
    { volume: 1, previous: 0.5, next: 1.5 },
    { volume: 1.5, previous: 1, next: null },
    { volume: 0.7, previous: 0.5, next: 1 },
    { volume: 0.2, previous: null, next: 0.5 },
    { volume: 2, previous: 1.5, next: null },
    { volume: 1.0004, previous: 0.5, next: 1.5 },
    { volume: 0.9996, previous: 0.5, next: 1.5 },
    { volume: NaN, previous: null, next: null },
    { volume: Infinity, previous: null, next: null },
    { volume: -Infinity, previous: null, next: null },
  ])(
    "steps from $volume to previous $previous and next $next",
    ({ volume, previous, next }) => {
      expect(nextFaderPreset(volume, "previous")).toBe(previous)
      expect(nextFaderPreset(volume, "next")).toBe(next)
    }
  )
})
