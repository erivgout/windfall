import { describe, expect, it } from "vitest"

import { DEFAULT_NOTE_EXPRESSION } from "@/lib/note-expression"

import { GLIDE_PRESETS, glidePresetUpdates } from "./glide-presets"

describe("glide presets", () => {
  it("lists 16th, 8th, Quarter, and Half in ticks", () => {
    expect(GLIDE_PRESETS).toEqual([
      { label: "16th", glideTicks: 240 },
      { label: "8th", glideTicks: 480 },
      { label: "Quarter", glideTicks: 960 },
      { label: "Half", glideTicks: 1920 },
    ])
  })

  it("omits a note already at the preset", () => {
    expect(
      glidePresetUpdates(
        [
          {
            id: 1,
            expression: { ...DEFAULT_NOTE_EXPRESSION, glideTicks: 480 },
          },
        ],
        480
      )
    ).toEqual([])
  })

  it("treats a note without expression as 240 ticks", () => {
    expect(glidePresetUpdates([{ id: 1 }], 240)).toEqual([])
    expect(glidePresetUpdates([{ id: 1 }], 960)).toEqual([
      { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, glideTicks: 960 } },
    ])
  })

  it("treats an expression without glideTicks as 240 ticks", () => {
    const expression = {
      release: 0.25,
      finePitchCents: 42,
      modulationX: 0.2,
      modulationY: 0.8,
    }
    expect(glidePresetUpdates([{ id: 1, expression }], 240)).toEqual([])
    expect(glidePresetUpdates([{ id: 1, expression }], 480)).toEqual([
      {
        id: 1,
        expression: {
          ...DEFAULT_NOTE_EXPRESSION,
          ...expression,
          glideTicks: 480,
        },
      },
    ])
  })

  it("changes only glideTicks and keeps the other expression fields", () => {
    const expression = {
      release: 0.25,
      finePitchCents: 42,
      modulationX: 0.2,
      modulationY: 0.8,
      articulation: "portamento" as const,
      glideTicks: 480,
      colorGroup: 3,
      customField: "preserved",
    }
    expect(glidePresetUpdates([{ id: 1, expression }], 1920)).toEqual([
      { id: 1, expression: { ...expression, glideTicks: 1920 } },
    ])
  })

  it("keeps updates in the given order while omitting matches", () => {
    expect(
      glidePresetUpdates(
        [
          {
            id: 9,
            expression: { ...DEFAULT_NOTE_EXPRESSION, glideTicks: 960 },
          },
          { id: 3 },
          {
            id: 7,
            expression: { ...DEFAULT_NOTE_EXPRESSION, glideTicks: 480 },
          },
        ],
        240
      )
    ).toEqual([
      { id: 9, expression: { ...DEFAULT_NOTE_EXPRESSION, glideTicks: 240 } },
      { id: 7, expression: { ...DEFAULT_NOTE_EXPRESSION, glideTicks: 240 } },
    ])
  })

  it("does not mutate the input or share its expression", () => {
    const expression = Object.freeze({
      ...DEFAULT_NOTE_EXPRESSION,
      glideTicks: 480,
    })
    const notes = Object.freeze([Object.freeze({ id: 1, expression })])
    const updates = glidePresetUpdates(notes, 960)
    expect(updates).toEqual([
      { id: 1, expression: { ...expression, glideTicks: 960 } },
    ])
    expect(updates[0].expression).not.toBe(expression)
    expect(notes).toEqual([
      { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, glideTicks: 480 } },
    ])
  })
})
