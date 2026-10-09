import { describe, expect, it } from "vitest"

import {
  LOOP_CROSSFADE_PRESETS,
  nextLoopCrossfade,
} from "./loop-crossfade-presets"

describe("loop crossfade presets", () => {
  it("lists the four presets", () => {
    expect(LOOP_CROSSFADE_PRESETS).toEqual([
      { label: "None", crossfade: 0 },
      { label: "Short", crossfade: 0.25 },
      { label: "Medium", crossfade: 0.5 },
      { label: "Long", crossfade: 1 },
    ])
  })

  it("returns null when the crossfade is already that amount", () => {
    for (const { crossfade } of LOOP_CROSSFADE_PRESETS) {
      expect(nextLoopCrossfade(crossfade, crossfade)).toBeNull()
    }
  })

  it("counts a difference smaller than 0.001 as a match", () => {
    for (const difference of [-0.0009, 0.0009]) {
      expect(nextLoopCrossfade(0.5 + difference, 0.5)).toBeNull()
    }
  })

  it("returns the preset when the crossfade differs", () => {
    for (const { crossfade: preset } of LOOP_CROSSFADE_PRESETS) {
      for (const current of [0, 0.25, 0.5, 0.75, 1]) {
        if (current !== preset) {
          expect(nextLoopCrossfade(current, preset)).toBe(preset)
        }
      }
    }
    expect(nextLoopCrossfade(0.001, 0)).toBe(0)
  })
})
