import { describe, expect, it } from "vitest"

import { nextLoopCrossfadePreset } from "./crossfade-preset-step"
import { LOOP_CROSSFADE_PRESETS } from "./loop-crossfade-presets"

describe("loop crossfade preset step", () => {
  it("starts at None and ends at Long", () => {
    expect(LOOP_CROSSFADE_PRESETS[0].crossfade).toBe(0)
    expect(
      LOOP_CROSSFADE_PRESETS[LOOP_CROSSFADE_PRESETS.length - 1].crossfade
    ).toBe(1)
  })

  it.each([
    { crossfade: 0, previous: null, next: 0.25 },
    { crossfade: 0.25, previous: 0, next: 0.5 },
    { crossfade: 0.5, previous: 0.25, next: 1 },
    { crossfade: 1, previous: 0.5, next: null },
    { crossfade: 0.0004, previous: null, next: 0.25 },
    { crossfade: 0.249, previous: 0, next: 0.25 },
    { crossfade: 0.1, previous: 0, next: 0.25 },
    { crossfade: 0.3, previous: 0.25, next: 0.5 },
    { crossfade: 0.75, previous: 0.5, next: 1 },
    { crossfade: -0.1, previous: null, next: 0 },
    { crossfade: 1.1, previous: 1, next: null },
  ])(
    "steps crossfade $crossfade to its neighboring presets",
    ({ crossfade, previous, next }) => {
      expect(nextLoopCrossfadePreset(crossfade, "previous")).toBe(previous)
      expect(nextLoopCrossfadePreset(crossfade, "next")).toBe(next)
    }
  )

  it("treats a missing crossfade as None", () => {
    const crossfade: number | undefined = undefined
    expect(nextLoopCrossfadePreset(crossfade ?? 0, "previous")).toBeNull()
    expect(nextLoopCrossfadePreset(crossfade ?? 0, "next")).toBe(0.25)
  })

  it.each([NaN, Infinity, -Infinity])(
    "returns null in both directions for non-finite crossfade %s",
    (crossfade) => {
      expect(nextLoopCrossfadePreset(crossfade, "previous")).toBeNull()
      expect(nextLoopCrossfadePreset(crossfade, "next")).toBeNull()
    }
  )
})
