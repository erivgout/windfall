import { describe, expect, it } from "vitest"

import { matchingPatternIds } from "./pattern-filter"

const patterns = [
  { id: 3, name: "Bass intro" },
  { id: 1, name: "Drums" },
  { id: 2, name: "Bass chorus" },
]

describe("matchingPatternIds", () => {
  it("returns every id in order for a blank query", () => {
    expect(matchingPatternIds(patterns, "")).toEqual([3, 1, 2])
    expect(matchingPatternIds(patterns, " \t\n ")).toEqual([3, 1, 2])
  })

  it("ignores case and trims the query", () => {
    expect(matchingPatternIds(patterns, "  bAsS  ")).toEqual([3, 2])
  })

  it("drops a pattern whose name does not contain the query", () => {
    expect(matchingPatternIds(patterns, "intro")).toEqual([3])
  })

  it("keeps matching ids in their original order", () => {
    expect(matchingPatternIds(patterns, "ass")).toEqual([3, 2])
  })

  it("returns an empty list when no names match", () => {
    expect(matchingPatternIds(patterns, "missing")).toEqual([])
  })
})
