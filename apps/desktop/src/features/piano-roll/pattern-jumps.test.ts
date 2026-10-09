import { describe, expect, it } from "vitest"

import { TICKS_PER_STEP } from "@/lib/units"

import { patternEdgeTick } from "./pattern-jumps"

describe("patternEdgeTick", () => {
  it("starts a 16-step pattern at 0", () => {
    expect(patternEdgeTick(16, "start")).toBe(0)
  })

  it("ends a 16-step pattern at its last playable tick", () => {
    expect(patternEdgeTick(16, "end")).toBe(16 * TICKS_PER_STEP - 1)
  })

  it("ends a 1-step pattern at its last playable tick", () => {
    expect(patternEdgeTick(1, "end")).toBe(TICKS_PER_STEP - 1)
  })

  it("ends a zero-length pattern at 0", () => {
    expect(patternEdgeTick(0, "end")).toBe(0)
  })

  it("ends a negative-length pattern at 0", () => {
    expect(patternEdgeTick(-1, "end")).toBe(0)
  })
})
