import { describe, expect, it } from "vitest"

import { nextDialogOffsetScale, scaledDialogOffset } from "./dialog-offset-scale"

describe("dialog offset scale", () => {
  it.each([
    [-10, -5],
    [10, 5],
    [0.2, 0.1],
    [-0.2, -0.1],
  ])("halves %s ms to %s ms without rounding", (ms, expected) => {
    expect(scaledDialogOffset(ms, "half")).toBe(expected)
    expect(nextDialogOffsetScale(ms, "half")).toBe(expected)
  })

  it.each([
    [10, 20],
    [-10, -20],
    [600, 1000],
    [-600, -1000],
  ])("doubles %s ms to %s ms within the range", (ms, expected) => {
    expect(scaledDialogOffset(ms, "double")).toBe(expected)
    expect(nextDialogOffsetScale(ms, "double")).toBe(expected)
  })

  it.each(["half", "double"] as const)("keeps zero at no offset for %s", (factor) => {
    expect(scaledDialogOffset(0, factor)).toBe(0)
    expect(nextDialogOffsetScale(0, factor)).toBeNull()
  })

  it.each([-1000, 1000])("returns null when doubling the %s ms limit", (ms) => {
    expect(scaledDialogOffset(ms, "double")).toBe(ms)
    expect(nextDialogOffsetScale(ms, "double")).toBeNull()
  })

  it("returns null for changes smaller than 0.001 ms", () => {
    for (const sign of [-1, 1]) {
      expect(nextDialogOffsetScale(sign * 0.0018, "half")).toBeNull()
      expect(nextDialogOffsetScale(sign * 0.0009, "double")).toBeNull()
      expect(nextDialogOffsetScale(sign * 999.9995, "double")).toBeNull()
    }
  })

  it("applies changes of exactly 0.001 ms", () => {
    for (const sign of [-1, 1]) {
      expect(nextDialogOffsetScale(sign * 0.002, "half")).toBe(sign * 0.001)
      expect(nextDialogOffsetScale(sign * 0.001, "double")).toBe(sign * 0.002)
    }
  })
})
