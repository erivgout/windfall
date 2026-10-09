import { describe, expect, it } from "vitest"

import { nextPlaylistSnapScale } from "./playlist-snap-scale"
import { SNAP_MODES, type SnapMode } from "./snap"

describe("playlist snap scaling", () => {
  it("walks step, beat, then bar from the reversed snapping modes", () => {
    const walk = SNAP_MODES.filter((item) => item.mode !== "none")
      .reverse()
      .map((item) => item.mode)

    expect(walk).toEqual(["step", "beat", "bar"])
  })

  it.each<{
    mode: string
    direction: "finer" | "coarser"
    expected: SnapMode | null
  }>([
    { mode: "none", direction: "finer", expected: null },
    { mode: "none", direction: "coarser", expected: null },
    { mode: "step", direction: "finer", expected: null },
    { mode: "step", direction: "coarser", expected: "beat" },
    { mode: "beat", direction: "finer", expected: "step" },
    { mode: "beat", direction: "coarser", expected: "bar" },
    { mode: "bar", direction: "finer", expected: "beat" },
    { mode: "bar", direction: "coarser", expected: null },
    { mode: "unknown", direction: "finer", expected: null },
    { mode: "unknown", direction: "coarser", expected: null },
  ])(
    "returns $expected for $mode made $direction",
    ({ mode, direction, expected }) => {
      expect(nextPlaylistSnapScale(mode, direction)).toBe(expected)
    }
  )
})
