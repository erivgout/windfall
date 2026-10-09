import { describe, expect, it } from "vitest"

import { DEFAULT_NOTE_EXPRESSION } from "@/lib/note-expression"
import { MAX_PATTERN_TICKS } from "@/lib/units"

import { glideScaleUpdates, scaledGlide } from "./glide-scale"

describe("glide scaling", () => {
  it("halves 240 ticks to 120 and doubles them to 480", () => {
    expect(scaledGlide(240, "half")).toBe(120)
    expect(scaledGlide(240, "double")).toBe(480)
  })

  it("rounds half of 5 ticks down to 2", () => {
    expect(scaledGlide(5, "half")).toBe(2)
  })

  it("keeps 1 tick at 1 and omits its update when halving", () => {
    expect(scaledGlide(1, "half")).toBe(1)
    expect(
      glideScaleUpdates(
        [{ id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, glideTicks: 1 } }],
        "half"
      )
    ).toEqual([])
  })

  it("keeps the maximum duration unchanged and omits it when doubling", () => {
    expect(scaledGlide(MAX_PATTERN_TICKS, "double")).toBe(MAX_PATTERN_TICKS)
    expect(
      glideScaleUpdates(
        [{ id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, glideTicks: MAX_PATTERN_TICKS } }],
        "double"
      )
    ).toEqual([])
  })

  it("caps a doubled duration at the maximum pattern length", () => {
    expect(scaledGlide(MAX_PATTERN_TICKS - 1, "double")).toBe(MAX_PATTERN_TICKS)
  })

  it("treats a note without expression as 240 ticks", () => {
    expect(glideScaleUpdates([{ id: 1 }], "half")).toEqual([
      { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, glideTicks: 120 } },
    ])
    expect(glideScaleUpdates([{ id: 1 }], "double")).toEqual([
      { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, glideTicks: 480 } },
    ])
  })

  it("treats an expression without glideTicks as 240 and keeps its fields", () => {
    const expression = {
      release: 0.25,
      finePitchCents: 42,
      modulationX: 0.2,
      modulationY: 0.8,
      articulation: "portamento" as const,
      colorGroup: 3,
    }
    expect(glideScaleUpdates([{ id: 1, expression }], "half")).toEqual([
      {
        id: 1,
        expression: { ...DEFAULT_NOTE_EXPRESSION, ...expression, glideTicks: 120 },
      },
    ])
    expect(glideScaleUpdates([{ id: 1, expression }], "double")).toEqual([
      {
        id: 1,
        expression: { ...DEFAULT_NOTE_EXPRESSION, ...expression, glideTicks: 480 },
      },
    ])
  })

  it("changes only glideTicks and keeps every other expression field", () => {
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
    expect(glideScaleUpdates([{ id: 1, expression }], "double")).toEqual([
      { id: 1, expression: { ...expression, glideTicks: 960 } },
    ])
  })

  it("omits an unchanged note while including neighbors in the given order", () => {
    expect(
      glideScaleUpdates(
        [
          { id: 9, expression: { ...DEFAULT_NOTE_EXPRESSION, glideTicks: 960 } },
          { id: 3, expression: { ...DEFAULT_NOTE_EXPRESSION, glideTicks: 1 } },
          { id: 7, expression: { ...DEFAULT_NOTE_EXPRESSION, glideTicks: 240 } },
        ],
        "half"
      )
    ).toEqual([
      { id: 9, expression: { ...DEFAULT_NOTE_EXPRESSION, glideTicks: 480 } },
      { id: 7, expression: { ...DEFAULT_NOTE_EXPRESSION, glideTicks: 120 } },
    ])
  })

  it("does not mutate the input or share its expression", () => {
    const expression = Object.freeze({ ...DEFAULT_NOTE_EXPRESSION, glideTicks: 480 })
    const notes = Object.freeze([Object.freeze({ id: 1, expression })])
    const updates = glideScaleUpdates(notes, "double")
    expect(updates).toEqual([
      { id: 1, expression: { ...expression, glideTicks: 960 } },
    ])
    expect(updates[0].expression).not.toBe(expression)
    expect(notes).toEqual([
      { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, glideTicks: 480 } },
    ])
  })
})
