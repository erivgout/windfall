import { afterEach, describe, expect, it, vi } from "vitest"

import * as detection from "./detection"
import { chordReadout } from "./readout"

const notes = [60, 64, 67].map((key, id) => ({
  id,
  key,
  start: 10,
  length: 10,
}))

afterEach(() => vi.restoreAllMocks())

describe("chord readout", () => {
  it("names selected notes even without a playhead", () => {
    expect(chordReadout(notes, new Set([0, 1, 2]), null)).toBe("C major")
  })

  it("uses the floored playhead and excludes ended and future notes", () => {
    const sounding = notes.map((note) => ({
      ...note,
      key: note.key === 64 ? 63 : note.key,
    }))
    expect(
      chordReadout(
        [
          ...sounding,
          { id: 3, key: 64, start: 0, length: 10 },
          { id: 4, key: 71, start: 11, length: 10 },
        ],
        new Set(),
        10.9
      )
    ).toBe("C minor")
    expect(chordReadout(sounding, new Set(), 20)).toBe("—")
  })

  it("selection wins over sounding notes", () => {
    expect(
      chordReadout(
        [...notes, { id: 3, key: 63, start: 0, length: 10 }],
        new Set([0, 1, 2]),
        5
      )
    ).toBe("C major")
  })

  it("preserves the detector's unknown and empty labels when keys exist", () => {
    expect(chordReadout(notes, new Set([0]), null)).toBe("Unknown chord (C)")
    expect(chordReadout([{ ...notes[0], key: NaN }], new Set([0]), null)).toBe(
      "No notes selected"
    )
  })

  it("does not call the detector when there are no keys", () => {
    const detect = vi.spyOn(detection, "detectChord")
    expect(chordReadout(notes, new Set(), null)).toBe("—")
    expect(chordReadout([], new Set(), 10)).toBe("—")
    expect(chordReadout(notes, new Set(), 20)).toBe("—")
    expect(detect).not.toHaveBeenCalled()
  })
})
