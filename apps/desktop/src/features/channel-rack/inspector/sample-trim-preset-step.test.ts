import { describe, expect, it } from "vitest"

import { nextSampleTrimPreset } from "./sample-trim-preset-step"
import { SAMPLE_TRIM_PRESETS } from "./sample-trim-presets"

function trimPair(preset: { start: number; end: number } | null) {
  return preset ? { start: preset.start, end: preset.end } : null
}

describe("sample trim preset stepping", () => {
  it("starts at Whole and ends at Middle", () => {
    expect(SAMPLE_TRIM_PRESETS[0].label).toBe("Whole")
    expect(SAMPLE_TRIM_PRESETS[SAMPLE_TRIM_PRESETS.length - 1].label).toBe(
      "Middle"
    )
  })

  it.each(
    SAMPLE_TRIM_PRESETS.map((preset, index) => ({
      ...preset,
      previous: SAMPLE_TRIM_PRESETS[index - 1] ?? null,
      next: SAMPLE_TRIM_PRESETS[index + 1] ?? null,
    }))
  )("steps from $label in both directions without wrapping", (preset) => {
    expect(
      nextSampleTrimPreset(preset.start, preset.end, "previous")
    ).toEqual(trimPair(preset.previous))
    expect(nextSampleTrimPreset(preset.start, preset.end, "next")).toEqual(
      trimPair(preset.next)
    )
  })

  it("matches Whole with a start difference smaller than 0.001", () => {
    expect(nextSampleTrimPreset(0.0004, 1, "previous")).toBeNull()
    expect(nextSampleTrimPreset(0.0004, 1, "next")).toEqual(
      trimPair(SAMPLE_TRIM_PRESETS[1])
    )
  })

  it.each([
    { start: 0, end: 0.501 },
    { start: 0.1, end: 0.9 },
    { start: NaN, end: 1 },
  ])("keeps an unmatched trim $start to $end", ({ start, end }) => {
    expect(nextSampleTrimPreset(start, end, "previous")).toBeNull()
    expect(nextSampleTrimPreset(start, end, "next")).toBeNull()
  })

  it.each([
    { start: undefined, end: undefined },
    { start: undefined, end: 1 },
    { start: 0, end: undefined },
  ])("treats missing edges $start to $end as Whole", ({ start, end }) => {
    expect(
      nextSampleTrimPreset(start ?? 0, end ?? 1, "previous")
    ).toBeNull()
    expect(nextSampleTrimPreset(start ?? 0, end ?? 1, "next")).toEqual(
      trimPair(SAMPLE_TRIM_PRESETS[1])
    )
  })
})
