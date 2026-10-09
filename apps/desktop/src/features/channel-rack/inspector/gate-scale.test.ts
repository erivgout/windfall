import { describe, expect, it } from "vitest"

import { MAX_PATTERN_STEPS, TICKS_PER_STEP } from "@/lib/units"

import { nextGateScale, scaledGate } from "./gate-scale"

describe("gate scale", () => {
  it("halves 480 to 240 and doubles it to 960", () => {
    expect(scaledGate(480, "half")).toBe(240)
    expect(scaledGate(480, "double")).toBe(960)
    expect(nextGateScale(480, "half")).toBe(240)
    expect(nextGateScale(480, "double")).toBe(960)
  })

  it("halves 5 to 2 whole ticks", () => {
    expect(scaledGate(5, "half")).toBe(2)
    expect(nextGateScale(5, "half")).toBe(2)
  })

  it("keeps a written-length gate of 0 without a half command", () => {
    expect(scaledGate(0, "half")).toBe(0)
    expect(nextGateScale(0, "half")).toBeNull()
  })

  it("returns null when doubling the maximum gate", () => {
    const maximum = MAX_PATTERN_STEPS * TICKS_PER_STEP
    expect(scaledGate(maximum, "double")).toBe(maximum)
    expect(nextGateScale(maximum, "double")).toBeNull()
  })

  it("doubles one tick below the maximum to the maximum", () => {
    const maximum = MAX_PATTERN_STEPS * TICKS_PER_STEP
    expect(scaledGate(maximum - 1, "double")).toBe(maximum)
    expect(nextGateScale(maximum - 1, "double")).toBe(maximum)
  })
})
