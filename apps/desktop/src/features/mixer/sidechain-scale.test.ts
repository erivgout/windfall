import { describe, expect, it } from "vitest"

import { nextSidechainGainScale, scaledSidechainGain } from "./sidechain-scale"

describe("sidechain gain scaling", () => {
  it("halves unity to 0.5 and doubles unity to 2", () => {
    expect(scaledSidechainGain(1, "half")).toBe(0.5)
    expect(scaledSidechainGain(1, "double")).toBe(2)
    expect(nextSidechainGainScale(1, "half")).toBe(0.5)
    expect(nextSidechainGainScale(1, "double")).toBe(2)
  })

  it("caps double at the maximum gain", () => {
    expect(scaledSidechainGain(1.5, "double")).toBe(2)
    expect(nextSidechainGainScale(1.5, "double")).toBe(2)
  })

  it("returns null when doubling the maximum gain", () => {
    expect(nextSidechainGainScale(2, "double")).toBeNull()
  })

  it("keeps half of silence silent without a command", () => {
    expect(scaledSidechainGain(0, "half")).toBe(0)
    expect(nextSidechainGainScale(0, "half")).toBeNull()
  })

  it("returns null for changes smaller than 0.001", () => {
    expect(nextSidechainGainScale(0.0018, "half")).toBeNull()
    expect(nextSidechainGainScale(0.0009, "double")).toBeNull()
    expect(nextSidechainGainScale(1.9995, "double")).toBeNull()
  })

  it("keeps changes at the threshold and does not round", () => {
    expect(nextSidechainGainScale(0.002, "half")).toBe(0.001)
    expect(nextSidechainGainScale(0.001, "double")).toBe(0.002)
    expect(scaledSidechainGain(0.123456, "half")).toBe(0.061728)
    expect(scaledSidechainGain(0.123456, "double")).toBe(0.246912)
  })
})
