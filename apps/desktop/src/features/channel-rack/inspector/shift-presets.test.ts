import { describe, expect, it } from "vitest"

import { SHIFT_PRESETS, nextShift } from "./shift-presets"

describe("shift presets", () => {
  it("lists the four presets in whole ticks", () => {
    expect(SHIFT_PRESETS).toEqual([
      { label: "16th early", ticks: -240 },
      { label: "On grid", ticks: 0 },
      { label: "16th late", ticks: 240 },
      { label: "8th late", ticks: 480 },
    ])
  })

  it("returns null when the shift is already that amount", () => {
    for (const { ticks } of SHIFT_PRESETS) {
      expect(nextShift(ticks, ticks)).toBeNull()
    }
  })

  it("returns the preset when the shift differs", () => {
    for (const { ticks: preset } of SHIFT_PRESETS) {
      for (const current of [-960, -240, 0, 1, 240, 480, 960]) {
        if (current !== preset) {
          expect(nextShift(current, preset)).toBe(preset)
        }
      }
    }
  })
})
