import { describe, expect, it } from "vitest"

import { matchingChannelIds } from "./channel-filter"

const channels = [
  { id: 3, name: "Bass intro" },
  { id: 1, name: "Drums" },
  { id: 2, name: "Bass chorus" },
]

describe("matchingChannelIds", () => {
  it("returns every id in order for a blank query", () => {
    expect(matchingChannelIds(channels, "")).toEqual([3, 1, 2])
    expect(matchingChannelIds(channels, " \t\n ")).toEqual([3, 1, 2])
  })

  it("ignores case and trims the query", () => {
    expect(matchingChannelIds(channels, "  bAsS  ")).toEqual([3, 2])
  })

  it("drops a channel whose name does not contain the query", () => {
    expect(matchingChannelIds(channels, "intro")).toEqual([3])
  })

  it("returns an empty list when no names match", () => {
    expect(matchingChannelIds(channels, "missing")).toEqual([])
  })
})
