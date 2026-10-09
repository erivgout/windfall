import { describe, expect, it } from "vitest"

import { matchingRuns } from "./history-filter"

const runs = [
  { label: "Add channel", first: 1, last: 1 },
  { label: "Toggle step", first: 2, last: 5 },
  { label: "Add pattern", first: 6, last: 6 },
]

describe("matchingRuns", () => {
  it("returns every run for a blank query", () => {
    expect(matchingRuns(runs, "")).toEqual(runs)
    expect(matchingRuns(runs, " \t\n ")).toEqual(runs)
  })

  it("ignores case and trims the query", () => {
    expect(matchingRuns(runs, "  tOgGlE  ")).toEqual([runs[1]])
  })

  it("drops runs whose labels do not contain the query without reordering", () => {
    expect(matchingRuns(runs, "add")).toEqual([runs[0], runs[2]])
  })

  it("keeps a matching run's first and last indexes", () => {
    const kept = matchingRuns(runs, "step")
    expect(kept).toEqual([{ label: "Toggle step", first: 2, last: 5 }])
    expect(kept[0]).toBe(runs[1])
    expect(runs[1]).toEqual({ label: "Toggle step", first: 2, last: 5 })
  })

  it("returns an empty list when no labels match", () => {
    expect(matchingRuns(runs, "missing")).toEqual([])
  })
})
