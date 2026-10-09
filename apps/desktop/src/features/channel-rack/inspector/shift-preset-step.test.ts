import { describe, expect, it } from "vitest"

import { TICKS_PER_STEP } from "@/lib/units"

import { nextShiftPreset } from "./shift-preset-step"
import { SHIFT_PRESETS } from "./shift-presets"

describe("channel shift preset stepping", () => {
  it("starts at a 16th early and ends at an 8th late", () => {
    expect(SHIFT_PRESETS[0].ticks).toBe(-TICKS_PER_STEP)
    expect(SHIFT_PRESETS[SHIFT_PRESETS.length - 1].ticks).toBe(TICKS_PER_STEP * 2)
  })

  it.each([
    { ticks: -TICKS_PER_STEP, previous: null, next: 0 },
    { ticks: 0, previous: -TICKS_PER_STEP, next: TICKS_PER_STEP },
    { ticks: TICKS_PER_STEP, previous: 0, next: TICKS_PER_STEP * 2 },
    { ticks: TICKS_PER_STEP * 2, previous: TICKS_PER_STEP, next: null },
    { ticks: -1, previous: -TICKS_PER_STEP, next: 0 },
    { ticks: TICKS_PER_STEP + 1, previous: TICKS_PER_STEP, next: TICKS_PER_STEP * 2 },
    { ticks: -(TICKS_PER_STEP + 1), previous: null, next: -TICKS_PER_STEP },
    { ticks: TICKS_PER_STEP * 2 + 1, previous: TICKS_PER_STEP * 2, next: null },
    { ticks: NaN, previous: null, next: null },
    { ticks: Infinity, previous: null, next: null },
    { ticks: -Infinity, previous: null, next: null },
  ])(
    "steps from $ticks to previous $previous and next $next",
    ({ ticks, previous, next }) => {
      expect(nextShiftPreset(ticks, "previous")).toBe(previous)
      expect(nextShiftPreset(ticks, "next")).toBe(next)
    }
  )

  it("treats values close to each preset as between presets", () => {
    for (const [index, item] of SHIFT_PRESETS.entries()) {
      expect(nextShiftPreset(item.ticks - 0.0001, "previous")).toBe(
        SHIFT_PRESETS[index - 1]?.ticks ?? null
      )
      expect(nextShiftPreset(item.ticks - 0.0001, "next")).toBe(item.ticks)
      expect(nextShiftPreset(item.ticks + 0.0001, "previous")).toBe(item.ticks)
      expect(nextShiftPreset(item.ticks + 0.0001, "next")).toBe(
        SHIFT_PRESETS[index + 1]?.ticks ?? null
      )
    }
  })
})
