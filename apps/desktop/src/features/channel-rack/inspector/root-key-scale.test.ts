import { describe, expect, it } from "vitest"

import { DEFAULT_KEY } from "@/lib/units"

import { nextRootKeyScale, scaledRootKey } from "./root-key-scale"

describe("root key scale", () => {
  it.each([
    { key: DEFAULT_KEY, factor: "half", expected: DEFAULT_KEY, next: null },
    { key: DEFAULT_KEY, factor: "double", expected: DEFAULT_KEY, next: null },
    { key: 72, factor: "half", expected: 66, next: 66 },
    { key: 72, factor: "double", expected: 84, next: 84 },
    { key: 48, factor: "half", expected: 54, next: 54 },
    { key: 48, factor: "double", expected: 36, next: 36 },
    { key: 61, factor: "half", expected: DEFAULT_KEY, next: DEFAULT_KEY },
    { key: 61, factor: "double", expected: 62, next: 62 },
    { key: 59, factor: "half", expected: DEFAULT_KEY, next: DEFAULT_KEY },
    { key: 59, factor: "double", expected: 58, next: 58 },
    { key: 0, factor: "double", expected: 0, next: null },
    { key: 127, factor: "double", expected: 127, next: null },
    { key: 96, factor: "double", expected: 127, next: 127 },
    { key: 24, factor: "double", expected: 0, next: 0 },
  ] as const)(
    "$factor of key $key scales to $expected and returns $next",
    ({ key, factor, expected, next }) => {
      expect(scaledRootKey(key, factor)).toBe(expected)
      expect(nextRootKeyScale(key, factor)).toBe(next)
    }
  )
})
