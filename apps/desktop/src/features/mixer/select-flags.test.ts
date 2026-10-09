import { describe, expect, it } from "vitest"

import type { MixerTrack, TrackId } from "@/bindings"

import { mutedTrackIds, soloTrackIds } from "./select-flags"

function track(id: TrackId, muted = false, solo = false): MixerTrack {
  return {
    id,
    name: `Track ${id}`,
    color: 0,
    volume: 1,
    pan: 0,
    muted,
    solo,
    output: null,
    sends: [],
    effects: [],
  }
}

describe("mixer flag selection", () => {
  it("returns muted tracks in order and leaves out unmuted tracks", () => {
    const tracks = [track(4, true), track(1), track(3, true)]

    expect(mutedTrackIds(tracks)).toEqual([4, 3])
  })

  it("returns solo tracks in order and leaves out unsoloed tracks", () => {
    const tracks = [track(4, false, true), track(1), track(3, false, true)]

    expect(soloTrackIds(tracks)).toEqual([4, 3])
  })

  it("includes a track that is both muted and solo in both lists", () => {
    const tracks = [track(2, true, true)]

    expect(mutedTrackIds(tracks)).toEqual([2])
    expect(soloTrackIds(tracks)).toEqual([2])
  })

  it("returns empty arrays when no tracks match", () => {
    expect(mutedTrackIds([track(1)])).toEqual([])
    expect(soloTrackIds([track(1)])).toEqual([])
    expect(mutedTrackIds([])).toEqual([])
    expect(soloTrackIds([])).toEqual([])
  })
})
