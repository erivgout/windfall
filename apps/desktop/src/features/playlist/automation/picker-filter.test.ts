import { describe, expect, it } from "vitest"

import { matchingAutomationIds } from "./picker-filter"

const items = [
  { id: 3, name: "Bass intro", target: "Bass → pan" },
  { id: 1, name: "Fade out", target: "Kick → volume" },
  { id: 2, name: "Bass chorus", target: "Lead · Cutoff" },
]

describe("matchingAutomationIds", () => {
  it("returns every id in order for a blank query", () => {
    expect(matchingAutomationIds(items, "")).toEqual([3, 1, 2])
    expect(matchingAutomationIds(items, " \t\n ")).toEqual([3, 1, 2])
  })

  it("matches names ignoring case and keeps the original order", () => {
    expect(matchingAutomationIds(items, "  bAsS  ")).toEqual([3, 2])
  })

  it("matches target words even when the name does not match", () => {
    expect(matchingAutomationIds(items, "  VOLUME  ")).toEqual([1])
  })

  it("drops an item that matches neither the name nor the target", () => {
    expect(matchingAutomationIds(items, "intro")).toEqual([3])
  })

  it("returns an empty list when nothing matches", () => {
    expect(matchingAutomationIds(items, "missing")).toEqual([])
  })
})
