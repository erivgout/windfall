import { describe, expect, it } from "vitest"

import { notePanScaleUpdates, scaledNotePan } from "./note-pan-scale"

describe("note pan scaling", () => {
  it("halves 0.5 to 0.25 and doubles 0.5 to 1", () => {
    expect(scaledNotePan(0.5, "half")).toBe(0.25)
    expect(scaledNotePan(0.5, "double")).toBe(1)
  })

  it("halves -0.5 to -0.25 and doubles -0.5 to -1", () => {
    expect(scaledNotePan(-0.5, "half")).toBe(-0.25)
    expect(scaledNotePan(-0.5, "double")).toBe(-1)
  })

  it("caps doubled pan at hard left and hard right", () => {
    expect(scaledNotePan(0.6, "double")).toBe(1)
    expect(scaledNotePan(-0.6, "double")).toBe(-1)
  })

  it("omits hard left and hard right when doubled", () => {
    expect(
      notePanScaleUpdates(
        [
          { id: 1, pan: 1 },
          { id: 2, pan: -1 },
        ],
        "double"
      )
    ).toEqual([])
  })

  it("omits centered notes for both factors", () => {
    expect(notePanScaleUpdates([{ id: 1, pan: 0 }], "half")).toEqual([])
    expect(notePanScaleUpdates([{ id: 1, pan: 0 }], "double")).toEqual([])
  })

  it("omits changes smaller than 0.001", () => {
    expect(
      notePanScaleUpdates(
        [
          { id: 1, pan: 0.001 },
          { id: 2, pan: -0.001 },
        ],
        "half"
      )
    ).toEqual([])
    expect(
      notePanScaleUpdates(
        [
          { id: 3, pan: 0.0005 },
          { id: 4, pan: -0.0005 },
          { id: 5, pan: 0.9995 },
          { id: 6, pan: -0.9995 },
        ],
        "double"
      )
    ).toEqual([])
  })

  it("keeps updates in the given order with only id and pan", () => {
    const notes = [
      { id: 9, pan: -0.25, velocity: 0.5 },
      { id: 3, pan: 0, velocity: 1 },
      { id: 7, pan: 0.4, velocity: 0.75 },
    ]

    expect(notePanScaleUpdates(notes, "double")).toEqual([
      { id: 9, pan: -0.5 },
      { id: 7, pan: 0.8 },
    ])
    expect(notePanScaleUpdates(notes, "half")).toEqual([
      { id: 9, pan: -0.125 },
      { id: 7, pan: 0.2 },
    ])
  })

  it("does not round pan values", () => {
    for (const pan of [0.12345, -0.12345]) {
      expect(scaledNotePan(pan, "half")).toBe(pan / 2)
      expect(scaledNotePan(pan, "double")).toBe(pan * 2)
    }
  })

  it("includes changes exactly at the 0.001 threshold", () => {
    expect(
      notePanScaleUpdates(
        [
          { id: 1, pan: 0.002 },
          { id: 2, pan: -0.002 },
        ],
        "half"
      )
    ).toEqual([
      { id: 1, pan: 0.001 },
      { id: 2, pan: -0.001 },
    ])
    expect(
      notePanScaleUpdates(
        [
          { id: 1, pan: 0.001 },
          { id: 2, pan: -0.001 },
        ],
        "double"
      )
    ).toEqual([
      { id: 1, pan: 0.002 },
      { id: 2, pan: -0.002 },
    ])
  })

  it("does not mutate the input", () => {
    const notes = Object.freeze([
      Object.freeze({ id: 1, pan: 0.6 }),
      Object.freeze({ id: 2, pan: -0.6 }),
    ])

    expect(notePanScaleUpdates(notes, "half")).toEqual([
      { id: 1, pan: 0.3 },
      { id: 2, pan: -0.3 },
    ])
    expect(notePanScaleUpdates(notes, "double")).toEqual([
      { id: 1, pan: 1 },
      { id: 2, pan: -1 },
    ])
    expect(notes).toEqual([
      { id: 1, pan: 0.6 },
      { id: 2, pan: -0.6 },
    ])
  })
})
