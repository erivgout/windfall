import { describe, expect, it } from "vitest"

import type { Clip, ClipContent } from "@/bindings"

import { makeUniqueClipIds } from "./make-unique"

function clip(id: number, content: ClipContent): Clip {
  return {
    id,
    track: 1,
    start: 0,
    length: 960,
    offset: 0,
    muted: false,
    content,
  }
}

describe("make unique playlist clips", () => {
  it("keeps pattern and audio clips in order and drops automation clips", () => {
    const selected = [
      clip(3, { type: "pattern", pattern: 10 }),
      clip(2, { type: "automation", automation: 20 }),
      clip(1, {
        type: "audio",
        sample: 30,
        mixerTrack: 1,
        gain: 1,
        pan: 0,
        fadeIn: 0,
        fadeOut: 0,
        reverse: false,
        pitch: 0,
      }),
    ]

    expect(makeUniqueClipIds(selected)).toEqual([3, 1])
  })

  it("returns nothing for a selection of only automation clips", () => {
    expect(
      makeUniqueClipIds([
        clip(1, { type: "automation", automation: 10 }),
        clip(2, { type: "automation", automation: 20 }),
      ])
    ).toEqual([])
  })

  it("returns nothing for an empty selection", () => {
    expect(makeUniqueClipIds([])).toEqual([])
  })
})
