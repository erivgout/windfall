import { describe, expect, it } from "vitest"

import {
  fileName,
  formatClock,
  formatPosition,
  formatSampleRate,
  formatTempo,
  normalizeTempo,
  parseTempo,
  tickToPosition,
  ticksPerBar,
  ticksPerBeat,
  ticksToSeconds,
} from "./time"
import { colorToCss, dbToGain, gainToDb } from "./units"

const fourFour = { numerator: 4, denominator: 4 }
const sixEight = { numerator: 6, denominator: 8 }

describe("musical time", () => {
  it("measures beats and bars in ticks", () => {
    expect(ticksPerBeat(fourFour)).toBe(960)
    expect(ticksPerBar(fourFour)).toBe(3840)
    expect(ticksPerBeat(sixEight)).toBe(480)
    expect(ticksPerBar(sixEight)).toBe(2880)
    expect(ticksPerBar({ numerator: 3, denominator: 4 })).toBe(2880)
  })

  it("counts bars, beats and steps from one", () => {
    expect(tickToPosition(0, fourFour)).toEqual({ bar: 1, beat: 1, step: 1 })
    expect(tickToPosition(239.9, fourFour)).toEqual({
      bar: 1,
      beat: 1,
      step: 1,
    })
    expect(tickToPosition(240, fourFour)).toEqual({ bar: 1, beat: 1, step: 2 })
    expect(tickToPosition(960, fourFour)).toEqual({ bar: 1, beat: 2, step: 1 })
    expect(tickToPosition(3840 + 960 * 3 + 720, fourFour)).toEqual({
      bar: 2,
      beat: 4,
      step: 4,
    })
    expect(tickToPosition(2880 + 480 * 5 + 240, sixEight)).toEqual({
      bar: 2,
      beat: 6,
      step: 2,
    })
    expect(tickToPosition(-5, fourFour)).toEqual({ bar: 1, beat: 1, step: 1 })
  })

  it("formats a position at a fixed width", () => {
    expect(formatPosition(0, fourFour)).toBe("001:01:1")
    expect(formatPosition(3840 * 11 + 960 * 2 + 480, fourFour)).toBe("012:03:3")
    expect(formatPosition(3840 * 120, fourFour)).toBe("121:01:1")
  })

  it("turns ticks into seconds at a tempo", () => {
    expect(ticksToSeconds(960, 120)).toBe(0.5)
    expect(ticksToSeconds(3840, 60)).toBe(4)
  })
})

describe("clock and tempo text", () => {
  it("formats minutes, seconds and hundredths", () => {
    expect(formatClock(0)).toBe("0:00.00")
    expect(formatClock(7.256)).toBe("0:07.25")
    expect(formatClock(67.5)).toBe("1:07.50")
    expect(formatClock(600)).toBe("10:00.00")
    expect(formatClock(-1)).toBe("0:00.00")
  })

  it("shows a tempo with two decimals", () => {
    expect(formatTempo(128)).toBe("128.00")
    expect(formatTempo(99.555)).toBe("99.56")
  })

  it("reads a typed tempo and keeps it in range", () => {
    expect(parseTempo("128")).toBe(128)
    expect(parseTempo(" 95,5 ")).toBe(95.5)
    expect(parseTempo("1000")).toBe(522)
    expect(parseTempo("1")).toBe(10)
    expect(parseTempo("")).toBeNull()
    expect(parseTempo("fast")).toBeNull()
    expect(normalizeTempo(120.12345)).toBe(120.123)
  })

  it("names sample rates and files", () => {
    expect(formatSampleRate(48_000)).toBe("48 kHz")
    expect(formatSampleRate(44_100)).toBe("44.1 kHz")
    expect(fileName("C:\\Music\\Beat.windfall")).toBe("Beat.windfall")
    expect(fileName("/projects/night.windfall")).toBe("night.windfall")
  })
})

describe("units", () => {
  it("converts between gain and decibels", () => {
    expect(gainToDb(1)).toBe(0)
    expect(gainToDb(2)).toBeCloseTo(6.02, 2)
    expect(gainToDb(0)).toBe(-Infinity)
    expect(dbToGain(-Infinity)).toBe(0)
    expect(dbToGain(gainToDb(0.5))).toBeCloseTo(0.5)
  })

  it("writes a color as CSS", () => {
    expect(colorToCss(0xe5488f)).toBe("#e5488f")
    expect(colorToCss(0x00ff)).toBe("#0000ff")
  })
})
