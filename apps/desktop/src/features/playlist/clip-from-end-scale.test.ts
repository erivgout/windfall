import { beforeEach, describe, expect, it, vi } from "vitest"

import type { Clip } from "@/bindings"
import { MAX_SONG_TICKS } from "@/lib/units"

import {
  clipFromEndScaleUpdates,
  scaledClipFromEnd,
  setSelectedClipFromEndScale,
} from "./clip-from-end-scale"

const { dispatch, selectedClips } = vi.hoisted(() => ({
  dispatch: vi.fn().mockResolvedValue(undefined),
  selectedClips: vi.fn<() => Pick<Clip, "id" | "start" | "length">[]>(),
}))

vi.mock("@/lib/store/project", () => ({ dispatch }))
vi.mock("./selectors", () => ({ selectedClips }))

beforeEach(() => {
  dispatch.mockClear()
  selectedClips.mockReset().mockReturnValue([])
})

describe("playlist clip scaling from the end", () => {
  it.each([
    {
      start: 0,
      length: 40,
      half: { start: 20, length: 20 },
      double: null,
    },
    {
      start: 100,
      length: 40,
      half: { start: 120, length: 20 },
      double: { start: 60, length: 80 },
    },
    {
      start: 10,
      length: 1,
      half: null,
      double: { start: 9, length: 2 },
    },
    {
      start: 50,
      length: 5,
      half: { start: 53, length: 2 },
      double: { start: 45, length: 10 },
    },
    {
      start: 8,
      length: 10,
      half: { start: 13, length: 5 },
      double: { start: 0, length: 18 },
    },
    { start: 0, length: 1, half: null, double: null },
    {
      start: 3,
      length: 7,
      half: { start: 7, length: 3 },
      double: { start: 0, length: 10 },
    },
  ])(
    "keeps the end for start $start and length $length, omitting unchanged clips",
    ({ start, length, half, double }) => {
      const clip = { id: 1, start, length }
      expect(scaledClipFromEnd(start, length, "half")).toEqual(half)
      expect(scaledClipFromEnd(start, length, "double")).toEqual(double)
      expect(clipFromEndScaleUpdates([clip], "half")).toEqual(
        half === null ? [] : [{ id: 1, ...half }]
      )
      expect(clipFromEndScaleUpdates([clip], "double")).toEqual(
        double === null ? [] : [{ id: 1, ...double }]
      )
      for (const next of [half, double]) {
        if (next !== null) {
          expect(next.start + next.length).toBe(start + length)
        }
      }
    }
  )

  it.each(["half", "double"] as const)(
    "omits clips whose end passes the song limit with %s",
    (factor) => {
      const clip = { id: 1, start: MAX_SONG_TICKS - 5, length: 6 }
      expect(scaledClipFromEnd(clip.start, clip.length, factor)).toBeNull()
      expect(clipFromEndScaleUpdates([clip], factor)).toEqual([])
    }
  )

  it.each([
    { factor: "half", start: MAX_SONG_TICKS - 20, length: 20 },
    { factor: "double", start: MAX_SONG_TICKS - 80, length: 80 },
  ] as const)(
    "allows a clip ending exactly at the song limit with $factor",
    ({ factor, start, length }) => {
      expect(scaledClipFromEnd(MAX_SONG_TICKS - 40, 40, factor)).toEqual({
        start,
        length,
      })
    }
  )

  it.each(["half", "double"] as const)(
    "returns no updates for an empty selection with %s",
    (factor) => {
      expect(clipFromEndScaleUpdates([], factor)).toEqual([])
    }
  )

  it("preserves order and input clips while including only id, start, and length", () => {
    const clips = Object.freeze(
      [
        { id: 3, start: 50, length: 5, muted: true },
        { id: 2, start: 0, length: 1, muted: false },
        { id: 4, start: MAX_SONG_TICKS, length: 1, muted: false },
        { id: 1, start: 100, length: 40, muted: false },
      ].map((clip) => Object.freeze(clip))
    )
    const before = structuredClone(clips)

    expect(clipFromEndScaleUpdates(clips, "half")).toEqual([
      { id: 3, start: 53, length: 2 },
      { id: 1, start: 120, length: 20 },
    ])
    expect(clips).toEqual(before)
  })

  it.each(["half", "double"] as const)(
    "does not dispatch for an empty selection with %s",
    async (factor) => {
      await setSelectedClipFromEndScale(factor)
      expect(selectedClips).toHaveBeenCalledOnce()
      expect(dispatch).not.toHaveBeenCalled()
    }
  )

  it.each(["half", "double"] as const)(
    "does not dispatch when every selected clip is omitted with %s",
    async (factor) => {
      const clips = [
        { id: 1, start: 0, length: 1 },
        { id: 2, start: MAX_SONG_TICKS, length: 1 },
      ]
      expect(clipFromEndScaleUpdates(clips, factor)).toEqual([])
      selectedClips.mockReturnValue(clips)

      await setSelectedClipFromEndScale(factor)

      expect(selectedClips).toHaveBeenCalledOnce()
      expect(dispatch).not.toHaveBeenCalled()
    }
  )

  it.each([
    { factor: "half", start: 120, length: 20 },
    { factor: "double", start: 60, length: 80 },
  ] as const)(
    "dispatches both start and length for only changed clips with $factor",
    async ({ factor, start, length }) => {
      const clips = [
        { id: 1, start: 100, length: 40 },
        { id: 2, start: 0, length: 1 },
        { id: 3, start: MAX_SONG_TICKS - 5, length: 6 },
      ]
      const before = structuredClone(clips)
      selectedClips.mockReturnValue(clips)

      await setSelectedClipFromEndScale(factor)

      expect(selectedClips).toHaveBeenCalledOnce()
      expect(dispatch).toHaveBeenCalledExactlyOnceWith({
        type: "updateClips",
        updates: [{ id: 1, patch: { start, length } }],
      })
      expect(clips).toEqual(before)
    }
  )
})
