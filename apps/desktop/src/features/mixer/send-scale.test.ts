import { describe, expect, it } from "vitest"

import { nextSendGainScale, scaledSendGain } from "./send-scale"

describe("send gain scaling", () => {
  it("halves unity to 0.5 and doubles unity to 2", () => {
    expect(scaledSendGain(1, "half")).toBe(0.5)
    expect(scaledSendGain(1, "double")).toBe(2)
    expect(nextSendGainScale(1, "half")).toBe(0.5)
    expect(nextSendGainScale(1, "double")).toBe(2)
  })

  it("caps double at the maximum gain", () => {
    expect(scaledSendGain(1.5, "double")).toBe(2)
    expect(nextSendGainScale(1.5, "double")).toBe(2)
  })

  it("returns null when doubling the maximum gain", () => {
    expect(nextSendGainScale(2, "double")).toBeNull()
  })

  it("keeps half of silence silent without a command", () => {
    expect(scaledSendGain(0, "half")).toBe(0)
    expect(nextSendGainScale(0, "half")).toBeNull()
  })

  it("returns null for changes smaller than 0.001", () => {
    expect(nextSendGainScale(0.0018, "half")).toBeNull()
    expect(nextSendGainScale(0.0009, "double")).toBeNull()
    expect(nextSendGainScale(1.9995, "double")).toBeNull()
  })

  it("keeps changes at the threshold and does not round", () => {
    expect(nextSendGainScale(0.002, "half")).toBe(0.001)
    expect(nextSendGainScale(0.001, "double")).toBe(0.002)
    expect(scaledSendGain(0.123456, "half")).toBe(0.061728)
    expect(scaledSendGain(0.123456, "double")).toBe(0.246912)
  })
})
