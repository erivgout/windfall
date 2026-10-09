import { describe, expect, it } from "vitest"

import { nextMixScale, scaledMix } from "./mix-scale"

describe("effect mix scaling", () => {
  it("halves 0.5 to 0.25 and doubles 0.5 to 1", () => {
    expect(scaledMix(0.5, "halve")).toBe(0.25)
    expect(scaledMix(0.5, "double")).toBe(1)
    expect(nextMixScale(0.5, "halve")).toBe(0.25)
    expect(nextMixScale(0.5, "double")).toBe(1)
  })

  it("caps double of 0.6 at 1", () => {
    expect(scaledMix(0.6, "double")).toBe(1)
    expect(nextMixScale(0.6, "double")).toBe(1)
  })

  it("returns null when doubling fully wet", () => {
    expect(nextMixScale(1, "double")).toBeNull()
  })

  it("keeps halve of a dry effect dry without a command", () => {
    expect(scaledMix(0, "halve")).toBe(0)
    expect(nextMixScale(0, "halve")).toBeNull()
  })

  it("returns null for changes smaller than 0.001", () => {
    expect(nextMixScale(0.0018, "halve")).toBeNull()
    expect(nextMixScale(0.0009, "double")).toBeNull()
    expect(nextMixScale(0.9995, "double")).toBeNull()
  })

  it("keeps changes at the threshold and does not round", () => {
    expect(nextMixScale(0.002, "halve")).toBe(0.001)
    expect(nextMixScale(0.001, "double")).toBe(0.002)
    expect(scaledMix(0.123456, "halve")).toBe(0.061728)
    expect(scaledMix(0.123456, "double")).toBe(0.246912)
  })
})
