import { describe, expect, it } from "vitest"

import { MAX_PATTERN_TICKS } from "./edit-math"
import { CHORD_STAMPS, placeStamp, SCALE_STAMPS } from "./stamps"

describe("stamp placement arithmetic", () => {
  it("places authored chords simultaneously without changing their intervals or dynamics", () => {
    for (const stamp of CHORD_STAMPS) {
      const placed = placeStamp(stamp, 480, 60, 360, 0.35)
      expect(placed.error).toBeNull()
      expect(placed.notes.map((note) => note.key - 60)).toEqual(stamp.intervals)
      for (const note of placed.notes)
        expect(note).toMatchObject({
          start: 480,
          length: 360,
          velocity: 0.35,
          pan: 0,
        })
    }
  })

  it("uses consecutive current lengths for scales in both directions", () => {
    for (const stamp of SCALE_STAMPS) {
      const placed = placeStamp(stamp, 100, 60, 120, 0.8)
      expect(placed.error).toBeNull()
      expect(placed.notes.map((note) => note.start)).toEqual(
        stamp.intervals.map((_, index) => 100 + index * 120)
      )
      expect(placed.notes.map((note) => note.key - 60)).toEqual(
        stamp.layout === "descending"
          ? [...stamp.intervals].reverse()
          : stamp.intervals
      )
    }
  })

  it("accepts exact bounds and rejects the whole pattern when one note exceeds them", () => {
    const chord = CHORD_STAMPS[0]
    expect(
      placeStamp(chord, MAX_PATTERN_TICKS - 240, 120, 240, 0).error
    ).toBeNull()
    for (const [tick, key, length, velocity] of [
      [0, 121, 240, 0.8],
      [-1, 60, 240, 0.8],
      [0, -1, 240, 0.8],
      [MAX_PATTERN_TICKS - 239, 60, 240, 0.8],
      [0, 60, 0, 0.8],
      [0, 60, 1.5, 0.8],
      [0, 60, 240, NaN],
      [0, 60, 240, 2],
    ]) {
      const placed = placeStamp(chord, tick, key, length, velocity)
      expect(placed.notes).toEqual([])
      expect(placed.error).toBeTypeOf("string")
    }
    const scale = SCALE_STAMPS[0]
    expect(
      placeStamp(scale, MAX_PATTERN_TICKS - 240, 60, 240, 0.8).notes
    ).toEqual([])
    expect(
      placeStamp(
        scale,
        MAX_PATTERN_TICKS - scale.intervals.length * 240,
        60,
        240,
        0.8
      ).error
    ).toBeNull()
  })
})
