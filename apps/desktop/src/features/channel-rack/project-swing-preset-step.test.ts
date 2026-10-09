import { describe, expect, it } from "vitest"

import { nextProjectSwingPreset } from "./project-swing-preset-step"
import { SWING_PRESETS } from "./swing-presets"

describe("project swing preset stepping", () => {
  it("orders presets from straight to full, including heavy", () => {
    expect(SWING_PRESETS.map((preset) => preset.value)).toEqual([
      0, 0.25, 0.5, 0.75, 1,
    ])
  })

  it.each([
    { swing: 0, previous: null, next: 0.25 },
    { swing: 0.25, previous: 0, next: 0.5 },
    { swing: 0.5, previous: 0.25, next: 0.75 },
    { swing: 0.75, previous: 0.5, next: 1 },
    { swing: 1, previous: 0.75, next: null },
    { swing: 0.6, previous: 0.5, next: 0.75 },
    { swing: -0.1, previous: null, next: 0 },
    { swing: 1.2, previous: 1, next: null },
    { swing: 0.5004, previous: 0.25, next: 0.75 },
    { swing: 0.4996, previous: 0.25, next: 0.75 },
    { swing: NaN, previous: null, next: null },
    { swing: Infinity, previous: null, next: null },
    { swing: -Infinity, previous: null, next: null },
  ])(
    "steps from $swing to previous $previous and next $next",
    ({ swing, previous, next }) => {
      expect(nextProjectSwingPreset(swing, "previous")).toBe(previous)
      expect(nextProjectSwingPreset(swing, "next")).toBe(next)
    }
  )
})
