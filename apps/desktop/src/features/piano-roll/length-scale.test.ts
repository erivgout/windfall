import { describe, expect, it } from "vitest"

import { MAX_PATTERN_TICKS } from "@/lib/units"

import { lengthScaleUpdates, scaledLength } from "./length-scale"

describe("note length scaling", () => {
  it("halves 240 ticks to 120 and doubles them to 480", () => {
    expect(scaledLength(240, "half")).toBe(120)
    expect(scaledLength(240, "double")).toBe(480)
  })

  it("rounds half of 5 ticks down to 2", () => {
    expect(scaledLength(5, "half")).toBe(2)
  })

  it("keeps a 1-tick note at 1 and omits its update when halving", () => {
    expect(scaledLength(1, "half")).toBe(1)
    expect(lengthScaleUpdates([{ id: 1, length: 1 }], "half")).toEqual([])
  })

  it("keeps a maximum-length note unchanged and omits it when doubling", () => {
    expect(scaledLength(MAX_PATTERN_TICKS, "double")).toBe(MAX_PATTERN_TICKS)
    expect(
      lengthScaleUpdates([{ id: 1, length: MAX_PATTERN_TICKS }], "double")
    ).toEqual([])
  })

  it("caps a doubled length at the maximum pattern length", () => {
    expect(scaledLength(MAX_PATTERN_TICKS - 1, "double")).toBe(MAX_PATTERN_TICKS)
  })

  it("includes a changed note with only its id and new length", () => {
    const notes = [
      {
        id: 9,
        length: 240,
        start: 120,
        key: 60,
        velocity: 0.8,
        expression: { glideTicks: 480 },
      },
    ]
    expect(lengthScaleUpdates(notes, "double")).toEqual([
      { id: 9, length: 480 },
    ])
  })

  it("keeps the given order while omitting unchanged notes", () => {
    const notes = [
      { id: 9, length: 960 },
      { id: 3, length: 1 },
      { id: 7, length: 240 },
    ]
    expect(lengthScaleUpdates(notes, "half")).toEqual([
      { id: 9, length: 480 },
      { id: 7, length: 120 },
    ])
  })

  it("does not mutate the input", () => {
    const notes = Object.freeze([Object.freeze({ id: 1, length: 240 })])
    const updates = lengthScaleUpdates(notes, "double")
    expect(updates).toEqual([{ id: 1, length: 480 }])
    expect(updates[0]).not.toBe(notes[0])
    expect(notes).toEqual([{ id: 1, length: 240 }])
  })
})
