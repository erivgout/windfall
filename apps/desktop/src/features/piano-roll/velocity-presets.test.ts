import { describe, expect, it } from "vitest"

import { VELOCITY_PRESETS, velocityPresetUpdates } from "./velocity-presets"

describe("velocity presets", () => {
  it("lists Soft, Medium, Strong, and Full", () => {
    expect(VELOCITY_PRESETS).toEqual([
      { label: "Soft", velocity: 0.25 },
      { label: "Medium", velocity: 0.5 },
      { label: "Strong", velocity: 0.8 },
      { label: "Full", velocity: 1 },
    ])
  })

  it("omits a note already at the preset", () => {
    expect(velocityPresetUpdates([{ id: 1, velocity: 0.5 }], 0.5)).toEqual([])
  })

  it("counts a difference smaller than 0.001 as a match in either direction", () => {
    expect(
      velocityPresetUpdates(
        [
          { id: 1, velocity: 0.4995 },
          { id: 2, velocity: 0.5005 },
        ],
        0.5
      )
    ).toEqual([])
  })

  it("includes a differing note", () => {
    expect(velocityPresetUpdates([{ id: 1, velocity: 0.25 }], 0.8)).toEqual([
      { id: 1, velocity: 0.8 },
    ])
  })

  it("keeps updates in the given order while omitting matching notes", () => {
    expect(
      velocityPresetUpdates(
        [
          { id: 9, velocity: 0.25 },
          { id: 3, velocity: 0.5 },
          { id: 7, velocity: 1 },
        ],
        0.5
      )
    ).toEqual([
      { id: 9, velocity: 0.5 },
      { id: 7, velocity: 0.5 },
    ])
  })

  it("does not mutate the input", () => {
    const note = Object.freeze({ id: 1, velocity: 0.25 })
    const notes = Object.freeze([note])
    expect(velocityPresetUpdates(notes, 1)).toEqual([{ id: 1, velocity: 1 }])
    expect(notes).toEqual([{ id: 1, velocity: 0.25 }])
  })
})
