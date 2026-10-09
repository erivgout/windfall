import { describe, expect, it } from "vitest"

import { FADE_PRESETS, fadePresetUpdates } from "./fade-presets"

describe("fade presets", () => {
  it("lists None, Short, Medium, and Long with their fractions", () => {
    expect(FADE_PRESETS).toEqual([
      { label: "None", fraction: 0 },
      { label: "Short", fraction: 1 / 16 },
      { label: "Medium", fraction: 1 / 8 },
      { label: "Long", fraction: 1 / 4 },
    ])
  })

  it("sets Short to one sixteenth of each clip's length, rounded, on both fades", () => {
    expect(
      fadePresetUpdates(
        [
          { id: 1, length: 120, fadeIn: 0, fadeOut: 0 },
          { id: 2, length: 100, fadeIn: 0, fadeOut: 0 },
        ],
        FADE_PRESETS[1].fraction
      )
    ).toEqual([
      { id: 1, patch: { fadeIn: 8, fadeOut: 8 } },
      { id: 2, patch: { fadeIn: 6, fadeOut: 6 } },
    ])
  })

  it("never sets Long beyond the clip length", () => {
    for (const length of [0, 1, 2, 3, 37, 100]) {
      expect(
        fadePresetUpdates(
          [{ id: 1, length, fadeIn: length + 1, fadeOut: length + 1 }],
          FADE_PRESETS[3].fraction
        )
      ).toEqual([
        {
          id: 1,
          patch: {
            fadeIn: Math.round(length / 4),
            fadeOut: Math.round(length / 4),
          },
        },
      ])
      expect(Math.round(length / 4)).toBeLessThanOrEqual(length)
    }
  })

  it("clamps targets to zero and the clip length", () => {
    const clips = [{ id: 1, length: 100, fadeIn: 3, fadeOut: 4 }]
    expect(fadePresetUpdates(clips, 2)).toEqual([
      { id: 1, patch: { fadeIn: 100, fadeOut: 100 } },
    ])
    expect(fadePresetUpdates(clips, -1)).toEqual([
      { id: 1, patch: { fadeIn: 0, fadeOut: 0 } },
    ])
  })

  it("targets zero with None", () => {
    expect(
      fadePresetUpdates(
        [{ id: 1, length: 100, fadeIn: 8, fadeOut: 16 }],
        FADE_PRESETS[0].fraction
      )
    ).toEqual([{ id: 1, patch: { fadeIn: 0, fadeOut: 0 } }])
  })

  it("omits a clip already at the target", () => {
    expect(
      fadePresetUpdates(
        [{ id: 1, length: 128, fadeIn: 16, fadeOut: 16 }],
        FADE_PRESETS[2].fraction
      )
    ).toEqual([])
  })

  it("includes a clip that matches fade-in but not fade-out", () => {
    expect(
      fadePresetUpdates(
        [{ id: 1, length: 128, fadeIn: 16, fadeOut: 0 }],
        FADE_PRESETS[2].fraction
      )
    ).toEqual([{ id: 1, patch: { fadeIn: 16, fadeOut: 16 } }])
  })

  it("keeps updates in the given order while omitting unchanged clips", () => {
    expect(
      fadePresetUpdates(
        [
          { id: 9, length: 128, fadeIn: 0, fadeOut: 0 },
          { id: 3, length: 64, fadeIn: 8, fadeOut: 8 },
          { id: 7, length: 256, fadeIn: 0, fadeOut: 0 },
        ],
        FADE_PRESETS[2].fraction
      )
    ).toEqual([
      { id: 9, patch: { fadeIn: 16, fadeOut: 16 } },
      { id: 7, patch: { fadeIn: 32, fadeOut: 32 } },
    ])
  })

  it("does not mutate the input", () => {
    const clip = Object.freeze({ id: 1, length: 128, fadeIn: 0, fadeOut: 4 })
    const clips = Object.freeze([clip])
    expect(fadePresetUpdates(clips, FADE_PRESETS[1].fraction)).toEqual([
      { id: 1, patch: { fadeIn: 8, fadeOut: 8 } },
    ])
    expect(clips).toEqual([{ id: 1, length: 128, fadeIn: 0, fadeOut: 4 }])
  })
})
