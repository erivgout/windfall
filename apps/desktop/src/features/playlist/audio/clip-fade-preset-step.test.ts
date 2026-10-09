import { describe, expect, it } from "vitest"

import {
  clipFadePresetStepUpdates,
  nextClipFadePreset,
} from "./clip-fade-preset-step"
import { FADE_PRESETS } from "./fade-presets"

describe("clip fade preset stepping", () => {
  it("starts at no fade and ends at a quarter of the clip length", () => {
    expect(FADE_PRESETS[0].fraction).toBe(0)
    expect(FADE_PRESETS[FADE_PRESETS.length - 1].fraction).toBe(1 / 4)
  })

  it.each([
    { label: "None", fadeIn: 0, fadeOut: 0, previous: null, next: 1 },
    { label: "Short", fadeIn: 1, fadeOut: 1, previous: 0, next: 2 },
    { label: "Medium", fadeIn: 2, fadeOut: 2, previous: 1, next: 4 },
    { label: "Long", fadeIn: 4, fadeOut: 4, previous: 2, next: null },
    { label: "custom", fadeIn: 3, fadeOut: 3, previous: null, next: null },
    { label: "unequal", fadeIn: 1, fadeOut: 2, previous: null, next: null },
  ])(
    "steps $label fades for length 16 without wrapping or moving custom fades",
    ({ fadeIn, fadeOut, previous, next }) => {
      expect(nextClipFadePreset(16, fadeIn, fadeOut, "previous")).toEqual(
        previous === null ? null : { fadeIn: previous, fadeOut: previous }
      )
      expect(nextClipFadePreset(16, fadeIn, fadeOut, "next")).toEqual(
        next === null ? null : { fadeIn: next, fadeOut: next }
      )
    }
  )

  it("uses the first match and skips the equal Short and Medium targets at length 8", () => {
    const short = Math.round(8 * FADE_PRESETS[1].fraction)
    expect(short).toBe(Math.round(8 * FADE_PRESETS[2].fraction))
    expect(nextClipFadePreset(8, short, short, "previous")).toEqual({
      fadeIn: 0,
      fadeOut: 0,
    })
    expect(nextClipFadePreset(8, short, short, "next")).toEqual({
      fadeIn: 2,
      fadeOut: 2,
    })
  })

  it("stays at zero for a zero-length clip in both directions", () => {
    expect(nextClipFadePreset(0, 0, 0, "previous")).toBeNull()
    expect(nextClipFadePreset(0, 0, 0, "next")).toBeNull()
  })

  it("compares ticks exactly without a tolerance", () => {
    expect(nextClipFadePreset(16, 1.0004, 1.0004, "previous")).toBeNull()
    expect(nextClipFadePreset(16, 1.0004, 1.0004, "next")).toBeNull()
    expect(nextClipFadePreset(16, 1, 1.0004, "previous")).toBeNull()
    expect(nextClipFadePreset(16, 1, 1.0004, "next")).toBeNull()
  })

  it.each([
    { length: -1, fadeIn: 0, fadeOut: 0 },
    { length: NaN, fadeIn: 0, fadeOut: 0 },
    { length: Infinity, fadeIn: 0, fadeOut: 0 },
    { length: -Infinity, fadeIn: 0, fadeOut: 0 },
    { length: 16, fadeIn: NaN, fadeOut: 0 },
    { length: 16, fadeIn: Infinity, fadeOut: Infinity },
    { length: 16, fadeIn: -Infinity, fadeOut: -Infinity },
    { length: 16, fadeIn: 0, fadeOut: NaN },
    { length: 16, fadeIn: 0, fadeOut: Infinity },
    { length: 16, fadeIn: 0, fadeOut: -Infinity },
  ])("rejects invalid length or fades: $length, $fadeIn, $fadeOut", (clip) => {
    const { length, fadeIn, fadeOut } = clip
    expect(nextClipFadePreset(length, fadeIn, fadeOut, "previous")).toBeNull()
    expect(nextClipFadePreset(length, fadeIn, fadeOut, "next")).toBeNull()
    expect(clipFadePresetStepUpdates([{ id: 1, ...clip }], "previous")).toEqual(
      []
    )
    expect(clipFadePresetStepUpdates([{ id: 1, ...clip }], "next")).toEqual([])
  })

  it("omits a Long clip while moving a selected None clip next", () => {
    expect(
      clipFadePresetStepUpdates(
        [
          { id: 1, length: 8, fadeIn: 2, fadeOut: 2 },
          { id: 2, length: 16, fadeIn: 0, fadeOut: 0 },
        ],
        "next"
      )
    ).toEqual([{ id: 2, patch: { fadeIn: 1, fadeOut: 1 } }])
  })

  it("omits a None clip while moving a selected Long clip previous", () => {
    expect(
      clipFadePresetStepUpdates(
        [
          { id: 1, length: 16, fadeIn: 0, fadeOut: 0 },
          { id: 2, length: 8, fadeIn: 2, fadeOut: 2 },
        ],
        "previous"
      )
    ).toEqual([{ id: 2, patch: { fadeIn: 1, fadeOut: 1 } }])
  })

  it("steps each selected clip from its own fade and length", () => {
    expect(
      clipFadePresetStepUpdates(
        [
          { id: 1, length: 16, fadeIn: 1, fadeOut: 1 },
          { id: 2, length: 8, fadeIn: 1, fadeOut: 1 },
          { id: 3, length: 32, fadeIn: 4, fadeOut: 4 },
          { id: 4, length: 16, fadeIn: 3, fadeOut: 3 },
          { id: 5, length: 16, fadeIn: 1, fadeOut: 2 },
        ],
        "next"
      )
    ).toEqual([
      { id: 1, patch: { fadeIn: 2, fadeOut: 2 } },
      { id: 2, patch: { fadeIn: 2, fadeOut: 2 } },
      { id: 3, patch: { fadeIn: 8, fadeOut: 8 } },
    ])
  })

  it("patches only both fades, preserving pitch and pan", () => {
    const clips = [
      { id: 1, length: 16, fadeIn: 0, fadeOut: 0, pitch: 7, pan: 0.5, gain: 1 },
    ]
    const original = clips.map((clip) => ({ ...clip }))
    const updates = clipFadePresetStepUpdates(clips, "next")
    expect(updates).toEqual([{ id: 1, patch: { fadeIn: 1, fadeOut: 1 } }])
    expect(Object.keys(updates[0].patch)).toEqual(["fadeIn", "fadeOut"])
    expect(clips).toEqual(original)
  })
})
