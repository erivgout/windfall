import { describe, expect, it } from "vitest"

import type { BrowserRoot, Clip, Project, SampleAsset } from "@/bindings"

import {
  audioClipOverlap,
  mixerTrackOfSample,
  sampleOfFile,
} from "./audio-clips"

const audioClip = (id: number, sample: number, mixerTrack: number): Clip => ({
  id,
  track: 1,
  start: 0,
  length: 960,
  offset: 0,
  muted: false,
  content: {
    type: "audio",
    sample,
    mixerTrack,
    gain: 1,
    pan: 0,
    fadeIn: 0,
    fadeOut: 0,
    reverse: false,
    pitch: 0,
  },
})

const projectWith = (clips: Clip[], tracks: number[]) =>
  ({
    mixer: { tracks: tracks.map((id) => ({ id })) },
    playlist: { clips },
  }) as unknown as Pick<Project, "mixer" | "playlist">

describe("mixerTrackOfSample", () => {
  it("is the track of the most recent clip of the sample", () => {
    const project = projectWith(
      [audioClip(30, 7, 4), audioClip(12, 7, 3), audioClip(40, 8, 5)],
      [0, 3, 4, 5]
    )
    expect(mixerTrackOfSample(project, 7)).toBe(4)
    expect(mixerTrackOfSample(project, 8)).toBe(5)
  })

  it("is nothing for a sample with no clip, or whose track is gone", () => {
    const project = projectWith([audioClip(12, 7, 9)], [0, 3])
    expect(mixerTrackOfSample(project, 7)).toBeUndefined()
    expect(mixerTrackOfSample(project, 99)).toBeUndefined()
  })
})

describe("sampleOfFile", () => {
  const samples: SampleAsset[] = [
    { id: 1, name: "Kick", path: { kind: "factory", path: "Drums/Kick.wav" } },
    {
      id: 2,
      name: "Take",
      path: { kind: "project", path: "Samples/Take.wav" },
    },
    {
      id: 3,
      name: "Loop",
      path: { kind: "external", path: "D:\\Loops\\Loop.wav" },
    },
  ]
  const roots: BrowserRoot[] = [
    { name: "Factory", path: "C:\\Windfall\\factory", kind: "factory" },
    { name: "Mine", path: "D:\\Loops", kind: "user" },
  ]
  const where = { roots, projectPath: "D:\\Songs\\One\\One.windfall" }

  it("finds a factory sound by its place in the factory folder", () => {
    expect(
      sampleOfFile(samples, "C:\\Windfall\\factory\\Drums\\Kick.wav", where)?.id
    ).toBe(1)
    // The same relative path in a folder of the user's is another file.
    expect(
      sampleOfFile(samples, "D:\\Loops\\Drums\\Kick.wav", where)
    ).toBeUndefined()
  })

  it("finds a project sample beside the project file, and any other by its path", () => {
    expect(
      sampleOfFile(samples, "D:/Songs/One/Samples/Take.wav", where)?.id
    ).toBe(2)
    expect(
      sampleOfFile(samples, "D:/Songs/One/Samples/Take.wav", {
        roots,
        projectPath: null,
      })
    ).toBeUndefined()
    expect(sampleOfFile(samples, "D:/Loops/Loop.wav", where)?.id).toBe(3)
    expect(sampleOfFile(samples, "D:/Loops/Other.wav", where)).toBeUndefined()
  })
})

describe("audioClipOverlap", () => {
  const BAR = 3840
  const span = (
    id: number,
    start: number,
    length: number,
    more: Partial<Clip> = {}
  ): Clip => ({ ...audioClip(id, 1, 1), start, length, ...more })
  const tracks = (muted: number[] = []) =>
    [1, 2].map((id) => ({
      id,
      name: `Track ${id}`,
      color: 0,
      muted: muted.includes(id),
    })) as unknown as Project["playlist"]["tracks"]
  /** `count` clips on top of each other from `start`. */
  const stack = (count: number, start: number, from = 0) =>
    Array.from({ length: count }, (_, index) =>
      span(from + index, start, 2 * BAR)
    )

  it("counts the most clips that sound together", () => {
    const clips = [
      span(1, 0, 2 * BAR),
      span(2, BAR, 2 * BAR),
      span(3, BAR + 10, BAR),
      // Starts on the tick the first one ends: back to back, not together.
      span(4, 2 * BAR, BAR),
    ]
    expect(audioClipOverlap({ clips, tracks: tracks() })).toEqual({
      most: 3,
      overAt: null,
    })
    expect(audioClipOverlap({ clips: [], tracks: tracks() })).toEqual({
      most: 0,
      overAt: null,
    })
  })

  it("says where more than 64 first overlap", () => {
    const clips = [...stack(64, 0), ...stack(80, 8 * BAR, 100)]
    expect(audioClipOverlap({ clips, tracks: tracks() })).toEqual({
      most: 80,
      // The 65th of the second pile is the first one too many.
      overAt: 8 * BAR,
    })
    // Exactly 64 all sound.
    expect(
      audioClipOverlap({ clips: stack(64, 0), tracks: tracks() }).overAt
    ).toBeNull()
    expect(
      audioClipOverlap({ clips: stack(65, 3 * BAR), tracks: tracks() }).overAt
    ).toBe(3 * BAR)
  })

  it("leaves out muted clips, clips on muted tracks and clips that are not audio", () => {
    const pile = stack(70, 0)
    const muted = pile.map((clip, index) =>
      index < 6 ? { ...clip, muted: true } : clip
    )
    expect(audioClipOverlap({ clips: muted, tracks: tracks() })).toEqual({
      most: 64,
      overAt: null,
    })
    const moved = pile.map((clip, index) =>
      index < 10 ? { ...clip, track: 2 } : clip
    )
    expect(audioClipOverlap({ clips: moved, tracks: tracks([2]) }).most).toBe(
      60
    )
    const patterns = pile.map((clip, index) =>
      index < 10
        ? ({ ...clip, content: { type: "pattern", pattern: 1 } } as Clip)
        : clip
    )
    expect(audioClipOverlap({ clips: patterns, tracks: tracks() }).most).toBe(
      60
    )
  })

  it("takes another limit", () => {
    expect(
      audioClipOverlap({ clips: stack(3, BAR), tracks: tracks() }, 2).overAt
    ).toBe(BAR)
  })
})
