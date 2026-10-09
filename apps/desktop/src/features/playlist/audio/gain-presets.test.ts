import { describe, expect, it } from "vitest"

import { MAX_GAIN } from "@/lib/units"

import { GAIN_PRESETS, gainPresetUpdates } from "./gain-presets"

describe("gain presets", () => {
  it("lists Quiet, Unity, and Loud with gains inside the allowed range", () => {
    expect(GAIN_PRESETS).toEqual([
      { label: "Quiet", gain: 0.5 },
      { label: "Unity", gain: 1 },
      { label: "Loud", gain: 1.5 },
    ])
    for (const { gain } of GAIN_PRESETS) {
      expect(gain).toBeGreaterThan(0)
      expect(gain).toBeLessThan(MAX_GAIN)
    }
  })

  it("omits a clip already at the preset", () => {
    expect(gainPresetUpdates([{ id: 1, gain: 1 }], 1)).toEqual([])
  })

  it("counts a difference smaller than 0.001 as a match in either direction", () => {
    expect(
      gainPresetUpdates(
        [
          { id: 1, gain: 0.9995 },
          { id: 2, gain: 1.0005 },
        ],
        1
      )
    ).toEqual([])
  })

  it("includes a differing clip with a patch of only gain", () => {
    expect(gainPresetUpdates([{ id: 1, gain: 0.5 }], 1.5)).toEqual([
      { id: 1, patch: { gain: 1.5 } },
    ])
  })

  it("keeps updates in the given order while omitting matching clips", () => {
    expect(
      gainPresetUpdates(
        [
          { id: 9, gain: 0.5 },
          { id: 3, gain: 1 },
          { id: 7, gain: 1.5 },
        ],
        1
      )
    ).toEqual([
      { id: 9, patch: { gain: 1 } },
      { id: 7, patch: { gain: 1 } },
    ])
  })

  it("does not mutate the input", () => {
    const clip = Object.freeze({ id: 1, gain: 0.5 })
    const clips = Object.freeze([clip])
    expect(gainPresetUpdates(clips, 1)).toEqual([{ id: 1, patch: { gain: 1 } }])
    expect(clips).toEqual([{ id: 1, gain: 0.5 }])
  })
})
