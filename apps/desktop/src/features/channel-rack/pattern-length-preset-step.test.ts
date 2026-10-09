import { describe, expect, it } from "vitest"

import { DEFAULT_PATTERN_STEPS } from "@/lib/units"

import { LENGTH_PRESETS } from "./actions"
import { nextPatternLengthPreset } from "./pattern-length-preset-step"

const [first, second, third, fourth] = LENGTH_PRESETS

describe("pattern length preset stepping", () => {
  it("orders presets from 16 to 64 steps and defaults to 16 steps", () => {
    expect(LENGTH_PRESETS).toEqual([16, 32, 48, 64])
    expect(DEFAULT_PATTERN_STEPS).toBe(16)
  })

  it.each([
    { steps: first, previous: null, next: second },
    { steps: second, previous: first, next: third },
    { steps: third, previous: second, next: fourth },
    { steps: fourth, previous: third, next: null },
    { steps: 8, previous: null, next: first },
    { steps: 20, previous: first, next: second },
    { steps: 40, previous: second, next: third },
    { steps: 50, previous: third, next: fourth },
    { steps: 80, previous: fourth, next: null },
    { steps: 16.4, previous: first, next: second },
    { steps: first + 0.0004, previous: first, next: second },
    { steps: second - 0.0004, previous: first, next: second },
    { steps: NaN, previous: null, next: null },
    { steps: Infinity, previous: null, next: null },
    { steps: -Infinity, previous: null, next: null },
  ])(
    "steps from $steps to previous $previous and next $next",
    ({ steps, previous, next }) => {
      expect(nextPatternLengthPreset(steps, "previous")).toBe(previous)
      expect(nextPatternLengthPreset(steps, "next")).toBe(next)
    }
  )

  it("treats a missing length as the default 16 steps", () => {
    const pattern: { lengthSteps?: number } = {}
    const steps = pattern.lengthSteps ?? DEFAULT_PATTERN_STEPS

    expect(nextPatternLengthPreset(steps, "previous")).toBeNull()
    expect(nextPatternLengthPreset(steps, "next")).toBe(second)
  })
})
