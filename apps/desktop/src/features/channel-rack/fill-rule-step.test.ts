import { describe, expect, it } from "vitest"

import type { FillRule } from "./advanced-fill"
import { FILL_RULES, nextFillRule } from "./fill-rule-step"

describe("advanced fill rhythm rule stepping", () => {
  it.each<{
    rule: string
    direction: "previous" | "next"
    expected: FillRule | null
  }>([
    { rule: "regular", direction: "previous", expected: null },
    { rule: "regular", direction: "next", expected: "euclidean" },
    { rule: "euclidean", direction: "previous", expected: "regular" },
    { rule: "euclidean", direction: "next", expected: "random" },
    { rule: "random", direction: "previous", expected: "euclidean" },
    { rule: "random", direction: "next", expected: null },
    { rule: "unknown", direction: "previous", expected: null },
    { rule: "unknown", direction: "next", expected: null },
  ])(
    "returns $expected for $rule moved $direction",
    ({ rule, direction, expected }) => {
      expect(nextFillRule(rule, direction)).toBe(expected)
    }
  )

  it("uses regular and random as the ends of FILL_RULES", () => {
    expect(FILL_RULES[0]).toBe("regular")
    expect(FILL_RULES[FILL_RULES.length - 1]).toBe("random")
  })
})
