import { describe, expect, it } from "vitest"

import { DEFAULT_NOTE_EXPRESSION } from "@/lib/note-expression"
import { PPQ, TICKS_PER_STEP } from "@/lib/units"

import { glidePresetStepUpdates, nextGlidePreset } from "./glide-preset-step"
import { GLIDE_PRESETS } from "./glide-presets"

const [first, second, third, fourth] = GLIDE_PRESETS

describe("glide preset stepping", () => {
  it("walks presets from a 16th to a half note", () => {
    expect(first.glideTicks).toBe(TICKS_PER_STEP)
    expect(fourth.glideTicks).toBe(PPQ * 2)
  })

  it.each([
    {
      name: "16th",
      ticks: first.glideTicks,
      previous: null,
      next: second.glideTicks,
    },
    {
      name: "8th",
      ticks: second.glideTicks,
      previous: first.glideTicks,
      next: third.glideTicks,
    },
    {
      name: "quarter",
      ticks: third.glideTicks,
      previous: second.glideTicks,
      next: fourth.glideTicks,
    },
    {
      name: "half note",
      ticks: fourth.glideTicks,
      previous: third.glideTicks,
      next: null,
    },
    { name: "100 ticks", ticks: 100, previous: null, next: first.glideTicks },
    {
      name: "300 ticks",
      ticks: 300,
      previous: first.glideTicks,
      next: second.glideTicks,
    },
    {
      name: "500 ticks",
      ticks: 500,
      previous: second.glideTicks,
      next: third.glideTicks,
    },
    {
      name: "1000 ticks",
      ticks: 1000,
      previous: third.glideTicks,
      next: fourth.glideTicks,
    },
    {
      name: "2000 ticks",
      ticks: 2000,
      previous: fourth.glideTicks,
      next: null,
    },
    { name: "NaN", ticks: NaN, previous: null, next: null },
  ])(
    "steps $name to the neighboring preset or stops at the end",
    ({ ticks, previous, next }) => {
      expect(nextGlidePreset(ticks, "previous")).toBe(previous)
      expect(nextGlidePreset(ticks, "next")).toBe(next)
    }
  )

  it.each([Infinity, -Infinity])(
    "omits non-finite duration %s in both directions",
    (ticks) => {
      expect(nextGlidePreset(ticks, "previous")).toBeNull()
      expect(nextGlidePreset(ticks, "next")).toBeNull()
    }
  )

  it("uses exact equality for durations just below and above a preset", () => {
    const below = second.glideTicks - 0.0001
    const above = second.glideTicks + 0.0001
    expect(nextGlidePreset(below, "previous")).toBe(first.glideTicks)
    expect(nextGlidePreset(below, "next")).toBe(second.glideTicks)
    expect(nextGlidePreset(above, "previous")).toBe(second.glideTicks)
    expect(nextGlidePreset(above, "next")).toBe(third.glideTicks)
  })

  it("treats a note with no expression as the first preset", () => {
    expect(DEFAULT_NOTE_EXPRESSION.glideTicks).toBe(first.glideTicks)
    expect(glidePresetStepUpdates([{ id: 1 }], "previous")).toEqual([])
    expect(glidePresetStepUpdates([{ id: 1 }], "next")).toEqual([
      {
        id: 1,
        expression: {
          ...DEFAULT_NOTE_EXPRESSION,
          glideTicks: second.glideTicks,
        },
      },
    ])
  })

  it("defaults a missing duration and preserves the other expression fields", () => {
    const expression = {
      release: 0.25,
      finePitchCents: 42,
      modulationX: 0.2,
      modulationY: 0.8,
      articulation: "portamento" as const,
      colorGroup: 3,
    }
    expect(glidePresetStepUpdates([{ id: 1, expression }], "previous")).toEqual(
      []
    )
    expect(glidePresetStepUpdates([{ id: 1, expression }], "next")).toEqual([
      { id: 1, expression: { ...expression, glideTicks: second.glideTicks } },
    ])
  })

  it("omits the last-preset note while moving the first-preset note and copying its expression", () => {
    const expression = Object.freeze({
      release: 0.25,
      finePitchCents: 42,
      modulationX: 0.2,
      modulationY: 0.8,
      articulation: "portamento" as const,
      glideTicks: first.glideTicks,
      colorGroup: 3,
      customField: "preserved",
    })
    const lastExpression = Object.freeze({
      ...expression,
      glideTicks: fourth.glideTicks,
    })
    const notes = Object.freeze([
      Object.freeze({ id: 9, expression: lastExpression }),
      Object.freeze({ id: 3, expression }),
    ])
    const updates = glidePresetStepUpdates(notes, "next")
    expect(updates).toEqual([
      { id: 3, expression: { ...expression, glideTicks: second.glideTicks } },
    ])
    expect(updates[0].expression).not.toBe(expression)
    expect(notes[0].expression.glideTicks).toBe(fourth.glideTicks)
    expect(notes[1].expression.glideTicks).toBe(first.glideTicks)
  })

  it("omits a non-finite note while another note can still move", () => {
    expect(
      glidePresetStepUpdates(
        [
          {
            id: 9,
            expression: { ...DEFAULT_NOTE_EXPRESSION, glideTicks: NaN },
          },
          {
            id: 3,
            expression: {
              ...DEFAULT_NOTE_EXPRESSION,
              glideTicks: second.glideTicks,
            },
          },
        ],
        "previous"
      )
    ).toEqual([
      {
        id: 3,
        expression: {
          ...DEFAULT_NOTE_EXPRESSION,
          glideTicks: first.glideTicks,
        },
      },
    ])
  })
})
