import { describe, expect, it } from "vitest"

import { MAX_PATTERN_TICKS } from "@/lib/units"

import { nextDrawGlideScale, scaledDrawGlide } from "./draw-glide-scale"

describe("new portamento note duration scaling", () => {
  it("halves 240 ticks to 120 and doubles them to 480", () => {
    expect(scaledDrawGlide(240, "half")).toBe(120)
    expect(scaledDrawGlide(240, "double")).toBe(480)
    expect(nextDrawGlideScale(240, "half")).toBe(120)
    expect(nextDrawGlideScale(240, "double")).toBe(480)
  })

  it("rounds half of 5 ticks down to 2", () => {
    expect(scaledDrawGlide(5, "half")).toBe(2)
    expect(nextDrawGlideScale(5, "half")).toBe(2)
  })

  it("keeps 1 tick unchanged and returns null when halving", () => {
    expect(scaledDrawGlide(1, "half")).toBe(1)
    expect(nextDrawGlideScale(1, "half")).toBeNull()
  })

  it("keeps the maximum duration unchanged and returns null when doubling", () => {
    expect(scaledDrawGlide(MAX_PATTERN_TICKS, "double")).toBe(MAX_PATTERN_TICKS)
    expect(nextDrawGlideScale(MAX_PATTERN_TICKS, "double")).toBeNull()
  })

  it("caps a doubled duration at the maximum pattern length", () => {
    expect(scaledDrawGlide(MAX_PATTERN_TICKS - 1, "double")).toBe(MAX_PATTERN_TICKS)
    expect(nextDrawGlideScale(MAX_PATTERN_TICKS - 1, "double")).toBe(
      MAX_PATTERN_TICKS
    )
  })
})
