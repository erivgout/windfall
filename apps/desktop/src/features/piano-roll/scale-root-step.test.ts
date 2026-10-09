import { describe, expect, it } from "vitest"

import { nextScaleRoot } from "./scale-root-step"

describe("piano-roll scale root stepping", () => {
  it.each<{
    root: number
    direction: "lower" | "higher"
    expected: number | null
  }>([
    { root: 0, direction: "lower", expected: 11 },
    { root: 0, direction: "higher", expected: 1 },
    { root: 11, direction: "lower", expected: 10 },
    { root: 11, direction: "higher", expected: 0 },
    { root: 7, direction: "lower", expected: 6 },
    { root: 7, direction: "higher", expected: 8 },
    { root: 1.5, direction: "lower", expected: null },
    { root: 1.5, direction: "higher", expected: null },
    { root: -1, direction: "lower", expected: null },
    { root: -1, direction: "higher", expected: null },
    { root: 12, direction: "lower", expected: null },
    { root: 12, direction: "higher", expected: null },
  ])(
    "returns $expected for root $root moved $direction",
    ({ root, direction, expected }) => {
      expect(nextScaleRoot(root, direction)).toBe(expected)
    }
  )
})
