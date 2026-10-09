import { describe, expect, it } from "vitest"

import { nextClipStretchQuality, STRETCH_QUALITIES } from "./clip-quality-step"

describe("playlist clip stretch quality stepping", () => {
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
      expect(nextClipStretchQuality(quality, direction)).toBe(expected)
    }
  )

  it("orders qualities from fast to high", () => {
    expect(STRETCH_QUALITIES[0]).toBe("fast")
    expect(STRETCH_QUALITIES[STRETCH_QUALITIES.length - 1]).toBe("high")
  })
})
