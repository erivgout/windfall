import { describe, expect, it } from "vitest"

import { nextTrackHeight, TRACK_HEIGHT_PRESETS } from "./track-height-presets"

describe("track height presets", () => {
  it("lists Follow, Short, Normal, and Tall with their saved heights", () => {
    expect(TRACK_HEIGHT_PRESETS).toEqual([
      { label: "Follow", height: 0 },
      { label: "Short", height: 18 },
      { label: "Normal", height: 38 },
      { label: "Tall", height: 92 },
    ])
  })

  it("returns null when the track is already that height", () => {
    for (const { height } of TRACK_HEIGHT_PRESETS) {
      expect(nextTrackHeight(height, height)).toBeNull()
    }
  })

  it("returns the preset when the current height differs", () => {
    for (const { height } of TRACK_HEIGHT_PRESETS) {
      expect(nextTrackHeight(64, height)).toBe(height)
    }
  })

  it("matches Follow at zero without matching Short", () => {
    expect(nextTrackHeight(0, TRACK_HEIGHT_PRESETS[0].height)).toBeNull()
    expect(nextTrackHeight(0, TRACK_HEIGHT_PRESETS[1].height)).toBe(18)
  })
})
