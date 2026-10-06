// SPDX-License-Identifier: MIT
import { describe, expect, it } from "vitest"

import {
  dbToGain,
  formatDb,
  formatGain,
  formatHz,
  formatMs,
  formatPan,
  formatPercent,
  formatSemitones,
  gainToDb,
  gainUnit,
  parseDb,
  parseGain,
  parseHz,
  parseMs,
  parseNumber,
  parsePan,
  parsePercent,
  percentUnit,
} from "./units"

describe("gain and decibels", () => {
  it("converts both ways", () => {
    expect(gainToDb(1)).toBe(0)
    expect(gainToDb(2)).toBeCloseTo(6.0206, 4)
    expect(gainToDb(0.5)).toBeCloseTo(-6.0206, 4)
    expect(dbToGain(0)).toBe(1)
    expect(dbToGain(-6)).toBeCloseTo(0.5012, 4)
    expect(dbToGain(gainToDb(0.123))).toBeCloseTo(0.123, 10)
  })

  it("treats silence as minus infinity", () => {
    expect(gainToDb(0)).toBe(-Infinity)
    expect(gainToDb(-1)).toBe(-Infinity)
    expect(dbToGain(-Infinity)).toBe(0)
    expect(dbToGain(Number.NaN)).toBe(0)
  })
})

describe("formatters", () => {
  it("formats decibels with a sign and a real minus", () => {
    expect(formatDb(0)).toBe("0.0 dB")
    expect(formatDb(3)).toBe("+3.0 dB")
    expect(formatDb(-6)).toBe("−6.0 dB")
    expect(formatDb(-6.04, 2)).toBe("−6.04 dB")
    expect(formatDb(-0.01)).toBe("0.0 dB")
    expect(formatDb(-Infinity)).toBe("−∞ dB")
    expect(formatDb(-150)).toBe("−∞ dB")
  })

  it("formats gain as decibels", () => {
    expect(formatGain(1)).toBe("0.0 dB")
    expect(formatGain(2)).toBe("+6.0 dB")
    expect(formatGain(0)).toBe("−∞ dB")
  })

  it("formats pan", () => {
    expect(formatPan(0)).toBe("C")
    expect(formatPan(0.004)).toBe("C")
    expect(formatPan(-0.5)).toBe("L50")
    expect(formatPan(1)).toBe("R100")
  })

  it("formats percent, time, pitch and frequency", () => {
    expect(formatPercent(0.5)).toBe("50%")
    expect(formatPercent(0.125, 1)).toBe("12.5%")
    expect(formatMs(0)).toBe("0 ms")
    expect(formatMs(1.5)).toBe("1.5 ms")
    expect(formatMs(250)).toBe("250 ms")
    expect(formatMs(1250)).toBe("1.25 s")
    expect(formatMs(12500)).toBe("12.5 s")
    expect(formatSemitones(7)).toBe("+7 st")
    expect(formatSemitones(-12)).toBe("−12 st")
    expect(formatSemitones(0)).toBe("0 st")
    expect(formatHz(440)).toBe("440 Hz")
    expect(formatHz(8000)).toBe("8.00 kHz")
  })
})

describe("parsers", () => {
  it("reads plain and decorated numbers", () => {
    expect(parseNumber("-6")).toBe(-6)
    expect(parseNumber("−6 dB")).toBe(-6)
    expect(parseNumber("+3,5")).toBe(3.5)
    expect(parseNumber(".5")).toBe(0.5)
    expect(parseNumber("  12.25 ms ")).toBe(12.25)
    expect(parseNumber("abc")).toBeNull()
    expect(parseNumber("")).toBeNull()
  })

  it("reads infinity", () => {
    expect(parseNumber("-inf")).toBe(-Infinity)
    expect(parseNumber("−∞ dB")).toBe(-Infinity)
    expect(parseDb("-inf")).toBe(-Infinity)
    expect(parseGain("-inf")).toBe(0)
  })

  it("reads each unit", () => {
    expect(parseGain("-6 dB")).toBeCloseTo(0.5012, 4)
    expect(parseGain("0")).toBe(1)
    expect(parsePercent("50%")).toBe(0.5)
    expect(parsePercent("50")).toBe(0.5)
    expect(parsePan("C")).toBe(0)
    expect(parsePan("center")).toBe(0)
    expect(parsePan("L50")).toBe(-0.5)
    expect(parsePan("50 r")).toBe(0.5)
    expect(parsePan("L")).toBe(-1)
    expect(parsePan("-25")).toBe(-0.25)
    expect(parsePan("what")).toBeNull()
    expect(parseMs("250")).toBe(250)
    expect(parseMs("250 ms")).toBe(250)
    expect(parseMs("1.2 s")).toBe(1200)
    expect(parseMs("1.2s")).toBe(1200)
    expect(parseHz("1.2k")).toBe(1200)
    expect(parseHz("440 Hz")).toBe(440)
  })

  it("reads back what the matching formatter prints", () => {
    for (const gain of [0, 0.25, 0.5, 1, 1.5, 2]) {
      const back = gainUnit.parse(gainUnit.format(gain))
      expect(back).not.toBeNull()
      expect(gainToDb(back ?? 0)).toBeCloseTo(gainToDb(gain), 1)
    }
    expect(percentUnit.parse(percentUnit.format(0.37))).toBeCloseTo(0.37, 10)
    expect(parsePan(formatPan(-0.3))).toBeCloseTo(-0.3, 10)
    expect(parseMs(formatMs(1250))).toBe(1250)
  })
})
