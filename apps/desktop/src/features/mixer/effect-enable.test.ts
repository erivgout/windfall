import { describe, expect, it } from "vitest"

import type { EffectId, EffectSlot, MixerTrack, TrackId } from "@/bindings"

import { effectEnableUpdates } from "./effect-enable"

function slot(id: EffectId, enabled: boolean): EffectSlot {
  return {
    id,
    enabled,
    mix: 0.5,
    params: { type: "balance", gain: 1, pan: 0 },
  }
}

function track(id: TrackId, effects: EffectSlot[] = []): MixerTrack {
  return {
    id,
    name: `Track ${id}`,
    color: 0,
    volume: 1,
    pan: 0,
    muted: false,
    solo: false,
    output: null,
    sends: [],
    effects,
  }
}

describe("effectEnableUpdates", () => {
  it("bypasses enabled slots and omits bypassed slots in track and slot order", () => {
    const tracks = [
      track(4, [slot(8, true), slot(2, false), slot(6, true)]),
      track(1, [slot(3, false), slot(5, true)]),
    ]

    expect(effectEnableUpdates(tracks, false)).toEqual([
      { track: 4, effect: 8 },
      { track: 4, effect: 6 },
      { track: 1, effect: 5 },
    ])
  })

  it("enables bypassed slots and omits enabled slots", () => {
    const tracks = [
      track(4, [slot(8, false), slot(2, true), slot(6, false)]),
      track(1, [slot(3, true), slot(5, false)]),
    ]

    expect(effectEnableUpdates(tracks, true)).toEqual([
      { track: 4, effect: 8 },
      { track: 4, effect: 6 },
      { track: 1, effect: 5 },
    ])
  })

  it("gets nothing from a track with no slots", () => {
    expect(effectEnableUpdates([track(4)], false)).toEqual([])
    expect(effectEnableUpdates([track(4)], true)).toEqual([])
  })

  it("returns nothing for an empty track list", () => {
    expect(effectEnableUpdates([], false)).toEqual([])
    expect(effectEnableUpdates([], true)).toEqual([])
  })
})
