import { describe, expect, it } from "vitest"

import { PPQ } from "@/lib/units"

import { nextShiftScale, scaledShift } from "./shift-scale"

describe("shift scale", () => {
  it("halves 240 to 120 and doubles it to 480", () => {
    expect(scaledShift(240, "half")).toBe(120)
    expect(scaledShift(240, "double")).toBe(480)
    expect(nextShiftScale(240, "half")).toBe(120)
    expect(nextShiftScale(240, "double")).toBe(480)
  })

  it("halves -240 to -120 and doubles it to -480", () => {
    expect(scaledShift(-240, "half")).toBe(-120)
    expect(scaledShift(-240, "double")).toBe(-480)
    expect(nextShiftScale(-240, "half")).toBe(-120)
    expect(nextShiftScale(-240, "double")).toBe(-480)
  })

  it("halves odd shifts toward the grid in whole ticks", () => {
    expect(scaledShift(5, "half")).toBe(2)
    expect(scaledShift(-5, "half")).toBe(-2)
    expect(nextShiftScale(5, "half")).toBe(2)
    expect(nextShiftScale(-5, "half")).toBe(-2)
  })

  it("keeps an on-grid shift without a half or double command", () => {
    expect(scaledShift(0, "half")).toBe(0)
    expect(scaledShift(0, "double")).toBe(0)
    expect(nextShiftScale(0, "half")).toBeNull()
    expect(nextShiftScale(0, "double")).toBeNull()
  })

  it("returns a real change to the grid when halving one tick either way", () => {
    expect(nextShiftScale(1, "half")).toBe(0)
    // Math.trunc produces -0 for an early tick; both zeros are on the grid.
    expect(nextShiftScale(-1, "half") === 0).toBe(true)
  })

  it("returns null when doubling either end of the shift range", () => {
    expect(scaledShift(PPQ, "double")).toBe(PPQ)
    expect(scaledShift(-PPQ, "double")).toBe(-PPQ)
    expect(nextShiftScale(PPQ, "double")).toBeNull()
    expect(nextShiftScale(-PPQ, "double")).toBeNull()
  })

  it("doubles 480 and -480 to the ends of the shift range", () => {
    expect(scaledShift(480, "double")).toBe(PPQ)
    expect(scaledShift(-480, "double")).toBe(-PPQ)
    expect(nextShiftScale(480, "double")).toBe(PPQ)
    expect(nextShiftScale(-480, "double")).toBe(-PPQ)
  })
})
