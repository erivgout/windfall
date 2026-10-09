import { describe, expect, it } from "vitest"

import { nextNoteVelocityPreset } from "./velocity-preset-step"
import { VELOCITY_PRESETS } from "./velocity-presets"

describe("note velocity preset stepping", () => {
  it("starts at Soft 0.25 and ends at Full 1", () => {
    expect(VELOCITY_PRESETS[0].velocity).toBe(0.25)
    expect(VELOCITY_PRESETS[VELOCITY_PRESETS.length - 1].velocity).toBe(1)
  })

  it.each([
    { velocity: 0.25, previous: null, next: 0.5 },
    { velocity: 0.5, previous: 0.25, next: 0.8 },
    { velocity: 0.8, previous: 0.5, next: 1 },
    { velocity: 1, previous: 0.8, next: null },
    { velocity: 0.6, previous: 0.5, next: 0.8 },
    { velocity: 0.1, previous: null, next: 0.25 },
    { velocity: 1.2, previous: 1, next: null },
    { velocity: 0.5004, previous: 0.25, next: 0.8 },
    { velocity: 0.4996, previous: 0.25, next: 0.8 },
    { velocity: NaN, previous: null, next: null },
    { velocity: Infinity, previous: null, next: null },
    { velocity: -Infinity, previous: null, next: null },
  ])(
    "steps from $velocity to previous $previous and next $next",
    ({ velocity, previous, next }) => {
      expect(nextNoteVelocityPreset(velocity, "previous")).toBe(previous)
      expect(nextNoteVelocityPreset(velocity, "next")).toBe(next)
    }
  )
})
