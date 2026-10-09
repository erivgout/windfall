import { describe, expect, it } from "vitest"

import type { ArrangementBook, ClipId } from "@/bindings"

import { emptyArrangementBook } from "./model"
import { activeArrangementClipIds } from "./select-clips"

function book(clips: ClipId[]): ArrangementBook {
  return {
    ...emptyArrangementBook(),
    arrangements: [
      { id: 10, name: "Intro", clips: [99], tracks: [] },
      { id: 20, name: "Verse", clips, tracks: [] },
    ],
    active: 20,
  }
}

describe("select arrangement clips", () => {
  it("keeps the active arrangement's live clips in its order and drops stale ids", () => {
    const value = book([8, 4, 7, 2])
    const liveClipIds = [2, 7, 4, 99]
    const before = structuredClone({ value, liveClipIds })

    expect(activeArrangementClipIds(value, liveClipIds)).toEqual([4, 7, 2])
    expect({ value, liveClipIds }).toEqual(before)
  })

  it("keeps a repeated id once at its first place", () => {
    expect(activeArrangementClipIds(book([7, 4, 7, 2, 4]), [2, 4, 7])).toEqual([
      7, 4, 2,
    ])
  })

  it("returns an empty array for an active arrangement with no clips", () => {
    expect(activeArrangementClipIds(book([]), [2, 4, 7])).toEqual([])
  })

  it("returns null when there is no active arrangement", () => {
    expect(
      activeArrangementClipIds({ ...book([7]), active: null }, [7])
    ).toBeNull()
  })

  it("returns null when the active id is not in the book", () => {
    expect(
      activeArrangementClipIds({ ...book([7]), active: 30 }, [7])
    ).toBeNull()
  })
})
