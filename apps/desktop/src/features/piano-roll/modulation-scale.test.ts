import { describe, expect, it } from "vitest"

import { DEFAULT_NOTE_EXPRESSION } from "@/lib/note-expression"

import { modulationScaleUpdates } from "./modulation-scale"

const axes = ["modulationX", "modulationY"] as const
const factors = ["half", "double"] as const

describe("modulation scaling", () => {
  it.each(axes)("halves %s 0.8 to 0.65, not 0.4", (axis) => {
    const updates = modulationScaleUpdates(
      [{ id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0.8 } }],
      axis,
      "half"
    )
    expect(updates).toEqual([
      { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0.65 } },
    ])
  })

  it.each(axes)("scales %s quarter offsets from center", (axis) => {
    const notes = [
      { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0.25 } },
      { id: 2, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0.75 } },
    ]
    expect(modulationScaleUpdates(notes, axis, "half")).toEqual([
      { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0.375 } },
      { id: 2, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0.625 } },
    ])
    expect(modulationScaleUpdates(notes, axis, "double")).toEqual([
      { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0 } },
      { id: 2, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 1 } },
    ])
  })

  it.each(factors)("omits centered notes for %s", (factor) => {
    for (const axis of axes) {
      expect(
        modulationScaleUpdates(
          [{ id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0.5 } }],
          axis,
          factor
        )
      ).toEqual([])
    }
  })

  it.each(factors)("omits notes without expression for %s", (factor) => {
    for (const axis of axes) {
      expect(modulationScaleUpdates([{ id: 1 }], axis, factor)).toEqual([])
    }
  })

  it.each(axes)("omits both %s limits when doubled", (axis) => {
    expect(
      modulationScaleUpdates(
        [
          { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0 } },
          { id: 2, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 1 } },
        ],
        axis,
        "double"
      )
    ).toEqual([])
  })

  it.each(axes)("clamps doubled %s offsets to low and high", (axis) => {
    expect(
      modulationScaleUpdates(
        [
          { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0.1 } },
          { id: 2, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0.9 } },
        ],
        axis,
        "double"
      )
    ).toEqual([
      { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0 } },
      { id: 2, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 1 } },
    ])
  })

  it("changes only modulation X and keeps every other expression field", () => {
    const expression = {
      release: 0.25,
      finePitchCents: 100,
      modulationX: 0.8,
      modulationY: 0.2,
      articulation: "portamento" as const,
      glideTicks: 480,
      colorGroup: 3,
    }
    expect(
      modulationScaleUpdates([{ id: 1, expression }], "modulationX", "half")
    ).toEqual([{ id: 1, expression: { ...expression, modulationX: 0.65 } }])
    expect(expression.modulationX).toBe(0.8)
  })

  it("changes only modulation Y and keeps modulation X", () => {
    const expression = {
      ...DEFAULT_NOTE_EXPRESSION,
      modulationX: 0.2,
      modulationY: 0.75,
    }
    expect(
      modulationScaleUpdates([{ id: 1, expression }], "modulationY", "double")
    ).toEqual([{ id: 1, expression: { ...expression, modulationY: 1 } }])
    expect(expression.modulationY).toBe(0.75)
  })

  describe.each(axes)("%s change threshold", (axis) => {
    it.each(factors)(
      "omits a tiny %s change while including a neighboring note",
      (factor) => {
        expect(
          modulationScaleUpdates(
            [
              {
                id: 9,
                expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0.5005 },
              },
              {
                id: 3,
                expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0.75 },
              },
            ],
            axis,
            factor
          )
        ).toEqual([
          {
            id: 3,
            expression: {
              ...DEFAULT_NOTE_EXPRESSION,
              [axis]: factor === "half" ? 0.625 : 1,
            },
          },
        ])
      }
    )
  })
})
