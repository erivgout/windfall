import { describe, expect, it } from "vitest"

import { nextSignaturePreset } from "./signature-preset-step"
import { SIGNATURE_PRESETS } from "./signature-presets"

describe("signature preset stepping", () => {
  it("starts at 4/4 and ends at 12/8", () => {
    expect(SIGNATURE_PRESETS[0].label).toBe("4/4")
    expect(SIGNATURE_PRESETS[SIGNATURE_PRESETS.length - 1].label).toBe("12/8")
  })

  it.each([
    {
      numerator: 4,
      denominator: 4,
      previous: null,
      next: { numerator: 3, denominator: 4 },
    },
    {
      numerator: 3,
      denominator: 4,
      previous: { numerator: 4, denominator: 4 },
      next: { numerator: 2, denominator: 4 },
    },
    {
      numerator: 2,
      denominator: 4,
      previous: { numerator: 3, denominator: 4 },
      next: { numerator: 6, denominator: 8 },
    },
    {
      numerator: 6,
      denominator: 8,
      previous: { numerator: 2, denominator: 4 },
      next: { numerator: 5, denominator: 4 },
    },
    {
      numerator: 5,
      denominator: 4,
      previous: { numerator: 6, denominator: 8 },
      next: { numerator: 7, denominator: 8 },
    },
    {
      numerator: 7,
      denominator: 8,
      previous: { numerator: 5, denominator: 4 },
      next: { numerator: 12, denominator: 8 },
    },
    {
      numerator: 12,
      denominator: 8,
      previous: { numerator: 7, denominator: 8 },
      next: null,
    },
    { numerator: 1, denominator: 4, previous: null, next: null },
    { numerator: 4, denominator: 8, previous: null, next: null },
  ])(
    "steps from $numerator/$denominator in both directions",
    ({ numerator, denominator, previous, next }) => {
      const signature = { numerator, denominator }
      expect(nextSignaturePreset(signature, "previous")).toEqual(previous)
      expect(nextSignaturePreset(signature, "next")).toEqual(next)
    }
  )
})
