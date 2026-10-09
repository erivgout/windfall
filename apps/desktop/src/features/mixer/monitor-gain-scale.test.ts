import { describe, expect, it } from "vitest"

import { nextMonitorGainScale, scaledMonitorGain } from "./monitor-gain-scale"

describe("monitor gain scale", () => {
  it.each([
    { gain: 0, factor: "half", scaled: 0, next: null },
    { gain: 0, factor: "double", scaled: 0, next: null },
    { gain: 1, factor: "double", scaled: 1, next: null },
    { gain: 0.5, factor: "half", scaled: 0.25, next: 0.25 },
    { gain: 0.5, factor: "double", scaled: 1, next: 1 },
    { gain: 0.25, factor: "double", scaled: 0.5, next: 0.5 },
    { gain: 0.6, factor: "double", scaled: 1, next: 1 },
  ] as const)("scales $gain by $factor to $scaled with next $next", ({ gain, factor, scaled, next }) => {
    expect(scaledMonitorGain(gain, factor)).toBe(scaled)
    expect(nextMonitorGainScale(gain, factor)).toBe(next)
  })

  it.each(["half", "double"] as const)("preserves stored gain precision for %s", (factor) => {
    const gain = 0.123456789
    const expected = factor === "half" ? gain / 2 : gain * 2
    expect(scaledMonitorGain(gain, factor)).toBe(expected)
    expect(nextMonitorGainScale(gain, factor)).toBe(expected)
  })

  it("returns null for changes smaller than 0.001", () => {
    expect(nextMonitorGainScale(0.0018, "half")).toBeNull()
    expect(nextMonitorGainScale(0.0009, "double")).toBeNull()
    expect(nextMonitorGainScale(0.9991, "double")).toBeNull()
  })

  it("applies changes of exactly 0.001", () => {
    expect(nextMonitorGainScale(0.002, "half")).toBe(0.001)
    expect(nextMonitorGainScale(0.001, "double")).toBe(0.002)
  })
})
