import { describe, expect, it } from "vitest"

import type { Clip, ClipContent } from "@/bindings"

import { selectMatchingClips } from "./select-matching"

function clip(id: number, content: ClipContent): Clip {
  return {
    id,
    track: 1,
    start: id * 960,
    length: 960,
    offset: 0,
    muted: false,
    content,
  }
}

function audio(
  sample: number,
  overrides: Partial<Extract<ClipContent, { type: "audio" }>> = {}
): ClipContent {
  return {
    type: "audio",
    sample,
    mixerTrack: 1,
    gain: 1,
    pan: 0,
    fadeIn: 0,
    fadeOut: 0,
    reverse: false,
    pitch: 0,
    ...overrides,
  }
}

describe("select matching playlist clips", () => {
  it("selects every instance of one pattern and leaves the other pattern alone", () => {
    const clips = [
      clip(1, { type: "pattern", pattern: 10 }),
      clip(2, { type: "pattern", pattern: 20 }),
      clip(3, { type: "pattern", pattern: 10 }),
      clip(4, { type: "pattern", pattern: 20 }),
      clip(5, { type: "pattern", pattern: 10 }),
    ]

    expect(selectMatchingClips([clips[2]], clips)).toEqual([3, 1, 5])
  })

  it("matches audio by sample id regardless of playback settings or mixer track", () => {
    const clips = [
      clip(1, audio(10)),
      clip(2, audio(20)),
      clip(
        3,
        audio(10, {
          mixerTrack: 2,
          gain: 0.5,
          pan: -0.75,
          fadeIn: 120,
          fadeOut: 240,
          reverse: true,
          pitch: 12,
          normalize: true,
          stretch: {
            mode: "spectral",
            ratio: 2,
            quality: "high",
            formants: true,
          },
        })
      ),
    ]

    expect(selectMatchingClips([clips[0]], clips)).toEqual([1, 3])
  })

  it("keeps an empty selection empty without mutating either input", () => {
    const selected: Clip[] = []
    const clips = [clip(1, { type: "pattern", pattern: 10 })]
    const before = structuredClone(clips)

    expect(selectMatchingClips(selected, clips)).toEqual([])
    expect(selected).toEqual([])
    expect(clips).toEqual(before)
  })

  it("expands pattern and audio families independently even when source ids coincide", () => {
    const clips = [
      clip(1, { type: "pattern", pattern: 10 }),
      clip(2, audio(20)),
      clip(3, { type: "pattern", pattern: 10 }),
      clip(4, audio(20)),
      clip(5, audio(10)),
      clip(6, { type: "pattern", pattern: 20 }),
      clip(7, { type: "automation", automation: 10 }),
      clip(8, { type: "automation", automation: 20 }),
    ]

    expect(selectMatchingClips([clips[0], clips[1]], clips)).toEqual([
      1, 2, 3, 4,
    ])
  })

  it("matches automation ids without selecting patterns or audio with the same id", () => {
    const clips = [
      clip(1, { type: "automation", automation: 10 }),
      clip(2, { type: "automation", automation: 20 }),
      clip(3, { type: "automation", automation: 10 }),
      clip(4, { type: "pattern", pattern: 10 }),
      clip(5, audio(10)),
    ]

    expect(selectMatchingClips([clips[0]], clips)).toEqual([1, 3])
  })

  it("preserves original clips and removes duplicates without mutating the inputs", () => {
    const original = clip(1, { type: "pattern", pattern: 10 })
    const matching = clip(2, { type: "pattern", pattern: 10 })
    const selected = [original, original]
    const clips = [matching, matching]
    const before = structuredClone({ selected, clips })

    expect(selectMatchingClips(selected, clips)).toEqual([1, 2])
    expect({ selected, clips }).toEqual(before)
  })
})
