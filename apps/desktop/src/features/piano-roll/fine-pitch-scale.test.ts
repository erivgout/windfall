import { describe, expect, it } from "vitest"

import { DEFAULT_NOTE_EXPRESSION } from "@/lib/note-expression"

import { finePitchScaleUpdates, scaledFinePitch } from "./fine-pitch-scale"

describe("fine pitch scaling", () => {
  it("halves 100 to 50 and doubles 100 to 200", () => {
    expect(scaledFinePitch(100, "half")).toBe(50)
    expect(scaledFinePitch(100, "double")).toBe(200)
  })

  it("halves -100 to -50 and doubles -100 to -200", () => {
    expect(scaledFinePitch(-100, "half")).toBe(-50)
    expect(scaledFinePitch(-100, "double")).toBe(-200)
  })

  it("does not round fine pitch values", () => {
    expect(scaledFinePitch(100.12345, "half")).toBe(100.12345 / 2)
    expect(scaledFinePitch(-100.12345, "half")).toBe(-100.12345 / 2)
  })

  it("omits in tune notes for half and double", () => {
    const notes = [
      { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: 0 } },
    ]
    expect(finePitchScaleUpdates(notes, "half")).toEqual([])
    expect(finePitchScaleUpdates(notes, "double")).toEqual([])
  })

  it("treats a note without expression as in tune", () => {
    expect(finePitchScaleUpdates([{ id: 1 }], "half")).toEqual([])
    expect(finePitchScaleUpdates([{ id: 1 }], "double")).toEqual([])
  })

  it("omits both octave limits when doubled", () => {
    expect(
      finePitchScaleUpdates(
        [
          {
            id: 1,
            expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: 1200 },
          },
          {
            id: 2,
            expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: -1200 },
          },
        ],
        "double"
      )
    ).toEqual([])
  })

  it("caps doubled 700 and -700 at their octave limits", () => {
    expect(
      finePitchScaleUpdates(
        [
          {
            id: 1,
            expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: 700 },
          },
          {
            id: 2,
            expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: -700 },
          },
        ],
        "double"
      )
    ).toEqual([
      {
        id: 1,
        expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: 1200 },
      },
      {
        id: 2,
        expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: -1200 },
      },
    ])
  })

  it("doubles 600 to 1200", () => {
    expect(scaledFinePitch(600, "double")).toBe(1200)
  })

  it("changes only fine pitch and keeps the other expression fields", () => {
    const expression = {
      release: 0.25,
      finePitchCents: 100,
      modulationX: 0.2,
      modulationY: 0.8,
      articulation: "portamento" as const,
      glideTicks: 480,
      colorGroup: 3,
    }
    expect(finePitchScaleUpdates([{ id: 1, expression }], "half")).toEqual([
      { id: 1, expression: { ...expression, finePitchCents: 50 } },
    ])
  })

  it("omits a tiny half change while including a neighboring note", () => {
    expect(
      finePitchScaleUpdates(
        [
          {
            id: 9,
            expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: 0.001 },
          },
          {
            id: 3,
            expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: 100 },
          },
        ],
        "half"
      )
    ).toEqual([
      { id: 3, expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: 50 } },
    ])
  })

  it("omits a tiny double change while including a neighboring note", () => {
    expect(
      finePitchScaleUpdates(
        [
          {
            id: 9,
            expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: -0.0005 },
          },
          {
            id: 3,
            expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: -100 },
          },
        ],
        "double"
      )
    ).toEqual([
      {
        id: 3,
        expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: -200 },
      },
    ])
  })

  it("includes changes exactly at the 0.001 threshold", () => {
    expect(
      finePitchScaleUpdates(
        [
          {
            id: 1,
            expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: 0.002 },
          },
        ],
        "half"
      )
    ).toEqual([
      {
        id: 1,
        expression: { ...DEFAULT_NOTE_EXPRESSION, finePitchCents: 0.001 },
      },
    ])
  })
})
