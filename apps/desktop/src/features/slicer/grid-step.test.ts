import { describe, expect, it } from "vitest"
import { PPQ } from "@/lib/units"
import { nextSliceGrid, SLICE_GRIDS } from "./grid-step"

describe("slice grid steps", () => {
  it("orders grids from a sixteenth note to four beats", () => {
    expect(SLICE_GRIDS[0]).toBe(PPQ / 4)
    expect(SLICE_GRIDS[SLICE_GRIDS.length - 1]).toBe(PPQ * 4)
  })

  it.each([
    { ticks: PPQ / 4, direction: "finer", expected: null },
    { ticks: PPQ / 4, direction: "coarser", expected: PPQ / 2 },
    { ticks: PPQ / 2, direction: "finer", expected: PPQ / 4 },
    { ticks: PPQ / 2, direction: "coarser", expected: PPQ },
    { ticks: PPQ, direction: "finer", expected: PPQ / 2 },
    { ticks: PPQ, direction: "coarser", expected: PPQ * 2 },
    { ticks: PPQ * 2, direction: "finer", expected: PPQ },
    { ticks: PPQ * 2, direction: "coarser", expected: PPQ * 4 },
    { ticks: PPQ * 4, direction: "finer", expected: PPQ * 2 },
    { ticks: PPQ * 4, direction: "coarser", expected: null },
    { ticks: 100, direction: "finer", expected: null },
    { ticks: 100, direction: "coarser", expected: null },
  ] as const)(
    "moves $ticks ticks $direction to $expected",
    ({ ticks, direction, expected }) => {
      expect(nextSliceGrid(ticks, direction)).toBe(expected)
    }
  )
})
