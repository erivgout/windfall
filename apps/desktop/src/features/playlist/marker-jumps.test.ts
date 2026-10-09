import { describe, expect, it } from "vitest"

import type { TimelineMarker } from "@/bindings"

import { markerJumps } from "./marker-jumps"

function marker(overrides: Partial<TimelineMarker> = {}): TimelineMarker {
  return {
    id: 1,
    tick: 960,
    name: "Chorus",
    kind: { type: "named" },
    ...overrides,
  }
}

describe("markerJumps", () => {
  it("uses the marker name and tick", () => {
    expect(markerJumps([marker()])).toEqual([
      { title: "Jump to Chorus", tick: 960 },
    ])
  })

  it("uses a fallback title for blank names", () => {
    expect(
      markerJumps([marker({ name: "" }), marker({ name: " \t\n " })])
    ).toEqual([
      { title: "Jump to marker", tick: 960 },
      { title: "Jump to marker", tick: 960 },
    ])
  })

  it("orders markers by tick", () => {
    expect(
      markerJumps([
        marker({ id: 1, tick: 960 }),
        marker({ id: 2, tick: 480, name: "Verse" }),
      ])
    ).toEqual([
      { title: "Jump to Verse", tick: 480 },
      { title: "Jump to Chorus", tick: 960 },
    ])
  })

  it("orders markers at the same tick by id", () => {
    expect(
      markerJumps([
        marker({ id: 2, name: "Second" }),
        marker({ id: 1, name: "First" }),
      ])
    ).toEqual([
      { title: "Jump to First", tick: 960 },
      { title: "Jump to Second", tick: 960 },
    ])
  })

  it("seeks the start tick of a loop marker", () => {
    expect(
      markerJumps([marker({ kind: { type: "loop", end: 1920 } })])
    ).toEqual([{ title: "Jump to Chorus", tick: 960 }])
  })

  it("includes skip and pause markers and uses the skip start tick", () => {
    expect(
      markerJumps([
        marker({ id: 1, name: "Skip", kind: { type: "skip", end: 1920 } }),
        marker({ id: 2, name: "Pause", kind: { type: "pause" } }),
      ])
    ).toEqual([
      { title: "Jump to Skip", tick: 960 },
      { title: "Jump to Pause", tick: 960 },
    ])
  })

  it("does not mutate the input markers", () => {
    const later = Object.freeze(marker({ id: 1, tick: 960 }))
    const earlier = Object.freeze(marker({ id: 2, tick: 480 }))
    const markers = Object.freeze([later, earlier])

    markerJumps(markers)

    expect(markers).toEqual([later, earlier])
  })

  it("returns no entries for an empty list", () => {
    expect(markerJumps([])).toEqual([])
  })
})
