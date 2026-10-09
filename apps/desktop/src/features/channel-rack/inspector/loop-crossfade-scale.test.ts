import { describe, expect, it } from "vitest"

import {
  nextLoopCrossfadeScale,
  scaledLoopCrossfade,
} from "./loop-crossfade-scale"

describe("loop crossfade scale", () => {
  it("halves 0.5 to 0.25", () => {
    expect(scaledLoopCrossfade(0.5, "half")).toBe(0.25)
    expect(nextLoopCrossfadeScale(0.5, "half")).toBe(0.25)
  })

  it("doubles 0.5 to 1", () => {
    expect(scaledLoopCrossfade(0.5, "double")).toBe(1)
    expect(nextLoopCrossfadeScale(0.5, "double")).toBe(1)
  })

  it("doubles 0.25 to 0.5", () => {
    expect(scaledLoopCrossfade(0.25, "double")).toBe(0.5)
    expect(nextLoopCrossfadeScale(0.25, "double")).toBe(0.5)
  })

  it("caps double at 1", () => {
    expect(scaledLoopCrossfade(0.6, "double")).toBe(1)
    expect(nextLoopCrossfadeScale(0.6, "double")).toBe(1)
  })

  it("keeps no crossfade when halved", () => {
    expect(scaledLoopCrossfade(0, "half")).toBe(0)
    expect(nextLoopCrossfadeScale(0, "half")).toBeNull()
  })

  it("keeps a long crossfade when doubled", () => {
    expect(scaledLoopCrossfade(1, "double")).toBe(1)
    expect(nextLoopCrossfadeScale(1, "double")).toBeNull()
  })

  it("returns null when the scaled result differs by less than 0.001", () => {
    expect(nextLoopCrossfadeScale(0.0018, "half")).toBeNull()
    expect(nextLoopCrossfadeScale(0.0009, "double")).toBeNull()
    expect(nextLoopCrossfadeScale(0.9991, "double")).toBeNull()
  })

  it("keeps a change of exactly 0.001", () => {
    expect(nextLoopCrossfadeScale(0.002, "half")).toBe(0.001)
    expect(nextLoopCrossfadeScale(0.001, "double")).toBe(0.002)
  })
})
