import { describe, expect, it } from "vitest"

import { LENGTH_PRESETS, lengthPresetUpdates } from "./length-presets"

describe("length presets", () => {
  it("lists 16th, 8th, Quarter, Half, and Whole in ticks", () => {
    expect(LENGTH_PRESETS).toEqual([
      { label: "16th", length: 240 },
      { label: "8th", length: 480 },
      { label: "Quarter", length: 960 },
      { label: "Half", length: 1920 },
      { label: "Whole", length: 3840 },
    ])
  })

  it("omits a note already at the preset", () => {
    expect(lengthPresetUpdates([{ id: 1, length: 480 }], 480)).toEqual([])
  })

  it("includes a note that differs with the preset length", () => {
    expect(lengthPresetUpdates([{ id: 1, length: 240 }], 960)).toEqual([
      { id: 1, length: 960 },
    ])
  })

  it("keeps the given order and includes only id and length", () => {
    const notes = [
      {
        id: 9,
        length: 960,
        start: 120,
        key: 60,
        velocity: 0.8,
        pan: 0.2,
        expression: { glideTicks: 480 },
      },
      { id: 3, length: 240 },
      { id: 7, length: 480 },
    ]
    expect(lengthPresetUpdates(notes, 240)).toEqual([
      { id: 9, length: 240 },
      { id: 7, length: 240 },
    ])
  })

  it("does not mutate the input", () => {
    const notes = Object.freeze([Object.freeze({ id: 1, length: 480 })])
    const updates = lengthPresetUpdates(notes, 960)
    expect(updates).toEqual([{ id: 1, length: 960 }])
    expect(updates[0]).not.toBe(notes[0])
    expect(notes).toEqual([{ id: 1, length: 480 }])
  })
})
