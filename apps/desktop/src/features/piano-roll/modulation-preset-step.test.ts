import { describe, expect, it } from "vitest"

import { DEFAULT_NOTE_EXPRESSION } from "@/lib/note-expression"

import {
  modulationPresetStepUpdates,
  nextNoteModulationPreset,
} from "./modulation-preset-step"
import { MODULATION_PRESETS } from "./modulation-presets"

const AXES = ["modulationX", "modulationY"] as const

describe("modulation preset stepping", () => {
  it("walks presets whose first value is 0 and last value is 1", () => {
    expect(MODULATION_PRESETS[0].modulation).toBe(0)
    expect(MODULATION_PRESETS[MODULATION_PRESETS.length - 1].modulation).toBe(1)
  })

  it.each([
    { value: 0, previous: null, next: 0.5 },
    { value: 0.5, previous: 0, next: 1 },
    { value: 1, previous: 0.5, next: null },
    { value: 0.25, previous: 0, next: 0.5 },
    { value: 0.75, previous: 0.5, next: 1 },
    { value: -0.1, previous: null, next: 0 },
    { value: 1.1, previous: 1, next: null },
    { value: 0.0004, previous: null, next: 0.5 },
    { value: 0.499, previous: 0, next: 0.5 },
    { value: 0.001, previous: 0, next: 0.5 },
    { value: 0.4995, previous: 0, next: 1 },
    { value: 0.5004, previous: 0, next: 1 },
    { value: 0.9996, previous: 0.5, next: null },
    { value: NaN, previous: null, next: null },
    { value: Infinity, previous: null, next: null },
    { value: -Infinity, previous: null, next: null },
  ])("steps $value to its neighboring presets", ({ value, previous, next }) => {
    expect(nextNoteModulationPreset(value, "previous")).toBe(previous)
    expect(nextNoteModulationPreset(value, "next")).toBe(next)
  })

  it.each(AXES)("treats missing expression as Center for %s", (axis) => {
    expect(modulationPresetStepUpdates([{ id: 1 }], axis, "previous")).toEqual([
      { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0 } },
    ])
    expect(modulationPresetStepUpdates([{ id: 1 }], axis, "next")).toEqual([
      { id: 1, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 1 } },
    ])
  })

  it.each(AXES)("changes only %s and preserves the expression", (axis) => {
    const expression = Object.freeze({
      release: 0.25,
      finePitchCents: 42,
      modulationX: 0.25,
      modulationY: 0.75,
      articulation: "portamento" as const,
      glideTicks: 480,
      colorGroup: 3,
    })
    const notes = Object.freeze([Object.freeze({ id: 1, expression })])
    const updates = modulationPresetStepUpdates(notes, axis, "next")
    expect(updates).toEqual([
      {
        id: 1,
        expression: {
          ...expression,
          [axis]: axis === "modulationX" ? 0.5 : 1,
        },
      },
    ])
    expect(updates[0].expression).not.toBe(expression)
    expect(expression.modulationX).toBe(0.25)
    expect(expression.modulationY).toBe(0.75)
  })

  it.each(AXES)("omits a High note while moving a Low note on %s", (axis) => {
    expect(
      modulationPresetStepUpdates(
        [
          { id: 9, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 1 } },
          { id: 3, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0 } },
        ],
        axis,
        "next"
      )
    ).toEqual([
      { id: 3, expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: 0.5 } },
    ])
  })

  it.each(AXES)("omits non-finite %s values in both directions", (axis) => {
    const notes = [NaN, Infinity, -Infinity].map((value, id) => ({
      id,
      expression: { ...DEFAULT_NOTE_EXPRESSION, [axis]: value },
    }))
    expect(modulationPresetStepUpdates(notes, axis, "previous")).toEqual([])
    expect(modulationPresetStepUpdates(notes, axis, "next")).toEqual([])
  })
})
