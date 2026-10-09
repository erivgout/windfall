import { describe, expect, it } from "vitest"

import { NOTE_COLOR_GROUPS } from "@/lib/note-colors"

import { nextDrawColor } from "./draw-color-step"

describe("piano-roll draw color stepping", () => {
  const last = NOTE_COLOR_GROUPS.length - 1

  it.each<{
    current: number | null
    direction: "previous" | "next"
    expected: { color: number | null } | null
  }>([
    { current: null, direction: "previous", expected: null },
    { current: null, direction: "next", expected: { color: 0 } },
    { current: 0, direction: "previous", expected: { color: null } },
    { current: 0, direction: "next", expected: { color: 1 } },
    { current: 7, direction: "previous", expected: { color: 6 } },
    { current: 7, direction: "next", expected: { color: 8 } },
    { current: last, direction: "previous", expected: { color: last - 1 } },
    { current: last, direction: "next", expected: null },
  ])(
    "returns $expected for color $current moved $direction",
    ({ current, direction, expected }) => {
      expect(nextDrawColor(current, direction)).toEqual(expected)
    }
  )

  it.each([
    -1,
    NOTE_COLOR_GROUPS.length,
    NOTE_COLOR_GROUPS.length + 1,
    1.5,
    Number.NaN,
    Number.POSITIVE_INFINITY,
    Number.NEGATIVE_INFINITY,
  ])("does not step invalid color %s", (current) => {
    expect(nextDrawColor(current, "previous")).toBeNull()
    expect(nextDrawColor(current, "next")).toBeNull()
  })
})
