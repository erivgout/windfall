import { describe, expect, it } from "vitest"

import { nextLaneHeightScale, scaledLaneHeight } from "./lane-height-scale"

describe("lane height scaling", () => {
  it.each([
    [84, "half", 44, 44],
    [88, "half", 44, 44],
    [100, "half", 50, 50],
    [130, "double", 260, 260],
    [200, "double", 260, 260],
    [44, "half", 44, null],
    [260, "double", 260, null],
  ] as const)(
    "scales %i pixels by %s to %i and returns %s as the next height",
    (height, factor, scaled, next) => {
      expect(scaledLaneHeight(height, factor)).toBe(scaled)
      expect(nextLaneHeightScale(height, factor)).toBe(next)
    }
  )
})
