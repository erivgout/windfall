import { describe, expect, it } from "vitest"

import { nextSwingMixScale, scaledSwingMix } from "./swing-mix-scale"

describe("channel swing mix scaling", () => {
  it("halves 0.5 to 0.25 and doubles 0.5 to 1", () => {
    expect(scaledSwingMix(0.5, "halve")).toBe(0.25)
    expect(scaledSwingMix(0.5, "double")).toBe(1)
    expect(nextSwingMixScale(0.5, "halve")).toBe(0.25)
    expect(nextSwingMixScale(0.5, "double")).toBe(1)
  })

  it("doubles 0.25 to 0.5", () => {
    expect(scaledSwingMix(0.25, "double")).toBe(0.5)
    expect(nextSwingMixScale(0.25, "double")).toBe(0.5)
  })

  it("caps double of 0.6 at 1", () => {
    expect(scaledSwingMix(0.6, "double")).toBe(1)
    expect(nextSwingMixScale(0.6, "double")).toBe(1)
  })

  it("keeps halve of a straight mix straight without a command", () => {
    expect(scaledSwingMix(0, "halve")).toBe(0)
    expect(nextSwingMixScale(0, "halve")).toBeNull()
  })

  it("returns null when doubling a full mix", () => {
    expect(scaledSwingMix(1, "double")).toBe(1)
    expect(nextSwingMixScale(1, "double")).toBeNull()
  })

  it("returns null for changes smaller than 0.001", () => {
    expect(nextSwingMixScale(0.0018, "halve")).toBeNull()
    expect(nextSwingMixScale(0.0009, "double")).toBeNull()
    expect(nextSwingMixScale(0.9995, "double")).toBeNull()
  })

  it("keeps changes at the threshold and does not round", () => {
    expect(nextSwingMixScale(0.002, "halve")).toBe(0.001)
    expect(nextSwingMixScale(0.001, "double")).toBe(0.002)
    expect(scaledSwingMix(0.123456, "halve")).toBe(0.061728)
    expect(scaledSwingMix(0.123456, "double")).toBe(0.246912)
  })
})
