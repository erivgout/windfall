import { describe, expect, it } from "vitest"

import { nextFaderVolumeScale, scaledFaderVolume } from "./fader-scale"

describe("fader volume scaling", () => {
  it("halves unity to 0.5 and doubles unity to 2", () => {
    expect(scaledFaderVolume(1, "half")).toBe(0.5)
    expect(scaledFaderVolume(1, "double")).toBe(2)
    expect(nextFaderVolumeScale(1, "half")).toBe(0.5)
    expect(nextFaderVolumeScale(1, "double")).toBe(2)
  })

  it("caps double at the maximum volume", () => {
    expect(scaledFaderVolume(1.5, "double")).toBe(2)
    expect(nextFaderVolumeScale(1.5, "double")).toBe(2)
  })

  it("returns null when doubling the maximum volume", () => {
    expect(nextFaderVolumeScale(2, "double")).toBeNull()
  })

  it("keeps half of silence silent without a command", () => {
    expect(scaledFaderVolume(0, "half")).toBe(0)
    expect(nextFaderVolumeScale(0, "half")).toBeNull()
  })

  it("returns null for changes smaller than 0.001", () => {
    expect(nextFaderVolumeScale(0.0018, "half")).toBeNull()
    expect(nextFaderVolumeScale(0.0009, "double")).toBeNull()
    expect(nextFaderVolumeScale(1.9995, "double")).toBeNull()
  })

  it("applies a change of exactly 0.001", () => {
    expect(nextFaderVolumeScale(0.002, "half")).toBe(0.001)
    expect(nextFaderVolumeScale(0.001, "double")).toBe(0.002)
  })

  it("does not round scaled volumes", () => {
    expect(scaledFaderVolume(0.12345, "half")).toBe(0.12345 / 2)
    expect(scaledFaderVolume(0.12345, "double")).toBe(0.12345 * 2)
    expect(nextFaderVolumeScale(0.12345, "half")).toBe(0.12345 / 2)
    expect(nextFaderVolumeScale(0.12345, "double")).toBe(0.12345 * 2)
  })
})
