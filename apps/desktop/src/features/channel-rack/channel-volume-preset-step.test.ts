import { describe, expect, it } from "vitest"

import { nextChannelVolumePreset } from "./channel-volume-preset-step"
import { CHANNEL_VOLUME_PRESETS } from "./volume-presets"

describe("channel volume preset stepping", () => {
  it("starts at Quiet 0.5 and ends at Loud 1.5", () => {
    expect(CHANNEL_VOLUME_PRESETS[0].value).toBe(0.5)
    expect(CHANNEL_VOLUME_PRESETS[CHANNEL_VOLUME_PRESETS.length - 1].value).toBe(
      1.5
    )
  })

  it.each([
    { volume: 0.5, previous: null, next: 0.8 },
    { volume: 0.8, previous: 0.5, next: 1 },
    { volume: 1, previous: 0.8, next: 1.5 },
    { volume: 1.5, previous: 1, next: null },
    { volume: 0.6, previous: 0.5, next: 0.8 },
    { volume: 0.2, previous: null, next: 0.5 },
    { volume: 2, previous: 1.5, next: null },
    { volume: 0.8004, previous: 0.5, next: 1 },
    { volume: 0.7996, previous: 0.5, next: 1 },
    { volume: NaN, previous: null, next: null },
    { volume: Infinity, previous: null, next: null },
    { volume: -Infinity, previous: null, next: null },
  ])(
    "steps from $volume to previous $previous and next $next",
    ({ volume, previous, next }) => {
      expect(nextChannelVolumePreset(volume, "previous")).toBe(previous)
      expect(nextChannelVolumePreset(volume, "next")).toBe(next)
    }
  )
})
