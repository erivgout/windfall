import { describe, expect, it } from "vitest"

import { nextChannelVolumeScale, scaledChannelVolume } from "./volume-scale"

describe("channel volume scale", () => {
  it("halves unity to 0.5 and doubles it to 2", () => {
    expect(scaledChannelVolume(1, "half")).toBe(0.5)
    expect(scaledChannelVolume(1, "double")).toBe(2)
    expect(nextChannelVolumeScale(1, "half")).toBe(0.5)
    expect(nextChannelVolumeScale(1, "double")).toBe(2)
  })

  it("stops doubling at the maximum volume", () => {
    expect(scaledChannelVolume(1.5, "double")).toBe(2)
    expect(nextChannelVolumeScale(1.5, "double")).toBe(2)
  })

  it("returns null when doubling the maximum volume", () => {
    expect(nextChannelVolumeScale(2, "double")).toBeNull()
  })

  it("keeps half of silence silent without a command", () => {
    expect(scaledChannelVolume(0, "half")).toBe(0)
    expect(nextChannelVolumeScale(0, "half")).toBeNull()
  })

  it("returns null for changes smaller than 0.001", () => {
    expect(nextChannelVolumeScale(0.0018, "half")).toBeNull()
    expect(nextChannelVolumeScale(0.0009, "double")).toBeNull()
    expect(nextChannelVolumeScale(1.9991, "double")).toBeNull()
  })

  it("applies changes of exactly 0.001", () => {
    expect(nextChannelVolumeScale(0.002, "half")).toBe(0.001)
    expect(nextChannelVolumeScale(0.001, "double")).toBe(0.002)
  })

  it("does not round the scaled volume", () => {
    const volume = 0.123456789
    expect(nextChannelVolumeScale(volume, "half")).toBe(volume / 2)
    expect(nextChannelVolumeScale(volume, "double")).toBe(volume * 2)
  })
})
