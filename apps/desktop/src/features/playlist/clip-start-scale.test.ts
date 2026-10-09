import { beforeEach, describe, expect, it, vi } from "vitest"

import type { Clip } from "@/bindings"
import { MAX_SONG_TICKS } from "@/lib/units"

import {
  clipStartScaleUpdates,
  scaledClipStart,
  setSelectedClipStartScale,
} from "./clip-start-scale"

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

describe("playlist clip start scaling", () => {
  it.each([
    { start: 0, length: 10, half: 0, double: 0 },
    { start: 1, length: 10, half: 0, double: 2 },
    { start: 5, length: 10, half: 2, double: 10 },
    { start: 100, length: 40, half: 50, double: 200 },
    { start: 3840, length: 3840, half: 1920, double: 7680 },
    { start: 0, length: MAX_SONG_TICKS, half: 0, double: 0 },
    {
      start: MAX_SONG_TICKS - 50,
      length: 40,
      half: Math.floor((MAX_SONG_TICKS - 50) / 2),
      double: MAX_SONG_TICKS - 40,
    },
    {
      start: MAX_SONG_TICKS - 40,
      length: 40,
      half: Math.floor((MAX_SONG_TICKS - 40) / 2),
      double: MAX_SONG_TICKS - 40,
    },
  ])(
    "scales start $start with length $length and omits unchanged starts",
    ({ start, length, half, double }) => {
      const clip = { id: 1, start, length }
      expect(scaledClipStart(start, length, "half")).toBe(half)
      expect(scaledClipStart(start, length, "double")).toBe(double)
      expect(clipStartScaleUpdates([clip], "half")).toEqual(
        half === start ? [] : [{ id: 1, start: half }]
      )
      expect(clipStartScaleUpdates([clip], "double")).toEqual(
        double === start ? [] : [{ id: 1, start: double }]
      )
    }
  )

  it("clamps half to tick 0", () => {
    expect(scaledClipStart(-1, 10, "half")).toBe(0)
  })

  it("keeps the start when the length exceeds the song limit", () => {
    expect(scaledClipStart(5, MAX_SONG_TICKS + 1, "double")).toBe(5)
    expect(
      clipStartScaleUpdates(
        [{ id: 1, start: 5, length: MAX_SONG_TICKS + 1 }],
        "double"
      )
    ).toEqual([])
  })

  it.each(["half", "double"] as const)(
    "returns no updates for an empty selection with %s",
    (factor) => {
      expect(clipStartScaleUpdates([], factor)).toEqual([])
    }
  )

  it.each(["half", "double"] as const)(
    "returns no updates when every clip is unchanged with %s",
    (factor) => {
      expect(
        clipStartScaleUpdates(
          [
            { id: 1, start: 0, length: MAX_SONG_TICKS },
            { id: 2, start: 0, length: MAX_SONG_TICKS },
          ],
          factor
        )
      ).toEqual([])
    }
  )

  it("preserves order, includes only id and start, and keeps input lengths", () => {
    const clips = Object.freeze(
      [
        { id: 3, start: 5, length: 10, muted: true },
        { id: 2, start: 0, length: 10, muted: false },
        { id: 1, start: 100, length: 40, muted: false },
      ].map((clip) => Object.freeze(clip))
    )
    const before = structuredClone(clips)

    expect(clipStartScaleUpdates(clips, "half")).toEqual([
      { id: 3, start: 2 },
      { id: 1, start: 50 },
    ])
    expect(clips).toEqual(before)
  })

  it.each(["half", "double"] as const)(
    "does not dispatch for an empty selection with %s",
    async (factor) => {
      await setSelectedClipStartScale(factor)
      expect(selectedClips).toHaveBeenCalledOnce()
      expect(dispatch).not.toHaveBeenCalled()
    }
  )

  it.each(["half", "double"] as const)(
    "does not dispatch when every selected clip is unchanged with %s",
    async (factor) => {
      selectedClips.mockReturnValue([
        { id: 1, start: 0, length: MAX_SONG_TICKS },
        { id: 2, start: 0, length: MAX_SONG_TICKS },
      ])
      await setSelectedClipStartScale(factor)
      expect(dispatch).not.toHaveBeenCalled()
    }
  )

  it.each([
    { factor: "half", start: 50 },
    { factor: "double", start: 200 },
  ] as const)(
    "dispatches only changed start patches with $factor",
    async ({ factor, start }) => {
      const clips = [
        { id: 1, start: 100, length: 40 },
        { id: 2, start: 0, length: MAX_SONG_TICKS },
      ]
      const before = structuredClone(clips)
      selectedClips.mockReturnValue(clips)

      await setSelectedClipStartScale(factor)

      expect(selectedClips).toHaveBeenCalledOnce()
      expect(dispatch).toHaveBeenCalledExactlyOnceWith({
        type: "updateClips",
        updates: [{ id: 1, patch: { start } }],
      })
      expect(clips).toEqual(before)
    }
  )
})
