import { describe, expect, it } from "vitest"

import { nextMixPreset } from "./mix-preset-step"
import { MIX_PRESETS } from "./mix-presets"

describe("mix preset stepping", () => {
  it("starts at Dry and ends at Wet", () => {
    expect(MIX_PRESETS[0].value).toBe(0)
    expect(MIX_PRESETS[MIX_PRESETS.length - 1].value).toBe(1)
  })

  it.each([
    { mix: 0, previous: null, next: 0.25 },
    { mix: 0.25, previous: 0, next: 0.5 },
    { mix: 0.5, previous: 0.25, next: 0.75 },
    { mix: 0.75, previous: 0.5, next: 1 },
    { mix: 1, previous: 0.75, next: null },
    { mix: 0.6, previous: 0.5, next: 0.75 },
    { mix: -0.1, previous: null, next: 0 },
    { mix: 1.2, previous: 1, next: null },
    { mix: 0.5004, previous: 0.25, next: 0.75 },
    { mix: NaN, previous: null, next: null },
    { mix: Infinity, previous: null, next: null },
    { mix: -Infinity, previous: null, next: null },
  ])(
    "steps from $mix to previous $previous and next $next",
    ({ mix, previous, next }) => {
      expect(nextMixPreset(mix, "previous")).toBe(previous)
      expect(nextMixPreset(mix, "next")).toBe(next)
    }
  )

  it("matches every preset within the tolerance from either side", () => {
    for (const [index, item] of MIX_PRESETS.entries()) {
      for (const offset of [-0.0009, 0.0009]) {
        expect(nextMixPreset(item.value + offset, "previous")).toBe(
          MIX_PRESETS[index - 1]?.value ?? null
        )
        expect(nextMixPreset(item.value + offset, "next")).toBe(
          MIX_PRESETS[index + 1]?.value ?? null
        )
      }
    }
  })

  it("does not match Dry at the tolerance boundary", () => {
    expect(nextMixPreset(0.001, "previous")).toBe(0)
    expect(nextMixPreset(0.001, "next")).toBe(0.25)
    expect(nextMixPreset(-0.001, "previous")).toBeNull()
    expect(nextMixPreset(-0.001, "next")).toBe(0)
  })
})
