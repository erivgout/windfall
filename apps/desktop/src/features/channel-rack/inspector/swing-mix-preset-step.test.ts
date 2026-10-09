import { describe, expect, it } from "vitest"

import { nextSwingMixPreset } from "./swing-mix-preset-step"
import { SWING_PRESETS } from "./swing-presets"

describe("channel swing mix preset stepping", () => {
  it("starts at Straight and ends at Full", () => {
    expect(SWING_PRESETS[0].value).toBe(0)
    expect(SWING_PRESETS[SWING_PRESETS.length - 1].value).toBe(1)
  })

  it("does not include a 0.75 preset", () => {
    expect(SWING_PRESETS.map((item) => item.value)).not.toContain(0.75)
  })

  it.each([
    { mix: 0, previous: null, next: 0.25 },
    { mix: 0.25, previous: 0, next: 0.5 },
    { mix: 0.5, previous: 0.25, next: 1 },
    { mix: 1, previous: 0.5, next: null },
    { mix: 0.3, previous: 0.25, next: 0.5 },
    { mix: 0.6, previous: 0.5, next: 1 },
    { mix: -0.1, previous: null, next: 0 },
    { mix: 1.2, previous: 1, next: null },
    { mix: 0.5004, previous: 0.25, next: 1 },
    { mix: NaN, previous: null, next: null },
    { mix: Infinity, previous: null, next: null },
    { mix: -Infinity, previous: null, next: null },
  ])(
    "steps from $mix to previous $previous and next $next",
    ({ mix, previous, next }) => {
      expect(nextSwingMixPreset(mix, "previous")).toBe(previous)
      expect(nextSwingMixPreset(mix, "next")).toBe(next)
    }
  )

  it("matches every preset within the tolerance from either side", () => {
    for (const [index, item] of SWING_PRESETS.entries()) {
      for (const offset of [-0.0009, 0.0009]) {
        expect(nextSwingMixPreset(item.value + offset, "previous")).toBe(
          SWING_PRESETS[index - 1]?.value ?? null
        )
        expect(nextSwingMixPreset(item.value + offset, "next")).toBe(
          SWING_PRESETS[index + 1]?.value ?? null
        )
      }
    }
  })

  it("uses the same strict tolerance boundary as nextSwing", () => {
    expect(nextSwingMixPreset(0.001, "previous")).toBe(0)
    expect(nextSwingMixPreset(0.001, "next")).toBe(0.25)
    expect(nextSwingMixPreset(-0.001, "previous")).toBeNull()
    expect(nextSwingMixPreset(-0.001, "next")).toBe(0)
  })
})
