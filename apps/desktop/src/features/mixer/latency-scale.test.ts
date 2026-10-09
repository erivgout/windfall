import { describe, expect, it } from "vitest"

import { nextLatencyScale, scaledLatency } from "./latency-scale"

describe("latency correction scaling", () => {
  it.each([
    [10, "half", 5],
    [10, "double", 20],
    [-10, "half", -5],
    [-10, "double", -20],
    [5, "half", 2.5],
    [600, "double", 1000],
    [-600, "double", -1000],
  ] as const)("scales %s ms by %s to %s ms", (ms, factor, expected) => {
    expect(scaledLatency(ms, factor)).toBe(expected)
    expect(nextLatencyScale(ms, factor)).toBe(expected)
  })

  it.each([
    [0, "half"],
    [0, "double"],
    [1000, "double"],
    [-1000, "double"],
  ] as const)("keeps %s ms unchanged for %s without a command", (ms, factor) => {
    expect(scaledLatency(ms, factor)).toBe(ms)
    expect(nextLatencyScale(ms, factor)).toBeNull()
  })

  it.each([
    [0.0018, "half"],
    [-0.0018, "half"],
    [0.0009, "double"],
    [-0.0009, "double"],
    [999.9995, "double"],
    [-999.9995, "double"],
  ] as const)("skips changes smaller than 0.001 for %s ms and %s", (ms, factor) => {
    expect(nextLatencyScale(ms, factor)).toBeNull()
  })

  it.each([
    [0.002, "half", 0.001],
    [-0.002, "half", -0.001],
    [0.001, "double", 0.002],
    [-0.001, "double", -0.002],
  ] as const)("applies a change of exactly 0.001 for %s ms and %s", (ms, factor, expected) => {
    expect(nextLatencyScale(ms, factor)).toBe(expected)
  })
})
