import { describe, expect, it } from "vitest"

import { nextChannelPanScale, scaledChannelPan } from "./pan-scale"

describe("channel pan scale", () => {
  it("halves 0.5 to 0.25 and doubles it to 1", () => {
    expect(scaledChannelPan(0.5, "half")).toBe(0.25)
    expect(scaledChannelPan(0.5, "double")).toBe(1)
    expect(nextChannelPanScale(0.5, "half")).toBe(0.25)
    expect(nextChannelPanScale(0.5, "double")).toBe(1)
  })

  it("halves -0.5 to -0.25 and doubles it to -1", () => {
    expect(scaledChannelPan(-0.5, "half")).toBe(-0.25)
    expect(scaledChannelPan(-0.5, "double")).toBe(-1)
    expect(nextChannelPanScale(-0.5, "half")).toBe(-0.25)
    expect(nextChannelPanScale(-0.5, "double")).toBe(-1)
  })

  it("stops doubling at hard left or hard right", () => {
    expect(scaledChannelPan(0.6, "double")).toBe(1)
    expect(scaledChannelPan(-0.6, "double")).toBe(-1)
    expect(nextChannelPanScale(0.6, "double")).toBe(1)
    expect(nextChannelPanScale(-0.6, "double")).toBe(-1)
  })

  it("returns null when doubling hard left or hard right", () => {
    expect(nextChannelPanScale(1, "double")).toBeNull()
    expect(nextChannelPanScale(-1, "double")).toBeNull()
  })

  it("keeps center centered without a command for either factor", () => {
    expect(scaledChannelPan(0, "half")).toBe(0)
    expect(scaledChannelPan(0, "double")).toBe(0)
    expect(nextChannelPanScale(0, "half")).toBeNull()
    expect(nextChannelPanScale(0, "double")).toBeNull()
  })

  it("returns null for changes smaller than 0.001", () => {
    for (const sign of [-1, 1]) {
      expect(nextChannelPanScale(sign * 0.0018, "half")).toBeNull()
      expect(nextChannelPanScale(sign * 0.0009, "double")).toBeNull()
      expect(nextChannelPanScale(sign * 0.9991, "double")).toBeNull()
    }
  })

  it("applies changes of exactly 0.001", () => {
    for (const sign of [-1, 1]) {
      expect(nextChannelPanScale(sign * 0.002, "half")).toBe(sign * 0.001)
      expect(nextChannelPanScale(sign * 0.001, "double")).toBe(sign * 0.002)
    }
  })

  it("does not round the scaled pan", () => {
    for (const pan of [-0.123456789, 0.123456789]) {
      expect(nextChannelPanScale(pan, "half")).toBe(pan / 2)
      expect(nextChannelPanScale(pan, "double")).toBe(pan * 2)
    }
  })
})
