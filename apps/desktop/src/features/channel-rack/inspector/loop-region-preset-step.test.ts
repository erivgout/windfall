import { describe, expect, it } from "vitest"

import { nextLoopRegionPreset } from "./loop-region-preset-step"
import { LOOP_REGION_PRESETS } from "./loop-region-presets"

function regionPair(preset: { start: number; end: number } | null) {
  return preset ? { start: preset.start, end: preset.end } : null
}

describe("loop region preset stepping", () => {
  it("starts at Whole and ends at Last quarter", () => {
    expect(LOOP_REGION_PRESETS[0].label).toBe("Whole")
    expect(LOOP_REGION_PRESETS[LOOP_REGION_PRESETS.length - 1].label).toBe(
      "Last quarter"
    )
  })

  it.each(
    LOOP_REGION_PRESETS.map((preset, index) => ({
      ...preset,
      previous: LOOP_REGION_PRESETS[index - 1] ?? null,
      next: LOOP_REGION_PRESETS[index + 1] ?? null,
    }))
  )("steps from $label in both directions without wrapping", (preset) => {
    expect(
      nextLoopRegionPreset(preset.start, preset.end, "previous")
    ).toEqual(regionPair(preset.previous))
    expect(nextLoopRegionPreset(preset.start, preset.end, "next")).toEqual(
      regionPair(preset.next)
    )
  })

  it("matches Whole with a start difference smaller than 0.001", () => {
    expect(nextLoopRegionPreset(0.0004, 1, "previous")).toBeNull()
    expect(nextLoopRegionPreset(0.0004, 1, "next")).toEqual(
      regionPair(LOOP_REGION_PRESETS[1])
    )
  })

  it.each([
    { start: 0, end: 0.501 },
    { start: 0.1, end: 0.9 },
    { start: NaN, end: 1 },
  ])("keeps an unmatched region $start to $end", ({ start, end }) => {
    expect(nextLoopRegionPreset(start, end, "previous")).toBeNull()
    expect(nextLoopRegionPreset(start, end, "next")).toBeNull()
  })

  it.each([
    { start: undefined, end: undefined },
    { start: undefined, end: 1 },
    { start: 0, end: undefined },
  ])("treats missing edges $start to $end as Whole", ({ start, end }) => {
    expect(
      nextLoopRegionPreset(start ?? 0, end ?? 1, "previous")
    ).toBeNull()
    expect(nextLoopRegionPreset(start ?? 0, end ?? 1, "next")).toEqual(
      regionPair(LOOP_REGION_PRESETS[1])
    )
  })
})
