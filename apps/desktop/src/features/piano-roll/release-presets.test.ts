import { describe, expect, it } from "vitest"

import { DEFAULT_NOTE_EXPRESSION } from "@/lib/note-expression"

import { RELEASE_PRESETS, releasePresetUpdates } from "./release-presets"

describe("release presets", () => {
  it("lists Short, Natural, and Long", () => {
    expect(RELEASE_PRESETS).toEqual([
      { label: "Short", release: 0 },
      { label: "Natural", release: 0.5 },
      { label: "Long", release: 1 },
    ])
  })

  it("omits a note already at the preset", () => {
    expect(
      releasePresetUpdates(
        [{ id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 1 } }],
        1
      )
    ).toEqual([])
  })

  it("omits differences smaller than 0.001 in either direction", () => {
    expect(
      releasePresetUpdates(
        [
          {
            id: 1,
            expression: { ...DEFAULT_NOTE_EXPRESSION, release: 0.4995 },
          },
          {
            id: 2,
            expression: { ...DEFAULT_NOTE_EXPRESSION, release: 0.5005 },
          },
        ],
        0.5
      )
    ).toEqual([])
    expect(
      releasePresetUpdates(
        [{ id: 3, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 0.001 } }],
        0
      )
    ).toEqual([
      { id: 3, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 0 } },
    ])
  })

  it("treats a note without expression as release 0.5", () => {
    expect(releasePresetUpdates([{ id: 1 }], 0.5)).toEqual([])
    expect(releasePresetUpdates([{ id: 1 }], 0)).toEqual([
      { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 0 } },
    ])
  })

  it("changes only release and keeps the other expression fields", () => {
    const expression = {
      release: 0.25,
      finePitchCents: 42,
      modulationX: 0.2,
      modulationY: 0.8,
      articulation: "portamento" as const,
      glideTicks: 480,
      colorGroup: 3,
    }
    expect(releasePresetUpdates([{ id: 1, expression }], 1)).toEqual([
      { id: 1, expression: { ...expression, release: 1 } },
    ])
  })

  it("keeps updates in the given order while omitting matching notes", () => {
    expect(
      releasePresetUpdates(
        [
          { id: 9, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 0 } },
          { id: 3 },
          { id: 7, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 1 } },
        ],
        0.5
      )
    ).toEqual([
      { id: 9, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 0.5 } },
      { id: 7, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 0.5 } },
    ])
  })

  it("does not mutate the input or share its expression with the updates", () => {
    const expression = Object.freeze({
      ...DEFAULT_NOTE_EXPRESSION,
      release: 0.25,
    })
    const notes = Object.freeze([Object.freeze({ id: 1, expression })])
    const updates = releasePresetUpdates(notes, 1)
    expect(updates).toEqual([
      { id: 1, expression: { ...expression, release: 1 } },
    ])
    expect(updates[0].expression).not.toBe(expression)
    expect(notes).toEqual([
      { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 0.25 } },
    ])
  })
})
