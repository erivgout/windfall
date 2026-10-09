import { describe, expect, it } from "vitest"

import { LOOP_REGION_PRESETS, nextLoopRegion } from "./loop-region-presets"

describe("loop region presets", () => {
  it("lists the four presets as fractions of the trimmed sample", () => {
    expect(LOOP_REGION_PRESETS).toEqual([
      { label: "Whole", start: 0, end: 1 },
      { label: "First half", start: 0, end: 0.5 },
      { label: "Second half", start: 0.5, end: 1 },
      { label: "Last quarter", start: 0.75, end: 1 },
    ])
  })

  it("returns null when both edges already match", () => {
    for (const preset of LOOP_REGION_PRESETS) {
      expect(nextLoopRegion(preset.start, preset.end, preset)).toBeNull()
    }
  })

  it("counts differences smaller than 0.001 as a match", () => {
    for (const preset of LOOP_REGION_PRESETS) {
      for (const startDifference of [-0.0009, 0, 0.0009]) {
        for (const endDifference of [-0.0009, 0, 0.0009]) {
          expect(
            nextLoopRegion(
              preset.start + startDifference,
              preset.end + endDifference,
              preset
            )
          ).toBeNull()
        }
      }
    }
  })

  it("returns the preset when either edge differs", () => {
    for (const preset of LOOP_REGION_PRESETS) {
      for (const difference of [-0.0011, 0.0011]) {
        expect(
          nextLoopRegion(preset.start + difference, preset.end, preset)
        ).toBe(preset)
        expect(
          nextLoopRegion(preset.start, preset.end + difference, preset)
        ).toBe(preset)
        expect(
          nextLoopRegion(
            preset.start + difference,
            preset.end + difference,
            preset
          )
        ).toBe(preset)
      }
    }
    const whole = LOOP_REGION_PRESETS[0]
    expect(nextLoopRegion(0.001, 1, whole)).toBe(whole)
    const firstHalf = LOOP_REGION_PRESETS[1]
    expect(nextLoopRegion(0, 0.501, firstHalf)).toBe(firstHalf)
  })

  it("does not mutate the preset", () => {
    const preset = Object.freeze({ start: 0.75, end: 1 })
    expect(nextLoopRegion(0, 0.5, preset)).toBe(preset)
    expect(nextLoopRegion(0.75, 1, preset)).toBeNull()
    expect(preset).toEqual({ start: 0.75, end: 1 })
  })
})
