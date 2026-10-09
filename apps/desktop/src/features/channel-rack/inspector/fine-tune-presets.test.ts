import { describe, expect, it } from "vitest"

import { FINE_TUNE_PRESETS, nextSamplerFine } from "./fine-tune-presets"

describe("sampler fine-tune presets", () => {
  it("lists the five presets in cents", () => {
    expect(FINE_TUNE_PRESETS).toEqual([
      { label: "−50", cents: -50 },
      { label: "−25", cents: -25 },
      { label: "In tune", cents: 0 },
      { label: "+25", cents: 25 },
      { label: "+50", cents: 50 },
    ])
  })

  it("sets 25 cents on a tune of zero", () => {
    expect(nextSamplerFine(0, 0, 25)).toBe(0.25)
  })

  it("returns null when zero cents already matches", () => {
    expect(nextSamplerFine(0, 0, 0)).toBeNull()
  })

  it("removes cents while keeping the shown semitone", () => {
    expect(nextSamplerFine(3.1, 3, 0)).toBe(3)
  })

  it("does not apply a preset beyond the tune limit", () => {
    expect(nextSamplerFine(48, 48, 50)).toBeNull()
  })

  it("returns null when the shown cents already match the preset", () => {
    for (const { cents } of FINE_TUNE_PRESETS) {
      expect(nextSamplerFine(3 + cents / 100, 3, cents)).toBeNull()
    }
  })
})
