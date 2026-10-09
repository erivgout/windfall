import { describe, expect, it } from "vitest"

import { nextTempo, TEMPO_PRESETS } from "./tempo-presets"

describe("tempo presets", () => {
  it("lists exactly the seven presets in order", () => {
    expect(TEMPO_PRESETS).toEqual([80, 100, 120, 128, 140, 160, 174])
  })

  it("returns null when the current tempo matches", () => {
    for (const preset of TEMPO_PRESETS) {
      expect(nextTempo(preset, preset)).toBeNull()
    }
  })

  it("returns the preset when the current tempo differs", () => {
    for (const preset of TEMPO_PRESETS) {
      expect(nextTempo(90, preset)).toBe(preset)
    }
  })

  it("counts a difference smaller than 0.001 as a match", () => {
    expect(nextTempo(120.0005, 120)).toBeNull()
    expect(nextTempo(119.9995, 120)).toBeNull()
  })
})
