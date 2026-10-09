import { describe, expect, it } from "vitest"

import { nextMetronomeGainScale, scaledMetronomeGain } from "./metronome-gain-scale"

describe("metronome gain scale", () => {
  it("halves the default gain to 0.125 and doubles it to 0.5", () => {
    expect(scaledMetronomeGain(0.25, "half")).toBe(0.125)
    expect(scaledMetronomeGain(0.25, "double")).toBe(0.5)
    expect(nextMetronomeGainScale(0.25, "half")).toBe(0.125)
    expect(nextMetronomeGainScale(0.25, "double")).toBe(0.5)
  })

  it("halves medium gain to 0.25 and doubles it to 1", () => {
    expect(scaledMetronomeGain(0.5, "half")).toBe(0.25)
    expect(scaledMetronomeGain(0.5, "double")).toBe(1)
    expect(nextMetronomeGainScale(0.5, "half")).toBe(0.25)
    expect(nextMetronomeGainScale(0.5, "double")).toBe(1)
  })

  it("stops doubling at full volume", () => {
    expect(scaledMetronomeGain(0.6, "double")).toBe(1)
    expect(nextMetronomeGainScale(0.6, "double")).toBe(1)
    expect(scaledMetronomeGain(1, "double")).toBe(1)
    expect(nextMetronomeGainScale(1, "double")).toBeNull()
  })

  it("keeps half of silence silent without a command", () => {
    expect(scaledMetronomeGain(0, "half")).toBe(0)
    expect(nextMetronomeGainScale(0, "half")).toBeNull()
  })

  it("returns null for changes smaller than 0.001", () => {
    expect(nextMetronomeGainScale(0.0018, "half")).toBeNull()
    expect(nextMetronomeGainScale(0.0009, "double")).toBeNull()
    expect(nextMetronomeGainScale(0.9991, "double")).toBeNull()
  })

  it("applies changes of exactly 0.001", () => {
    expect(nextMetronomeGainScale(0.002, "half")).toBe(0.001)
    expect(nextMetronomeGainScale(0.001, "double")).toBe(0.002)
  })

  it("does not round the scaled gain", () => {
    const gain = 0.123456789
    expect(scaledMetronomeGain(gain, "half")).toBe(gain / 2)
    expect(scaledMetronomeGain(gain, "double")).toBe(gain * 2)
    expect(nextMetronomeGainScale(gain, "half")).toBe(gain / 2)
    expect(nextMetronomeGainScale(gain, "double")).toBe(gain * 2)
  })
})
