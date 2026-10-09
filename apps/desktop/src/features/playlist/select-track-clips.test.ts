import { describe, expect, it } from "vitest"

import type { Clip, PlaylistTrackId } from "@/bindings"

import { clipIdsOnTrack } from "./select-track-clips"

function clip(id: number, track: PlaylistTrackId): Clip {
  return {
    id,
    track,
    start: id * 960,
    length: 960,
    offset: 0,
    muted: false,
    content: { type: "pattern", pattern: 10 },
  }
}

describe("select playlist track clips", () => {
  it("keeps clips on the named track in order and drops another track's clip", () => {
    const clips = [clip(3, 1), clip(2, 2), clip(1, 1)]

    expect(clipIdsOnTrack(clips, 1)).toEqual([3, 1])
  })

  it("returns nothing for a track with no clips", () => {
    expect(clipIdsOnTrack([clip(1, 1)], 2)).toEqual([])
  })

  it("returns nothing for an empty clip list", () => {
    expect(clipIdsOnTrack([], 1)).toEqual([])
  })
})
