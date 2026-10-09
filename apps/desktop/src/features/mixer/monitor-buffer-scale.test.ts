import { describe, expect, it } from "vitest"

import { nextMonitorBufferScale, scaledMonitorBuffer } from "./monitor-buffer-scale"

describe("monitor buffer scale", () => {
  it.each([
    { ms: 5, factor: "half", scaled: 5, next: null },
    { ms: 100, factor: "double", scaled: 100, next: null },
    { ms: 20, factor: "half", scaled: 10, next: 10 },
    { ms: 20, factor: "double", scaled: 40, next: 40 },
    { ms: 12, factor: "half", scaled: 6, next: 6 },
    { ms: 9, factor: "half", scaled: 5, next: 5 },
    { ms: 11, factor: "half", scaled: 5, next: 5 },
    { ms: 51, factor: "double", scaled: 100, next: 100 },
    { ms: 50, factor: "double", scaled: 100, next: 100 },
  ] as const)("scales $ms ms by $factor to $scaled with next $next", ({ ms, factor, scaled, next }) => {
    expect(scaledMonitorBuffer(ms, factor)).toBe(scaled)
    expect(nextMonitorBufferScale(ms, factor)).toBe(next)
  })
})
