import { describe, expect, it } from "vitest"
import { TapTempoEstimator } from "./tap-tempo"
import { MAX_TEMPO_BPM, MIN_TEMPO_BPM } from "@/lib/units"

describe("tap tempo measurement", () => {
  it("estimates a steady rhythm without accepting bounce or invalid timestamps", () => {
    const meter = new TapTempoEstimator()
    expect(meter.tap(1000)).toEqual({ bpm: null, taps: 1 })
    expect(meter.tap(1500)).toEqual({ bpm: 120, taps: 2 })
    for (const time of [1550, 1490, NaN, Infinity, -Infinity])
      expect(meter.tap(time)).toEqual({ bpm: 120, taps: 2 })
    expect(meter.tap(2000)).toEqual({ bpm: 120, taps: 3 })
  })
  it("ignores a missed beat in an established rhythm", () => {
    const meter = new TapTempoEstimator()
    for (const time of [0, 500, 1000, 2000]) meter.tap(time)
    expect(meter.tap(2500)).toEqual({ bpm: 120, taps: 5 })
  })
  it("supports the full project tempo range and resets after a long pause", () => {
    for (const bpm of [MIN_TEMPO_BPM, MAX_TEMPO_BPM]) {
      const meter = new TapTempoEstimator()
      meter.tap(0)
      expect(meter.tap(60_000 / bpm).bpm).toBe(bpm)
      expect(meter.tap(20_000)).toEqual({ bpm: null, taps: 1 })
    }
  })
  it("bounds retained taps and eventually follows a new rhythm", () => {
    const meter = new TapTempoEstimator()
    let time = 0
    for (let i = 0; i < 40; i++) {
      expect(meter.tap(time).taps).toBeLessThanOrEqual(8)
      time += 500
    }
    for (let i = 0; i < 8; i++) {
      meter.tap(time)
      time += 1000
    }
    expect(meter.tap(time)).toEqual({ bpm: 60, taps: 8 })
    expect(meter.reset()).toEqual({ bpm: null, taps: 0 })
  })
  it("keeps a finite estimate when a mixed rhythm has no median inliers", () => {
    const meter = new TapTempoEstimator()
    for (const time of [0, 120, 240, 1440]) meter.tap(time)
    expect(meter.tap(2640)).toEqual({ bpm: 90.909, taps: 5 })
  })
})
