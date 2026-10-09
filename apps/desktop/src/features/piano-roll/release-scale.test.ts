import { describe, expect, it } from "vitest"

import { DEFAULT_NOTE_EXPRESSION } from "@/lib/note-expression"

import { releaseScaleUpdates, scaledRelease } from "./release-scale"

describe("release scaling", () => {
  it("halves 0.5 to 0.25 and doubles 0.5 to 1", () => {
    expect(scaledRelease(0.5, "half")).toBe(0.25)
    expect(scaledRelease(0.5, "double")).toBe(1)
  })

  it("caps doubled 0.6 at a long release", () => {
    expect(scaledRelease(0.6, "double")).toBe(1)
  })

  it("omits release 0 when halved", () => {
    expect(
      releaseScaleUpdates(
        [{ id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 0 } }],
        "half"
      )
    ).toEqual([])
  })

  it("omits release 1 when doubled", () => {
    expect(
      releaseScaleUpdates(
        [{ id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 1 } }],
        "double"
      )
    ).toEqual([])
  })

  it("treats a note without expression as release 0.5", () => {
    expect(releaseScaleUpdates([{ id: 1 }], "half")).toEqual([
      { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 0.25 } },
    ])
    expect(releaseScaleUpdates([{ id: 1 }], "double")).toEqual([
      { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 1 } },
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
    expect(releaseScaleUpdates([{ id: 1, expression }], "double")).toEqual([
      { id: 1, expression: { ...expression, release: 0.5 } },
    ])
  })

  it("keeps updates in the given order while omitting unchanged notes", () => {
    expect(
      releaseScaleUpdates(
        [
          { id: 9, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 0.25 } },
          { id: 3, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 1 } },
          { id: 7 },
        ],
        "double"
      )
    ).toEqual([
      { id: 9, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 0.5 } },
      { id: 7, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 1 } },
    ])
  })

  it("omits changes smaller than 0.001", () => {
    expect(
      releaseScaleUpdates(
        [{ id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 0.001 } }],
        "half"
      )
    ).toEqual([])
    expect(
      releaseScaleUpdates(
        [
          { id: 2, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 0.0005 } },
          { id: 3, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 0.9995 } },
        ],
        "double"
      )
    ).toEqual([])
  })

  it("includes changes exactly at the 0.001 threshold", () => {
    expect(
      releaseScaleUpdates(
        [{ id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 0.002 } }],
        "half"
      )
    ).toEqual([
      { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 0.001 } },
    ])
  })

  it("does not round release values", () => {
    expect(scaledRelease(0.12345, "half")).toBe(0.12345 / 2)
    expect(scaledRelease(0.12345, "double")).toBe(0.12345 * 2)
  })

  it("does not mutate the input or share its expression with the updates", () => {
    const expression = Object.freeze({
      ...DEFAULT_NOTE_EXPRESSION,
      release: 0.25,
    })
    const notes = Object.freeze([Object.freeze({ id: 1, expression })])
    const updates = releaseScaleUpdates(notes, "double")
    expect(updates).toEqual([
      { id: 1, expression: { ...expression, release: 0.5 } },
    ])
    expect(updates[0].expression).not.toBe(expression)
    expect(notes).toEqual([
      { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, release: 0.25 } },
    ])
  })
})
