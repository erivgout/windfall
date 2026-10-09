import { describe, expect, it } from "vitest"

import { DEFAULT_NOTE_EXPRESSION } from "@/lib/note-expression"

import {
  FINE_PITCH_PRESETS,
  finePitchPresetUpdates,
} from "./fine-pitch-presets"

describe("fine pitch presets", () => {
  it("lists the five presets in cents", () => {
    expect(FINE_PITCH_PRESETS).toEqual([
      { label: "Octave down", finePitchCents: -1200 },
      { label: "Semitone down", finePitchCents: -100 },
      { label: "In tune", finePitchCents: 0 },
      { label: "Semitone up", finePitchCents: 100 },
      { label: "Octave up", finePitchCents: 1200 },
    ])
  })

  it("omits a note already at the preset", () => {
    expect(
      finePitchPresetUpdates(
        [
          {
            id: 1,
            expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: 100 },
          },
        ],
        100
      )
    ).toEqual([])
  })

  it("omits differences smaller than 0.001 in either direction", () => {
    expect(
      finePitchPresetUpdates(
        [
          {
            id: 1,
            expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: -0.0005 },
          },
          {
            id: 2,
            expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: 0.0005 },
          },
        ],
        0
      )
    ).toEqual([])
    expect(
      finePitchPresetUpdates(
        [
          {
            id: 3,
            expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: 0.001 },
          },
        ],
        0
      )
    ).toEqual([
      { id: 3, expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: 0 } },
    ])
  })

  it("treats a note without expression as 0 cents", () => {
    expect(finePitchPresetUpdates([{ id: 1 }], 0)).toEqual([])
    expect(finePitchPresetUpdates([{ id: 1 }], -1200)).toEqual([
      {
        id: 1,
        expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: -1200 },
      },
    ])
  })

  it("changes only fine pitch and keeps the other expression fields", () => {
    const expression = {
      release: 0.25,
      finePitchCents: 42,
      modulationX: 0.2,
      modulationY: 0.8,
      articulation: "portamento" as const,
      glideTicks: 480,
      colorGroup: 3,
    }
    expect(finePitchPresetUpdates([{ id: 1, expression }], 0)).toEqual([
      { id: 1, expression: { ...expression, finePitchCents: 0 } },
    ])
  })

  it("keeps updates in the given order while omitting matching notes", () => {
    expect(
      finePitchPresetUpdates(
        [
          {
            id: 9,
            expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: -100 },
          },
          { id: 3 },
          {
            id: 7,
            expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: 1200 },
          },
        ],
        0
      )
    ).toEqual([
      { id: 9, expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: 0 } },
      { id: 7, expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: 0 } },
    ])
  })

  it("does not mutate the input or share its expression with the updates", () => {
    const expression = Object.freeze({
      ...DEFAULT_NOTE_EXPRESSION,
      finePitchCents: 42,
    })
    const notes = Object.freeze([Object.freeze({ id: 1, expression })])
    const updates = finePitchPresetUpdates(notes, 100)
    expect(updates).toEqual([
      { id: 1, expression: { ...expression, finePitchCents: 100 } },
    ])
    expect(updates[0].expression).not.toBe(expression)
    expect(notes).toEqual([
      { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: 42 } },
    ])
  })
})
