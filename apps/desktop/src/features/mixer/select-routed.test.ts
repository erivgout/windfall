import { describe, expect, it } from "vitest"

import type { MixerTrack, TrackId } from "@/bindings"

import { routedTrackIds } from "./select-routed"

function track(
  id: TrackId,
  output: TrackId | null,
  sends: MixerTrack["sends"] = []
): MixerTrack {
  return {
    id,
    name: `Track ${id}`,
    color: 0,
    volume: 1,
    pan: 0,
    muted: false,
    solo: false,
    output,
    sends,
    effects: [],
  }
}

describe("routedTrackIds", () => {
  it("keeps tracks whose output is the target in the given order", () => {
    const tracks = [track(4, 2), track(1, 0), track(3, 2), track(2, 0)]

    expect(routedTrackIds(tracks, 2)).toEqual([4, 3])
  })

  it("omits the target itself even when its output points at itself", () => {
    expect(routedTrackIds([track(2, 2), track(1, 2)], 2)).toEqual([1])
  })

  it("omits a track with a null output", () => {
    expect(routedTrackIds([track(1, null), track(2, 0)], 0)).toEqual([2])
  })

  it("omits a track that only sends to the target with a different output", () => {
    const tracks = [track(1, 0, [{ target: 2, gain: 1 }]), track(3, 2)]

    expect(routedTrackIds(tracks, 2)).toEqual([3])
  })

  it("returns nothing for a target that nobody routes to", () => {
    expect(routedTrackIds([track(0, null), track(1, 0), track(2, 0)], 2))
      .toEqual([])
  })
})
