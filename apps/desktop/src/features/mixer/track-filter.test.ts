import { describe, expect, it } from "vitest"

import { matchingTrackIds } from "./track-filter"

const tracks = [
  { id: 4, name: "Sub Kick" },
  { id: 2, name: "Clap" },
  { id: 7, name: "KICK bus" },
]

describe("matchingTrackIds", () => {
  it("returns every id in order for a blank query", () => {
    expect(matchingTrackIds(tracks, "")).toEqual([4, 2, 7])
    expect(matchingTrackIds(tracks, " \t\n ")).toEqual([4, 2, 7])
  })

  it("matches names ignoring case and preserves their order", () => {
    expect(matchingTrackIds(tracks, " kIcK ")).toEqual([4, 7])
  })

  it("drops a track whose name does not contain the query", () => {
    expect(matchingTrackIds(tracks, "lap")).toEqual([2])
  })

  it("returns an empty list when no names match", () => {
    expect(matchingTrackIds(tracks, "Snare")).toEqual([])
  })
})
