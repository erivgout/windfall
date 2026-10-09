import { describe, expect, it } from "vitest"

import type { SampleAsset } from "@/bindings"

import { matchingPatternIds } from "../pattern-filter"

const samples: SampleAsset[] = [
  {
    id: 3,
    name: "Bass loop",
    path: { kind: "external", path: "/samples/bass-loop.wav" },
  },
  {
    id: 1,
    name: "Drums",
    path: { kind: "external", path: "/samples/drums.wav" },
  },
  {
    id: 2,
    name: "Bass hit",
    path: { kind: "external", path: "/samples/bass-hit.wav" },
  },
]

describe("matchingPatternIds for sounds", () => {
  it("returns every sample id in order for a blank query", () => {
    expect(matchingPatternIds(samples, "")).toEqual([3, 1, 2])
    expect(matchingPatternIds(samples, " \t\n ")).toEqual([3, 1, 2])
  })

  it("ignores case when matching sample names", () => {
    expect(matchingPatternIds(samples, "bAsS")).toEqual([3, 2])
  })

  it("drops a sample whose name does not contain the query", () => {
    expect(matchingPatternIds(samples, "loop")).toEqual([3])
  })

  it("returns an empty list when no sample names match", () => {
    expect(matchingPatternIds(samples, "missing")).toEqual([])
  })
})
