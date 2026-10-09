import { describe, expect, it } from "vitest"

import { MAX_SONG_TICKS } from "@/lib/units"

import { clipLengthScaleUpdates, scaledClipLength } from "./clip-length-scale"

describe("playlist clip length scaling", () => {
  it("halves and doubles a 3840-tick clip at the start of the song", () => {
    expect(scaledClipLength(0, 3840, "half")).toBe(1920)
    expect(scaledClipLength(0, 3840, "double")).toBe(7680)
  })

  it("rounds half down to a whole number of ticks", () => {
    expect(scaledClipLength(0, 5, "half")).toBe(2)
  })

  it("keeps a 1-tick clip at 1 tick and omits its update", () => {
    expect(scaledClipLength(0, 1, "half")).toBe(1)
    expect(
      clipLengthScaleUpdates([{ id: 1, start: 0, length: 1 }], "half")
    ).toEqual([])
  })

  it("omits a doubled clip that already ends at the song limit", () => {
    const start = MAX_SONG_TICKS - 3840
    expect(scaledClipLength(start, 3840, "double")).toBe(3840)
    expect(
      clipLengthScaleUpdates([{ id: 1, start, length: 3840 }], "double")
    ).toEqual([])
  })

  it("caps a doubled clip at the remaining song length", () => {
    expect(scaledClipLength(MAX_SONG_TICKS - 50, 40, "double")).toBe(50)
  })

  it.each([MAX_SONG_TICKS, MAX_SONG_TICKS + 1])(
    "keeps the current length when start %i leaves less than 1 tick",
    (start) => {
      expect(scaledClipLength(start, 40, "double")).toBe(40)
    }
  )

  it("preserves order, includes only id and length, and leaves input unchanged", () => {
    const clips = Object.freeze(
      [
        { id: 3, start: 480, length: 5, muted: true },
        { id: 2, start: 0, length: 1, muted: false },
        { id: 1, start: 1920, length: 3840, muted: false },
      ].map((clip) => Object.freeze(clip))
    )
    const before = structuredClone(clips)

    expect(clipLengthScaleUpdates(clips, "half")).toEqual([
      { id: 3, length: 2 },
      { id: 1, length: 1920 },
    ])
    expect(clips).toEqual(before)
  })
})
