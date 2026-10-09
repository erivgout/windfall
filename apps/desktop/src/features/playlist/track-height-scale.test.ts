import { describe, expect, it } from "vitest"

import { nextTrackHeightScale, scaledTrackHeight } from "./track-height-scale"

describe("track height scaling", () => {
  it.each([
    [0, "half", 0, null],
    [0, "double", 0, null],
    [38, "half", 19, 19],
    [38, "double", 76, 76],
    [92, "double", 160, 160],
    [80, "double", 160, 160],
    [160, "double", 160, null],
    [18, "half", 18, null],
    [19, "half", 18, 18],
    [37, "half", 18, 18],
  ] as const)(
    "scales %i pixels by %s to %i and returns %s as the next height",
    (height, factor, scaled, next) => {
      expect(scaledTrackHeight(height, factor)).toBe(scaled)
      expect(nextTrackHeightScale(height, factor)).toBe(next)
    }
  )
})
