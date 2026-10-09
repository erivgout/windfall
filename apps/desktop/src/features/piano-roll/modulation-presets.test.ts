import { describe, expect, it } from "vitest"

import { DEFAULT_NOTE_EXPRESSION } from "@/lib/note-expression"

import {
  MODULATION_PRESETS,
  modulationPresetUpdates,
} from "./modulation-presets"

const AXES = ["modulationX", "modulationY"] as const

describe("modulation presets", () => {
  it("lists Low, Center, and High from 0 to 1", () => {
    expect(MODULATION_PRESETS).toEqual([
      { label: "Low", modulation: 0 },
      { label: "Center", modulation: 0.5 },
      { label: "High", modulation: 1 },
    ])
  })

  it.each(AXES)("omits a note already at the %s preset", (axis) => {
    expect(
      modulationPresetUpdates(
        [{ id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 1 } }],
        axis,
        1
      )
    ).toEqual([])
  })

  it.each(AXES)("omits %s differences smaller than 0.001", (axis) => {
    expect(
      modulationPresetUpdates(
        [
          { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0.4995 } },
          { id: 2, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0.5005 } },
        ],
        axis,
        0.5
      )
    ).toEqual([])
    expect(
      modulationPresetUpdates(
        [{ id: 3, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0.001 } }],
        axis,
        0
      )
    ).toEqual([
      { id: 3, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0 } },
    ])
  })

  it.each(AXES)("treats missing expression as 0.5 for %s", (axis) => {
    expect(modulationPresetUpdates([{ id: 1 }], axis, 0.5)).toEqual([])
    expect(modulationPresetUpdates([{ id: 1 }], axis, 0)).toEqual([
      { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0 } },
    ])
    expect(modulationPresetUpdates([{ id: 1 }], axis, 1)).toEqual([
      { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 1 } },
    ])
  })

  it.each(AXES)(
    "changes only %s and keeps the other expression fields",
    (axis) => {
      const expression = {
        release: 0.25,
        finePitchCents: 42,
        modulationX: 0.2,
        modulationY: 0.8,
        articulation: "portamento" as const,
        glideTicks: 480,
        colorGroup: 3,
      }
      expect(
        modulationPresetUpdates([{ id: 1, expression }], axis, 0.5)
      ).toEqual([{ id: 1, expression: { ...expression, [axis]: 0.5 } }])
    }
  )

  it.each(AXES)(
    "keeps %s updates in the given order while omitting matches",
    (axis) => {
      expect(
        modulationPresetUpdates(
          [
            { id: 9, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0 } },
            { id: 3 },
            { id: 7, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 1 } },
          ],
          axis,
          0.5
        )
      ).toEqual([
        { id: 9, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0.5 } },
        { id: 7, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0.5 } },
      ])
    }
  )

  it.each(AXES)(
    "does not mutate the input or share its %s expression",
    (axis) => {
      const expression = Object.freeze({
        ...DEFAULT_NOTE_EXPRESSION,
        [axis]: 0.2,
      })
      const notes = Object.freeze([Object.freeze({ id: 1, expression })])
      const updates = modulationPresetUpdates(notes, axis, 1)
      expect(updates).toEqual([
        { id: 1, expression: { ...expression, [axis]: 1 } },
      ])
      expect(updates[0].expression).not.toBe(expression)
      expect(notes).toEqual([
        { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0.2 } },
      ])
    }
  )
})
