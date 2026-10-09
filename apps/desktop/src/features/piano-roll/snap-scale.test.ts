import { describe, expect, it } from "vitest"

import { SNAP_OPTIONS, type SnapId } from "./snap"
import { nextSnapScale } from "./snap-scale"

const grids = SNAP_OPTIONS.filter((option) => option.id !== "none")
const neighbors = grids.flatMap((option, index) => [
  { id: option.id, direction: "finer" as const, expected: grids[index - 1]?.id ?? null },
  { id: option.id, direction: "coarser" as const, expected: grids[index + 1]?.id ?? null },
])

describe("piano-roll snap scaling", () => {
  it.each(neighbors)("moves $id one entry $direction", ({ id, direction, expected }) => {
    expect(nextSnapScale(id, direction)).toBe(expected)
  })

  it.each<{
    id: string
    direction: "finer" | "coarser"
    expected: SnapId | null
  }>([
    { id: "none", direction: "finer", expected: null },
    { id: "none", direction: "coarser", expected: null },
    { id: "unknown", direction: "finer", expected: null },
    { id: "unknown", direction: "coarser", expected: null },
    { id: "step/6", direction: "finer", expected: null },
    { id: "bar", direction: "coarser", expected: null },
    { id: "step", direction: "finer", expected: "step/2" },
    { id: "step", direction: "coarser", expected: "beat/6" },
    { id: "beat", direction: "finer", expected: "beat/2" },
    { id: "beat", direction: "coarser", expected: "bar" },
    { id: "bar", direction: "finer", expected: "beat" },
    { id: "step/2", direction: "finer", expected: "step/3" },
    { id: "step/2", direction: "coarser", expected: "step" },
  ])("returns $expected for $id made $direction", ({ id, direction, expected }) => {
    expect(nextSnapScale(id, direction)).toBe(expected)
  })
})
