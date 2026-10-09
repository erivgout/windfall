import { describe, expect, it } from "vitest"

import { nextSamplerQuality, PREPARATION_QUALITIES } from "./sampler-quality-step"

describe("sampler preparation quality stepping", () => {
  it("starts with fast and ends with high", () => {
    expect(PREPARATION_QUALITIES[0]).toBe("fast")
    expect(PREPARATION_QUALITIES[PREPARATION_QUALITIES.length - 1]).toBe("high")
  })

  it.each<{
    quality: string
    direction: "previous" | "next"
    expected: "fast" | "standard" | "high" | null
  }>([
    { quality: "fast", direction: "previous", expected: null },
    { quality: "fast", direction: "next", expected: "standard" },
    { quality: "standard", direction: "previous", expected: "fast" },
    { quality: "standard", direction: "next", expected: "high" },
    { quality: "high", direction: "previous", expected: "standard" },
    { quality: "high", direction: "next", expected: null },
    { quality: "unknown", direction: "previous", expected: null },
    { quality: "unknown", direction: "next", expected: null },
  ])(
    "returns $expected for $quality moved $direction",
    ({ quality, direction, expected }) => {
      expect(nextSamplerQuality(quality, direction)).toBe(expected)
    }
  )
})
