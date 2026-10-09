import { describe, expect, it } from "vitest"

import type { Clip, PlaylistTrack } from "@/bindings"

import { clipsOnMutedTracks, mutedClipIds } from "./select-muted"

function clip(id: number, track: number, muted: boolean): Clip {
  return {
    id,
    track,
    start: id * 960,
    length: 960,
    offset: 0,
    muted,
    content: { type: "pattern", pattern: 10 },
  }
}

function track(id: number, muted: boolean): PlaylistTrack {
  return { id, name: `Track ${id}`, muted }
}

describe("select muted playlist clips", () => {
  it("returns muted clips in order and leaves out a sounding clip", () => {
    const clips = [clip(3, 1, true), clip(2, 1, false), clip(1, 2, true)]

    expect(mutedClipIds(clips)).toEqual([3, 1])
  })

  it("includes clips on muted tracks in clip order even when they are not muted", () => {
    const clips = [clip(3, 2, false), clip(2, 1, true), clip(1, 2, true)]
    const tracks = [track(1, true), track(2, true)]

    expect(clipsOnMutedTracks(clips, tracks)).toEqual([3, 2, 1])
  })

  it("leaves out a muted clip on an unmuted track", () => {
    const clips = [clip(3, 1, true), clip(2, 2, false), clip(1, 3, true)]
    const tracks = [track(1, false), track(2, true)]

    expect(clipsOnMutedTracks(clips, tracks)).toEqual([2])
  })

  it("returns empty arrays when nothing matches or the inputs are empty", () => {
    const clips = [clip(1, 1, false)]

    expect(mutedClipIds(clips)).toEqual([])
    expect(clipsOnMutedTracks(clips, [track(1, false)])).toEqual([])
    expect(mutedClipIds([])).toEqual([])
    expect(clipsOnMutedTracks([], [track(1, true)])).toEqual([])
    expect(clipsOnMutedTracks(clips, [])).toEqual([])
  })

  it("does not mutate the clips or tracks", () => {
    const clips = Object.freeze([
      Object.freeze(clip(3, 1, true)),
      Object.freeze(clip(1, 2, false)),
    ])
    const tracks = Object.freeze([
      Object.freeze(track(2, true)),
      Object.freeze(track(1, false)),
    ])
    const originalClips = structuredClone(clips)
    const originalTracks = structuredClone(tracks)

    expect(mutedClipIds(clips)).toEqual([3])
    expect(clipsOnMutedTracks(clips, tracks)).toEqual([1])
    expect(clips).toEqual(originalClips)
    expect(tracks).toEqual(originalTracks)
  })
})
