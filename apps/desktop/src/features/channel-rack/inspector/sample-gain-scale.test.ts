import { describe, expect, it } from "vitest"

import { nextSampleGainScale, scaledSampleGain } from "./sample-gain-scale"

describe("sample gain scale", () => {
  it("halves unity to 0.5 and doubles unity to 2", () => {
    expect(scaledSampleGain(1, "half")).toBe(0.5)
    expect(scaledSampleGain(1, "double")).toBe(2)
    expect(nextSampleGainScale(1, "half")).toBe(0.5)
    expect(nextSampleGainScale(1, "double")).toBe(2)
  })

  it("caps doubling at the maximum gain", () => {
    expect(scaledSampleGain(1.5, "double")).toBe(2)
    expect(nextSampleGainScale(1.5, "double")).toBe(2)
  })

  it("returns null when doubling the maximum gain", () => {
    expect(nextSampleGainScale(2, "double")).toBeNull()
  })

  it("returns null when halving silence", () => {
    expect(scaledSampleGain(0, "half")).toBe(0)
    expect(nextSampleGainScale(0, "half")).toBeNull()
  })

  it("returns null for changes smaller than 0.001", () => {
    expect(nextSampleGainScale(0.0018, "half")).toBeNull()
    expect(nextSampleGainScale(0.0009, "double")).toBeNull()
    expect(nextSampleGainScale(1.9991, "double")).toBeNull()
  })

  it("keeps changes equal to 0.001", () => {
    expect(nextSampleGainScale(0.002, "half")).toBe(0.001)
    expect(nextSampleGainScale(0.001, "double")).toBe(0.002)
  })

  it("does not round the scaled gain", () => {
    expect(scaledSampleGain(0.1234567, "half")).toBe(0.1234567 / 2)
    expect(scaledSampleGain(0.1234567, "double")).toBe(0.1234567 * 2)
    expect(nextSampleGainScale(0.1234567, "half")).toBe(0.1234567 / 2)
    expect(nextSampleGainScale(0.1234567, "double")).toBe(0.1234567 * 2)
  })
})
