import { describe, expect, it } from "vitest"

import { nextMixerPanScale, scaledMixerPan } from "./mixer-pan-scale"

describe("mixer pan scale", () => {
  it("halves 0.5 to 0.25 and doubles it to 1", () => {
    expect(scaledMixerPan(0.5, "half")).toBe(0.25)
    expect(scaledMixerPan(0.5, "double")).toBe(1)
    expect(nextMixerPanScale(0.5, "half")).toBe(0.25)
    expect(nextMixerPanScale(0.5, "double")).toBe(1)
  })

  it("halves -0.5 to -0.25 and doubles it to -1", () => {
    expect(scaledMixerPan(-0.5, "half")).toBe(-0.25)
    expect(scaledMixerPan(-0.5, "double")).toBe(-1)
    expect(nextMixerPanScale(-0.5, "half")).toBe(-0.25)
    expect(nextMixerPanScale(-0.5, "double")).toBe(-1)
  })

  it("stops doubling at hard left or hard right", () => {
    expect(scaledMixerPan(0.6, "double")).toBe(1)
    expect(scaledMixerPan(-0.6, "double")).toBe(-1)
    expect(nextMixerPanScale(0.6, "double")).toBe(1)
    expect(nextMixerPanScale(-0.6, "double")).toBe(-1)
  })

  it("returns null when doubling hard left or hard right", () => {
    expect(nextMixerPanScale(1, "double")).toBeNull()
    expect(nextMixerPanScale(-1, "double")).toBeNull()
  })

  it("keeps center centered without a command for either factor", () => {
    expect(scaledMixerPan(0, "half")).toBe(0)
    expect(scaledMixerPan(0, "double")).toBe(0)
    expect(nextMixerPanScale(0, "half")).toBeNull()
    expect(nextMixerPanScale(0, "double")).toBeNull()
  })

  it("returns null for changes smaller than 0.001", () => {
    for (const sign of [-1, 1]) {
      expect(nextMixerPanScale(sign * 0.0018, "half")).toBeNull()
      expect(nextMixerPanScale(sign * 0.0009, "double")).toBeNull()
      expect(nextMixerPanScale(sign * 0.9991, "double")).toBeNull()
    }
  })

  it("applies changes of exactly 0.001", () => {
    for (const sign of [-1, 1]) {
      expect(nextMixerPanScale(sign * 0.002, "half")).toBe(sign * 0.001)
      expect(nextMixerPanScale(sign * 0.001, "double")).toBe(sign * 0.002)
    }
  })

  it("does not round the scaled pan", () => {
    for (const pan of [-0.123456789, 0.123456789]) {
      expect(scaledMixerPan(pan, "half")).toBe(pan / 2)
      expect(scaledMixerPan(pan, "double")).toBe(pan * 2)
      expect(nextMixerPanScale(pan, "half")).toBe(pan / 2)
      expect(nextMixerPanScale(pan, "double")).toBe(pan * 2)
    }
  })
})
