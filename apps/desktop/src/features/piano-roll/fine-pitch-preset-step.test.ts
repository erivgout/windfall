import { describe, expect, it } from "vitest"

import { nextNoteFinePitchPreset } from "./fine-pitch-preset-step"
import { FINE_PITCH_PRESETS } from "./fine-pitch-presets"

describe("note fine pitch preset stepping", () => {
  it("starts at an octave down and ends at an octave up", () => {
    expect(FINE_PITCH_PRESETS[0].finePitchCents).toBe(-1200)
    expect(FINE_PITCH_PRESETS[FINE_PITCH_PRESETS.length - 1].finePitchCents).toBe(
      1200
    )
  })

  it.each([
    { cents: -1200, previous: null, next: -100 },
    { cents: -100, previous: -1200, next: 0 },
    { cents: 0, previous: -100, next: 100 },
    { cents: 100, previous: 0, next: 1200 },
    { cents: 1200, previous: 100, next: null },
    { cents: -50, previous: -100, next: 0 },
    { cents: 50, previous: 0, next: 100 },
    { cents: -200, previous: -1200, next: -100 },
    { cents: 200, previous: 100, next: 1200 },
    { cents: -1300, previous: null, next: -1200 },
    { cents: 1300, previous: 1200, next: null },
    { cents: 0.0004, previous: -100, next: 100 },
    { cents: -0.0004, previous: -100, next: 100 },
    { cents: 0.001, previous: 0, next: 100 },
    { cents: -0.001, previous: -100, next: 0 },
    { cents: NaN, previous: null, next: null },
    { cents: Infinity, previous: null, next: null },
    { cents: -Infinity, previous: null, next: null },
  ])(
    "steps from $cents to previous $previous and next $next",
    ({ cents, previous, next }) => {
      expect(nextNoteFinePitchPreset(cents, "previous")).toBe(previous)
      expect(nextNoteFinePitchPreset(cents, "next")).toBe(next)
    }
  )
})
