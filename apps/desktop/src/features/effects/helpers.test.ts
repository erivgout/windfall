import { describe, expect, it } from "vitest"

import type { EffectSlot, NoteDivision } from "@/bindings"
import { effectDescriptor } from "@/features/params"

import {
  DIVISION_BEATS,
  MAX_DELAY_MS,
  nearestDivision,
  syncedDelayMs,
} from "./delay/timing"
import {
  DB_RANGES,
  frequencyLines,
  frequencyToX,
  gainLines,
  gainToY,
  plotRect,
  roundTo,
  xToFrequency,
  yToGain,
} from "./eq/geometry"
import {
  chainLatencyFrames,
  formatLatency,
  lookaheadFrames,
} from "./limiter/latency"

describe("delay timing", () => {
  it("has a length in beats for every note the core offers", () => {
    const division = effectDescriptor("delay").params.find(
      (info) => info.id === "division"
    )
    expect(Object.keys(DIVISION_BEATS).sort()).toEqual(
      division?.choices.map((choice) => choice.value).sort()
    )
    expect(DIVISION_BEATS.whole).toBe(4)
    expect(DIVISION_BEATS.eighthDotted).toBe(0.75)
    expect(DIVISION_BEATS.quarterTriplet * 3).toBeCloseTo(2)
  })

  it("turns a note length into milliseconds at a tempo", () => {
    expect(syncedDelayMs("quarter", 120)).toBe(500)
    expect(syncedDelayMs("eighth", 128)).toBeCloseTo(234.375)
    expect(syncedDelayMs("sixteenthTriplet", 150)).toBeCloseTo(66.667, 2)
  })

  it("holds a synced time at four seconds, as the core does", () => {
    expect(syncedDelayMs("whole", 30)).toBe(MAX_DELAY_MS)
    expect(syncedDelayMs("whole", 60)).toBe(MAX_DELAY_MS)
    expect(syncedDelayMs("whole", 61)).toBeLessThan(MAX_DELAY_MS)
  })

  it("finds the note a time is closest to", () => {
    expect(nearestDivision(500, 120)).toEqual({
      division: "quarter",
      exact: true,
    })
    expect(nearestDivision(375, 120)).toEqual({
      division: "eighthDotted",
      exact: true,
    })
    expect(nearestDivision(400, 120)).toEqual({
      division: "eighthDotted",
      exact: false,
    })
    for (const division of Object.keys(DIVISION_BEATS) as NoteDivision[]) {
      if (syncedDelayMs(division, 140) === MAX_DELAY_MS) continue
      expect(nearestDivision(syncedDelayMs(division, 140), 140)).toEqual({
        division,
        exact: true,
      })
    }
  })
})

describe("limiter latency", () => {
  const limiter = (lookaheadMs: number, enabled = true): EffectSlot => ({
    id: 1,
    enabled,
    mix: 1,
    params: { ...effectDescriptor("limiter").defaults, lookaheadMs },
  })
  const eq: EffectSlot = {
    id: 2,
    enabled: true,
    mix: 1,
    params: effectDescriptor("eq").defaults,
  }

  it("is the look-ahead in frames, and one frame at the least", () => {
    expect(lookaheadFrames(5, 48_000)).toBe(240)
    expect(lookaheadFrames(5, 44_100)).toBe(221)
    expect(lookaheadFrames(0.1, 48_000)).toBe(5)
    expect(lookaheadFrames(0.001, 8000)).toBe(1)
  })

  it("adds up over a chain, whether a limiter is on or off", () => {
    expect(chainLatencyFrames([], 48_000)).toBe(0)
    expect(chainLatencyFrames([eq], 48_000)).toBe(0)
    expect(
      chainLatencyFrames([limiter(5), eq, limiter(1.5, false)], 48_000)
    ).toBe(240 + 72)
  })

  it("prints frames with the time they take", () => {
    expect(formatLatency(240, 48_000)).toBe("5.0 ms (240 samples)")
    expect(formatLatency(1, 48_000)).toBe("0.0 ms (1 sample)")
    expect(formatLatency(960, 48_000)).toBe("20 ms (960 samples)")
  })
})

describe("equaliser display geometry", () => {
  const plot = plotRect(320, 160)

  it("leaves room around the axes for a node at their ends", () => {
    expect(plot.left).toBeGreaterThanOrEqual(10)
    expect(plot.left + plot.width).toBeLessThanOrEqual(310)
    expect(plot.top).toBeGreaterThanOrEqual(10)
    expect(plot.top + plot.height).toBeLessThanOrEqual(150)
  })

  it("spreads 20 Hz to 20 kHz over the width, a decade to a third", () => {
    expect(frequencyToX(20, plot)).toBe(plot.left)
    expect(frequencyToX(20_000, plot)).toBeCloseTo(plot.left + plot.width)
    const decade = frequencyToX(2000, plot) - frequencyToX(200, plot)
    expect(decade).toBeCloseTo(plot.width / 3)
    for (const hz of [20, 55, 440, 1000, 12_345, 20_000]) {
      expect(xToFrequency(frequencyToX(hz, plot), plot)).toBeCloseTo(hz, 6)
    }
  })

  it("puts 0 dB in the middle of every range", () => {
    for (const range of DB_RANGES) {
      expect(gainToY(0, plot, range)).toBeCloseTo(plot.top + plot.height / 2)
      expect(gainToY(range, plot, range)).toBe(plot.top)
      expect(gainToY(-range, plot, range)).toBeCloseTo(plot.top + plot.height)
      expect(yToGain(gainToY(3.5, plot, range), plot, range)).toBeCloseTo(3.5)
    }
  })

  it("draws a line at every decade and labels 100, 1k and 10k", () => {
    const lines = frequencyLines()
    expect(lines[0].hz).toBe(20)
    expect(lines.at(-1)?.hz).toBe(20_000)
    expect(
      lines.filter((line) => line.label).map((line) => line.label)
    ).toEqual(["100", "1k", "10k"])
    expect(lines.filter((line) => line.decade).map((line) => line.hz)).toEqual([
      100, 1000, 10_000,
    ])
  })

  it("draws four lines either side of 0 dB", () => {
    expect(gainLines(24).map((line) => line.db)).toEqual([
      -24, -18, -12, -6, 0, 6, 12, 18, 24,
    ])
    expect(
      gainLines(24)
        .filter((line) => line.label)
        .map((line) => line.label)
    ).toEqual(["−24", "−12", "0", "+12", "+24"])
    expect(gainLines(6).map((line) => line.db)).toEqual([
      -6, -4.5, -3, -1.5, 0, 1.5, 3, 4.5, 6,
    ])
  })

  it("rounds dragged values to a few digits", () => {
    expect(roundTo(1234.567, 3)).toBe(1230)
    expect(roundTo(43.21, 3)).toBe(43.2)
    expect(roundTo(0.70712, 3)).toBe(0.707)
    expect(roundTo(0, 3)).toBe(0)
  })
})
