import { describe, expect, it } from "vitest"

import { PITCH_PRESETS, pitchPresetUpdates } from "./pitch-presets"

describe("pitch presets", () => {
  it("lists the five presets in semitones within -48 to 48", () => {
    expect(PITCH_PRESETS).toEqual([
      { label: "Octave down", pitch: -12 },
      { label: "Fifth down", pitch: -7 },
      { label: "Unison", pitch: 0 },
      { label: "Fifth up", pitch: 7 },
      { label: "Octave up", pitch: 12 },
    ])
    for (const { pitch } of PITCH_PRESETS) {
      expect(pitch).toBeGreaterThanOrEqual(-48)
      expect(pitch).toBeLessThanOrEqual(48)
    }
  })

  it("omits a clip already at the preset", () => {
    expect(pitchPresetUpdates([{ id: 1, pitch: -7 }], -7)).toEqual([])
  })

  it("counts a difference smaller than 0.001 as a match in either direction", () => {
    expect(
      pitchPresetUpdates(
        [
          { id: 1, pitch: -0.0005 },
          { id: 2, pitch: 0.0005 },
        ],
        0
      )
    ).toEqual([])
  })

  it("includes a differing clip with a patch of only pitch", () => {
    expect(pitchPresetUpdates([{ id: 1, pitch: -12 }], 7)).toEqual([
      { id: 1, patch: { pitch: 7 } },
    ])
  })

  it("keeps updates in the given order while omitting matching clips", () => {
    expect(
      pitchPresetUpdates(
        [
          { id: 9, pitch: -12 },
          { id: 3, pitch: 0 },
          { id: 7, pitch: 12 },
        ],
        0
      )
    ).toEqual([
      { id: 9, patch: { pitch: 0 } },
      { id: 7, patch: { pitch: 0 } },
    ])
  })

  it("does not mutate the input", () => {
    const clip = Object.freeze({ id: 1, pitch: -12 })
    const clips = Object.freeze([clip])
    expect(pitchPresetUpdates(clips, 12)).toEqual([
      { id: 1, patch: { pitch: 12 } },
    ])
    expect(clips).toEqual([{ id: 1, pitch: -12 }])
  })

  it("includes a clip at the 0.001 tolerance boundary", () => {
    expect(pitchPresetUpdates([{ id: 1, pitch: 0.001 }], 0)).toEqual([
      { id: 1, patch: { pitch: 0 } },
    ])
  })
})
