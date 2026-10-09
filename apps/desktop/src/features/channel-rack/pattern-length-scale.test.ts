import { describe, expect, it } from "vitest"

import { nextPatternLengthScale, scaledPatternLength } from "./pattern-length-scale"

describe("pattern length scale", () => {
  it("halves 16 steps to 8", () => {
    expect(scaledPatternLength(16, "half")).toBe(8)
    expect(nextPatternLengthScale(16, "half")).toBe(8)
  })

  it("doubles 16 steps to 32", () => {
    expect(scaledPatternLength(16, "double")).toBe(32)
    expect(nextPatternLengthScale(16, "double")).toBe(32)
  })

  it("halves 5 steps to 2", () => {
    expect(scaledPatternLength(5, "half")).toBe(2)
    expect(nextPatternLengthScale(5, "half")).toBe(2)
  })

  it("halves 3 steps to 1", () => {
    expect(scaledPatternLength(3, "half")).toBe(1)
    expect(nextPatternLengthScale(3, "half")).toBe(1)
  })

  it("doubles 48 steps to 96", () => {
    expect(scaledPatternLength(48, "double")).toBe(96)
    expect(nextPatternLengthScale(48, "double")).toBe(96)
  })

  it("caps double of 513 steps at 1024", () => {
    expect(scaledPatternLength(513, "double")).toBe(1024)
    expect(nextPatternLengthScale(513, "double")).toBe(1024)
  })

  it("keeps a 1-step pattern at 1 when halved without a change", () => {
    expect(scaledPatternLength(1, "half")).toBe(1)
    expect(nextPatternLengthScale(1, "half")).toBeNull()
  })

  it("keeps a 1024-step pattern at 1024 when doubled without a change", () => {
    expect(scaledPatternLength(1024, "double")).toBe(1024)
    expect(nextPatternLengthScale(1024, "double")).toBeNull()
  })
})
