import { describe, expect, it } from "vitest"
import { MAX_SONG_TICKS } from "@/lib/units"

import { nextDialogLoopScale, scaledDialogLoopEnd } from "./dialog-loop-scale"

describe("dialog loop scale", () => {
  it.each([
    [0, 3840, "half", 1920],
    [0, 3840, "double", 7680],
    [0, 1, "half", null],
    [0, 1, "double", 2],
    [0, 5, "half", 2],
    [0, 5, "double", 10],
    [100, 340, "half", 220],
    [100, 340, "double", 580],
    [0, 3, "half", 1],
    [0, 3, "double", 6],
    [MAX_SONG_TICKS - 50, MAX_SONG_TICKS - 10, "double", MAX_SONG_TICKS],
    [0, MAX_SONG_TICKS, "double", null],
    [MAX_SONG_TICKS - 1, MAX_SONG_TICKS, "half", null],
    [MAX_SONG_TICKS - 1, MAX_SONG_TICKS, "double", null],
  ] as const)("scales start %s and end %s by %s to %s", (start, end, factor, expected) => {
    expect(scaledDialogLoopEnd(start, end, factor)).toBe(expected ?? end)
    expect(nextDialogLoopScale(start, end, factor)).toBe(expected)
  })
})
