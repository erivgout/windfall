import { describe, expect, it } from "vitest"

import { nextSignature, SIGNATURE_PRESETS } from "./signature-presets"

describe("signature presets", () => {
  it("lists the presets in order with their numerator and denominator", () => {
    expect(SIGNATURE_PRESETS).toEqual([
      { label: "4/4", numerator: 4, denominator: 4 },
      { label: "3/4", numerator: 3, denominator: 4 },
      { label: "2/4", numerator: 2, denominator: 4 },
      { label: "6/8", numerator: 6, denominator: 8 },
      { label: "5/4", numerator: 5, denominator: 4 },
      { label: "7/8", numerator: 7, denominator: 8 },
      { label: "12/8", numerator: 12, denominator: 8 },
    ])
  })

  it("returns null when the current signature already matches", () => {
    expect(
      nextSignature({ numerator: 4, denominator: 4 }, SIGNATURE_PRESETS[0])
    ).toBeNull()
  })

  it("returns only the new numerator and denominator when they differ", () => {
    const current = { numerator: 4, denominator: 4, extra: "current" }
    const preset = { ...SIGNATURE_PRESETS[3], extra: "preset" }

    expect(nextSignature(current, preset)).toEqual({
      numerator: 6,
      denominator: 8,
    })
    expect(current).toEqual({ numerator: 4, denominator: 4, extra: "current" })
    expect(preset).toEqual({
      label: "6/8",
      numerator: 6,
      denominator: 8,
      extra: "preset",
    })
  })

  it("changes a matching numerator when the denominator differs", () => {
    expect(
      nextSignature({ numerator: 6, denominator: 4 }, SIGNATURE_PRESETS[3])
    ).toEqual({ numerator: 6, denominator: 8 })
  })
})
