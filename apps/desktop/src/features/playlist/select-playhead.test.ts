import { describe, expect, it } from "vitest"

import type { Clip } from "@/bindings"

import { clipsAtTick } from "./select-playhead"

function clip(id: number, start: number, length: number, muted = false): Clip {
  return {
    id,
    track: 1,
    start,
    length,
    offset: 0,
    muted,
    content: { type: "pattern", pattern: 1 },
  }
}

describe("select playlist clips at the playhead", () => {
  it("includes a clip covering the tick and excludes one ending on it", () => {
    expect(clipsAtTick([clip(1, 0, 960), clip(2, 0, 480)], 480)).toEqual([1])
  })

  it("includes a clip starting on the tick", () => {
    expect(clipsAtTick([clip(1, 480, 960)], 480)).toEqual([1])
  })

  it("keeps clips in the given order", () => {
    const clips = [clip(3, 240, 960), clip(1, 0, 960), clip(2, 480, 960)]

    expect(clipsAtTick(clips, 480)).toEqual([3, 1, 2])
  })

  it("excludes clips elsewhere in the song", () => {
    expect(clipsAtTick([clip(1, 0, 240), clip(2, 960, 480)], 480)).toEqual([])
  })

  it("includes muted clips that still cover the tick", () => {
    expect(clipsAtTick([clip(1, 0, 960, true)], 480)).toEqual([1])
  })
})
