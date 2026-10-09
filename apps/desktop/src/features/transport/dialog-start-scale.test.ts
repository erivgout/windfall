import { describe, expect, it } from "vitest"
import { MAX_SONG_TICKS } from "@/lib/units"

import { nextDialogStartScale, scaledDialogStart } from "./dialog-start-scale"

describe("dialog start scale", () => {
  it.each([
    [0, "half", null],
    [0, "double", null],
    [1, "half", 0],
    [1, "double", 2],
    [5, "half", 2],
    [5, "double", 10],
    [3840, "half", 1920],
    [3840, "double", 7680],
    [MAX_SONG_TICKS, "double", null],
    [MAX_SONG_TICKS - 100, "double", MAX_SONG_TICKS],
  ] as const)("scales start %s by %s to %s", (start, factor, expected) => {
    expect(scaledDialogStart(start, factor)).toBe(expected ?? start)
    expect(nextDialogStartScale(start, factor)).toBe(expected)
  })
})
