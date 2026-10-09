import { describe, expect, it } from "vitest"

import { NOTE_PAN_PRESETS, notePanPresetUpdates } from "./note-pan-presets"

describe("note pan presets", () => {
  it("lists Hard left, Left, Center, Right, and Hard right", () => {
    expect(NOTE_PAN_PRESETS).toEqual([
      { label: "Hard left", pan: -1 },
      { label: "Left", pan: -0.5 },
      { label: "Center", pan: 0 },
      { label: "Right", pan: 0.5 },
      { label: "Hard right", pan: 1 },
    ])
  })

  it("omits a note already at the preset", () => {
    expect(notePanPresetUpdates([{ id: 1, pan: 0.5 }], 0.5)).toEqual([])
  })

  it("counts a difference smaller than 0.001 as a match in either direction", () => {
    expect(
      notePanPresetUpdates(
        [
          { id: 1, pan: -0.0005 },
          { id: 2, pan: 0.0005 },
        ],
        0
      )
    ).toEqual([])
  })

  it("includes a differing note", () => {
    expect(notePanPresetUpdates([{ id: 1, pan: -0.5 }], 0.5)).toEqual([
      { id: 1, pan: 0.5 },
    ])
  })

  it("keeps updates in the given order while omitting matching notes", () => {
    expect(
      notePanPresetUpdates(
        [
          { id: 9, pan: -1 },
          { id: 3, pan: 0 },
          { id: 7, pan: 1 },
        ],
        0
      )
    ).toEqual([
      { id: 9, pan: 0 },
      { id: 7, pan: 0 },
    ])
  })

  it("does not mutate the input", () => {
    const note = Object.freeze({ id: 1, pan: -0.5 })
    const notes = Object.freeze([note])
    expect(notePanPresetUpdates(notes, 1)).toEqual([{ id: 1, pan: 1 }])
    expect(notes).toEqual([{ id: 1, pan: -0.5 }])
  })
})
