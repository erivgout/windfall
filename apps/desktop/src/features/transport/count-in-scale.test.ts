import { describe, expect, it } from "vitest"

import { nextCountInScale, scaledCountIn } from "./count-in-scale"

describe("count-in scale", () => {
  it.each<[number, number | null, number | null]>([
    [0, 0, null],
    [1, 0, 0],
    [2, 1, 1],
    [4, 2, 2],
    [8, 4, 4],
    [3, null, null],
  ])("halves %s bars to %s with next change %s", (bars, scaled, next) => {
    expect(scaledCountIn(bars, "half")).toBe(scaled)
    expect(nextCountInScale(bars, "half")).toBe(next)
  })

  it.each<[number, number | null, number | null]>([
    [0, 0, null],
    [1, 2, 2],
    [2, 4, 4],
    [4, 8, 8],
    [8, 8, null],
    [3, null, null],
  ])("doubles %s bars to %s with next change %s", (bars, scaled, next) => {
    expect(scaledCountIn(bars, "double")).toBe(scaled)
    expect(nextCountInScale(bars, "double")).toBe(next)
  })
})
