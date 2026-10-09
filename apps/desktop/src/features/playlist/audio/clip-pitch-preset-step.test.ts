import { describe, expect, it } from "vitest"

import {
  clipPitchPresetStepUpdates,
  nextClipPitchPreset,
} from "./clip-pitch-preset-step"
import { PITCH_PRESETS } from "./pitch-presets"

describe("clip pitch preset stepping", () => {
  it("starts at octave down -12 and ends at octave up 12", () => {
    expect(PITCH_PRESETS[0].pitch).toBe(-12)
    expect(PITCH_PRESETS[PITCH_PRESETS.length - 1].pitch).toBe(12)
  })

  it.each([
    { pitch: -12, previous: null, next: -7 },
    { pitch: -7, previous: -12, next: 0 },
    { pitch: 0, previous: -7, next: 7 },
    { pitch: 7, previous: 0, next: 12 },
    { pitch: 12, previous: 7, next: null },
    { pitch: -10, previous: -12, next: -7 },
    { pitch: -3, previous: -7, next: 0 },
    { pitch: 3, previous: 0, next: 7 },
    { pitch: 10, previous: 7, next: 12 },
    { pitch: -13, previous: null, next: -12 },
    { pitch: 13, previous: 12, next: null },
    { pitch: 0.0004, previous: -7, next: 7 },
    { pitch: -0.0004, previous: -7, next: 7 },
    { pitch: NaN, previous: null, next: null },
    { pitch: Infinity, previous: null, next: null },
    { pitch: -Infinity, previous: null, next: null },
  ])(
    "steps from $pitch to previous $previous and next $next",
    ({ pitch, previous, next }) => {
      expect(nextClipPitchPreset(pitch, "previous")).toBe(previous)
      expect(nextClipPitchPreset(pitch, "next")).toBe(next)
    }
  )

  it("omits octave up while moving a selected unison clip to fifth up", () => {
    expect(
      clipPitchPresetStepUpdates(
        [
          { id: 1, pitch: 12 },
          { id: 2, pitch: 0 },
        ],
        "next"
      )
    ).toEqual([{ id: 2, patch: { pitch: 7 } }])
  })

  it("omits octave down while moving a selected unison clip to fifth down", () => {
    expect(
      clipPitchPresetStepUpdates(
        [
          { id: 1, pitch: -12 },
          { id: 2, pitch: 0 },
        ],
        "previous"
      )
    ).toEqual([{ id: 2, patch: { pitch: -7 } }])
  })

  it("steps each selected clip from its own pitch", () => {
    expect(
      clipPitchPresetStepUpdates(
        [
          { id: 1, pitch: -10 },
          { id: 2, pitch: 3 },
          { id: 3, pitch: 7 },
        ],
        "next"
      )
    ).toEqual([
      { id: 1, patch: { pitch: -7 } },
      { id: 2, patch: { pitch: 7 } },
      { id: 3, patch: { pitch: 12 } },
    ])
  })

  it("patches only pitch", () => {
    const clips = [
      {
        id: 1,
        pitch: 0,
        pan: 0.5,
        gain: 1,
        fadeIn: 4,
        fadeOut: 8,
        reverse: true,
      },
    ]
    const updates = clipPitchPresetStepUpdates(clips, "next")
    expect(updates).toEqual([{ id: 1, patch: { pitch: 7 } }])
    expect(Object.keys(updates[0].patch)).toEqual(["pitch"])
  })

  it("omits non-finite pitches in either direction", () => {
    const clips = [
      { id: 1, pitch: NaN },
      { id: 2, pitch: Infinity },
      { id: 3, pitch: -Infinity },
    ]
    expect(clipPitchPresetStepUpdates(clips, "previous")).toEqual([])
    expect(clipPitchPresetStepUpdates(clips, "next")).toEqual([])
  })
})
