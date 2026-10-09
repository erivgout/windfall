import { describe, expect, it } from "vitest"

import { PPQ, TICKS_PER_STEP } from "@/lib/units"

import {
  nextNoteLengthPreset,
  noteLengthPresetStepUpdates,
} from "./length-preset-step"
import { LENGTH_PRESETS } from "./length-presets"

const [first, second, third, fourth, fifth] = LENGTH_PRESETS

describe("note length preset stepping", () => {
  it("walks presets from a 16th to a whole note", () => {
    expect(first.length).toBe(TICKS_PER_STEP)
    expect(fifth.length).toBe(PPQ * 4)
  })

  it.each([
    { name: "16th", length: first.length, previous: null, next: second.length },
    {
      name: "8th",
      length: second.length,
      previous: first.length,
      next: third.length,
    },
    {
      name: "quarter",
      length: third.length,
      previous: second.length,
      next: fourth.length,
    },
    {
      name: "half note",
      length: fourth.length,
      previous: third.length,
      next: fifth.length,
    },
    {
      name: "whole note",
      length: fifth.length,
      previous: fourth.length,
      next: null,
    },
    { name: "100 ticks", length: 100, previous: null, next: first.length },
    {
      name: "300 ticks",
      length: 300,
      previous: first.length,
      next: second.length,
    },
    {
      name: "500 ticks",
      length: 500,
      previous: second.length,
      next: third.length,
    },
    {
      name: "1000 ticks",
      length: 1000,
      previous: third.length,
      next: fourth.length,
    },
    {
      name: "2000 ticks",
      length: 2000,
      previous: fourth.length,
      next: fifth.length,
    },
    { name: "4000 ticks", length: 4000, previous: fifth.length, next: null },
    { name: "NaN", length: NaN, previous: null, next: null },
  ])(
    "steps $name to the neighboring preset or stops at the end",
    ({ length, previous, next }) => {
      expect(nextNoteLengthPreset(length, "previous")).toBe(previous)
      expect(nextNoteLengthPreset(length, "next")).toBe(next)
    }
  )

  it.each([Infinity, -Infinity])(
    "omits non-finite length %s in both directions",
    (length) => {
      expect(nextNoteLengthPreset(length, "previous")).toBeNull()
      expect(nextNoteLengthPreset(length, "next")).toBeNull()
    }
  )

  it("uses exact equality for lengths just below and above a preset", () => {
    const below = second.length - 0.0001
    const above = second.length + 0.0001
    expect(nextNoteLengthPreset(below, "previous")).toBe(first.length)
    expect(nextNoteLengthPreset(below, "next")).toBe(second.length)
    expect(nextNoteLengthPreset(above, "previous")).toBe(second.length)
    expect(nextNoteLengthPreset(above, "next")).toBe(third.length)
  })

  it("omits the whole-note note while moving the 16th note to an 8th", () => {
    expect(
      noteLengthPresetStepUpdates(
        [
          { id: 9, length: fifth.length },
          { id: 3, length: first.length },
        ],
        "next"
      )
    ).toEqual([{ id: 3, length: second.length }])
  })

  it("returns only id and length while leaving start and key unchanged", () => {
    const notes = Object.freeze([
      Object.freeze({
        id: 9,
        length: third.length,
        start: 120,
        key: 60,
        velocity: 0.8,
        expression: { glideTicks: second.length },
      }),
    ])
    expect(noteLengthPresetStepUpdates(notes, "previous")).toEqual([
      { id: 9, length: second.length },
    ])
    expect(notes[0].length).toBe(third.length)
    expect(notes[0].start).toBe(120)
    expect(notes[0].key).toBe(60)
  })

  it("returns no updates for an empty selection or notes at the requested end", () => {
    expect(noteLengthPresetStepUpdates([], "next")).toEqual([])
    expect(
      noteLengthPresetStepUpdates([{ id: 1, length: first.length }], "previous")
    ).toEqual([])
    expect(
      noteLengthPresetStepUpdates([{ id: 1, length: fifth.length }], "next")
    ).toEqual([])
  })

  it("omits a non-finite note while another note can still move", () => {
    expect(
      noteLengthPresetStepUpdates(
        [
          { id: 9, length: NaN },
          { id: 3, length: second.length },
        ],
        "previous"
      )
    ).toEqual([{ id: 3, length: first.length }])
  })
})
