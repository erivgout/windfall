import { describe, expect, it } from "vitest"

import {
  CLIP_LENGTH_BARS,
  clipLengthTicks,
  clipLengthUpdates,
} from "./clip-length-presets"

describe("playlist clip length presets", () => {
  it.each([
    { numerator: 4, denominator: 4, ticks: 3840 },
    { numerator: 3, denominator: 4, ticks: 2880 },
    { numerator: 6, denominator: 8, ticks: 2880 },
  ])("uses $ticks ticks per bar in $numerator/$denominator", (signature) => {
    expect(clipLengthTicks(signature, 1)).toBe(signature.ticks)
  })

  it("offers one, two, four, and eight bars in 4/4", () => {
    expect(CLIP_LENGTH_BARS).toEqual([1, 2, 4, 8])
    expect(
      CLIP_LENGTH_BARS.map((bars) =>
        clipLengthTicks({ numerator: 4, denominator: 4 }, bars)
      )
    ).toEqual([3840, 7680, 15360, 30720])
  })

  it("omits a clip already at the requested length", () => {
    expect(clipLengthUpdates([{ id: 1, length: 3840 }], 3840)).toEqual([])
  })

  it("includes a clip that differs with the requested length", () => {
    expect(clipLengthUpdates([{ id: 1, length: 960 }], 3840)).toEqual([
      { id: 1, length: 3840 },
    ])
  })

  it("preserves order, includes only id and length, and leaves input unchanged", () => {
    const clips = Object.freeze(
      [
        { id: 3, length: 960, start: 480, muted: true },
        { id: 2, length: 3840, start: 0, muted: false },
        { id: 1, length: 7680, start: 1920, muted: false },
      ].map((clip) =>
        Object.freeze({ ...clip, content: { type: "pattern", pattern: 1 } })
      )
    )
    const before = structuredClone(clips)

    expect(clipLengthUpdates(clips, 3840)).toEqual([
      { id: 3, length: 3840 },
      { id: 1, length: 3840 },
    ])
    expect(clips).toEqual(before)
  })
})
