import { describe, expect, it } from "vitest"

import { nextSampleTrim, SAMPLE_TRIM_PRESETS } from "./sample-trim-presets"

describe("sample trim presets", () => {
  it("lists the four presets as fractions of the sample", () => {
    expect(SAMPLE_TRIM_PRESETS).toEqual([
      { label: "Whole", start: 0, end: 1 },
      { label: "First half", start: 0, end: 0.5 },
      { label: "Second half", start: 0.5, end: 1 },
      { label: "Middle", start: 0.25, end: 0.75 },
    ])
  })

  it("returns null when both edges already match", () => {
    for (const preset of SAMPLE_TRIM_PRESETS) {
      expect(nextSampleTrim(preset.start, preset.end, preset)).toBeNull()
    }
  })

  it("counts differences smaller than 0.001 as a match", () => {
    for (const preset of SAMPLE_TRIM_PRESETS) {
      for (const startDifference of [-0.0009, 0, 0.0009]) {
        for (const endDifference of [-0.0009, 0, 0.0009]) {
          expect(
            nextSampleTrim(
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
    for (const preset of SAMPLE_TRIM_PRESETS) {
      for (const difference of [-0.0011, 0.0011]) {
        expect(
          nextSampleTrim(preset.start + difference, preset.end, preset)
        ).toBe(preset)
        expect(
          nextSampleTrim(preset.start, preset.end + difference, preset)
        ).toBe(preset)
        expect(
          nextSampleTrim(
            preset.start + difference,
            preset.end + difference,
            preset
          )
        ).toBe(preset)
      }
    }
    const whole = SAMPLE_TRIM_PRESETS[0]
    expect(nextSampleTrim(0.001, 1, whole)).toBe(whole)
    const firstHalf = SAMPLE_TRIM_PRESETS[1]
    expect(nextSampleTrim(0, 0.501, firstHalf)).toBe(firstHalf)
  })
})
