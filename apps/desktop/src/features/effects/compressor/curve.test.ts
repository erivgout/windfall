import { describe, expect, it } from "vitest"

import type { CompressorParams } from "@/bindings"
import descriptors from "@/bindings/descriptors.json"

import {
  formatRatio,
  outputLevelDb,
  reductionDb,
  reductionSlope,
  staticGainDb,
  totalMakeupDb,
} from "./curve"

const fixture = descriptors.fixtures.compressorCurve
const defaults = descriptors.effects.compressor.defaults as CompressorParams

describe("the compressor curve", () => {
  it("matches the engine's curve within 0.05 dB at every fixture level", () => {
    expect(fixture.cases.length).toBeGreaterThanOrEqual(4)
    for (const reference of fixture.cases) {
      const params = reference.params as CompressorParams
      fixture.levelsDb.forEach((level, index) => {
        expect(
          Math.abs(staticGainDb(params, level) - reference.gainsDb[index])
        ).toBeLessThan(0.05)
      })
    }
  })

  it("matches the engine's makeup gain, the automatic part included", () => {
    for (const reference of fixture.cases) {
      const params = reference.params as CompressorParams
      expect(
        Math.abs(totalMakeupDb(params) - reference.totalMakeupDb)
      ).toBeLessThan(0.05)
    }
    expect(fixture.cases.some((item) => item.params.autoMakeup)).toBe(true)
  })

  it("treats the top of the ratio range as a limiter", () => {
    expect(reductionSlope(100)).toBe(1)
    expect(reductionSlope(1)).toBe(0)
    expect(reductionSlope(4)).toBeCloseTo(0.75)
    const brick = { ...defaults, ratio: 100, kneeDb: 0, thresholdDb: -20 }
    expect(outputLevelDb(brick, 0)).toBeCloseTo(-20)
    expect(outputLevelDb(brick, -10)).toBeCloseTo(-20)
  })

  it("joins the knee to the straight parts", () => {
    const slope = reductionSlope(4)
    expect(reductionDb(-6, slope, 12)).toBe(0)
    expect(reductionDb(6, slope, 12)).toBeCloseTo(slope * 6)
    expect(reductionDb(0, slope, 12)).toBeCloseTo(slope * 1.5)
    expect(reductionDb(-0.001, slope, 0)).toBe(0)
  })

  it("leaves a signal under the threshold alone", () => {
    expect(staticGainDb(defaults, -60)).toBe(0)
    expect(outputLevelDb(defaults, -60)).toBeCloseTo(-60)
  })

  it("adds makeup to the output and blends by the mix", () => {
    const made = { ...defaults, makeupDb: 6 }
    expect(outputLevelDb(made, -60)).toBeCloseTo(-54)
    expect(outputLevelDb({ ...made, mix: 0 }, -60)).toBeCloseTo(-60)
    const half = outputLevelDb({ ...made, mix: 0.5 }, -60)
    expect(half).toBeGreaterThan(-60)
    expect(half).toBeLessThan(-54)
  })

  it("writes the ratio the way a compressor reads", () => {
    expect(formatRatio(4)).toBe("4.0:1")
    expect(formatRatio(1.5)).toBe("1.5:1")
    expect(formatRatio(20)).toBe("20:1")
    expect(formatRatio(100)).toBe("∞:1")
  })
})
