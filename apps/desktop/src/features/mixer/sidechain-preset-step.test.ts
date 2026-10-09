import { describe, expect, it } from "vitest"

import { nextSidechainPreset } from "./sidechain-preset-step"
import { SIDECHAIN_PRESETS } from "./sidechain-presets"

describe("sidechain preset stepping", () => {
  it("starts at Off and ends at Loud", () => {
    expect(SIDECHAIN_PRESETS[0].value).toBe(0)
    expect(SIDECHAIN_PRESETS[SIDECHAIN_PRESETS.length - 1].value).toBe(1.5)
  })

  it.each([
    { gain: 0, previous: null, next: 0.5 },
    { gain: 0.5, previous: 0, next: 1 },
    { gain: 1, previous: 0.5, next: 1.5 },
    { gain: 1.5, previous: 1, next: null },
    { gain: 0.7, previous: 0.5, next: 1 },
    { gain: -0.1, previous: null, next: 0 },
    { gain: 2, previous: 1.5, next: null },
    { gain: 0.5004, previous: 0, next: 1 },
  ])("steps $gain to previous $previous and next $next", ({ gain, previous, next }) => {
    expect(nextSidechainPreset(gain, "previous")).toBe(previous)
    expect(nextSidechainPreset(gain, "next")).toBe(next)
  })

  it.each([NaN, Infinity, -Infinity])("returns null for non-finite gain %s", (gain) => {
    expect(nextSidechainPreset(gain, "previous")).toBeNull()
    expect(nextSidechainPreset(gain, "next")).toBeNull()
  })
})
