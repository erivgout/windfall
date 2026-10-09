import { describe, expect, it } from "vitest"

import { GATE_PRESETS, nextGate } from "./gate-presets"

describe("gate presets", () => {
  it("lists the four presets in whole ticks", () => {
    expect(GATE_PRESETS).toEqual([
      { label: "Written", ticks: 0 },
      { label: "16th", ticks: 240 },
      { label: "8th", ticks: 480 },
      { label: "Quarter", ticks: 960 },
    ])
  })

  it("returns null when the gate is already that length", () => {
    for (const { ticks } of GATE_PRESETS) {
      expect(nextGate(ticks, ticks)).toBeNull()
    }
  })

  it("returns the preset when the gate differs", () => {
    for (const { ticks: preset } of GATE_PRESETS) {
      for (const current of [0, 1, 240, 480, 960]) {
        if (current !== preset) {
          expect(nextGate(current, preset)).toBe(preset)
        }
      }
    }
  })
})
