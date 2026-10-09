import { describe, expect, it } from "vitest"

import { PPQ, TICKS_PER_STEP } from "@/lib/units"

import { nextGatePreset } from "./gate-preset-step"
import { GATE_PRESETS } from "./gate-presets"

describe("channel gate preset stepping", () => {
  it("starts at Written and ends at a quarter note", () => {
    expect(GATE_PRESETS[0].ticks).toBe(0)
    expect(GATE_PRESETS[GATE_PRESETS.length - 1].ticks).toBe(PPQ)
  })

  it.each([
    { ticks: 0, previous: null, next: TICKS_PER_STEP },
    { ticks: TICKS_PER_STEP, previous: 0, next: TICKS_PER_STEP * 2 },
    { ticks: TICKS_PER_STEP * 2, previous: TICKS_PER_STEP, next: PPQ },
    { ticks: PPQ, previous: TICKS_PER_STEP * 2, next: null },
    { ticks: TICKS_PER_STEP + 1, previous: TICKS_PER_STEP, next: TICKS_PER_STEP * 2 },
    { ticks: -1, previous: null, next: 0 },
    { ticks: PPQ + 1, previous: PPQ, next: null },
    { ticks: NaN, previous: null, next: null },
    { ticks: Infinity, previous: null, next: null },
    { ticks: -Infinity, previous: null, next: null },
  ])(
    "steps from $ticks to previous $previous and next $next",
    ({ ticks, previous, next }) => {
      expect(nextGatePreset(ticks, "previous")).toBe(previous)
      expect(nextGatePreset(ticks, "next")).toBe(next)
    }
  )

  it("treats values close to each preset as between presets", () => {
    for (const [index, item] of GATE_PRESETS.entries()) {
      expect(nextGatePreset(item.ticks - 0.0001, "previous")).toBe(
        GATE_PRESETS[index - 1]?.ticks ?? null
      )
      expect(nextGatePreset(item.ticks - 0.0001, "next")).toBe(item.ticks)
      expect(nextGatePreset(item.ticks + 0.0001, "previous")).toBe(item.ticks)
      expect(nextGatePreset(item.ticks + 0.0001, "next")).toBe(
        GATE_PRESETS[index + 1]?.ticks ?? null
      )
    }
  })
})
