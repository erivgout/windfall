import { describe, expect, it } from "vitest"

import type { EqParams } from "@/bindings"
import descriptors from "@/bindings/descriptors.json"

import {
  bandResponseDb,
  cutSections,
  EQ_BAND_IDS,
  eqGainAt,
  eqResponseDb,
  logFrequencies,
  responseGrid,
  sectionGainDb,
} from "./eq-response"

const fixture = descriptors.fixtures.eqResponse
const defaults = descriptors.effects.eq.defaults as EqParams

/** The largest difference between two curves, in dB. */
function worst(actual: ArrayLike<number>, expected: ArrayLike<number>) {
  let gap = 0
  for (let index = 0; index < expected.length; index += 1) {
    gap = Math.max(gap, Math.abs(actual[index] - expected[index]))
  }
  return gap
}

describe("the equaliser curve", () => {
  it("matches the engine's curve within 0.1 dB at every fixture frequency", () => {
    const grid = responseGrid(fixture.frequenciesHz, fixture.sampleRate)
    expect(fixture.cases.length).toBeGreaterThanOrEqual(6)
    for (const reference of fixture.cases) {
      const curve = eqResponseDb(
        reference.params as EqParams,
        fixture.sampleRate,
        grid
      )
      expect(curve).toHaveLength(fixture.frequenciesHz.length)
      expect(worst(curve, reference.gainsDb)).toBeLessThan(0.1)
    }
  })

  it("is flat at the default settings", () => {
    const grid = responseGrid(logFrequencies(256), 48_000)
    const curve = eqResponseDb(defaults, 48_000, grid)
    expect(Math.max(...curve.map(Math.abs))).toBe(0)
  })

  it("is the sum of its bands and the output gain", () => {
    const params = fixture.cases[4].params as EqParams
    const grid = responseGrid(logFrequencies(64), 48_000)
    const sum = new Float32Array(64).fill(params.outputGainDb)
    for (const id of EQ_BAND_IDS) {
      const band = bandResponseDb(params, id, 48_000, grid)
      for (let index = 0; index < sum.length; index += 1) {
        sum[index] += band[index]
      }
    }
    expect(worst(sum, eqResponseDb(params, 48_000, grid))).toBeLessThan(1e-3)
  })

  it("leaves a band that is off, or at 0 dB, out of the curve", () => {
    const grid = responseGrid(logFrequencies(32), 48_000)
    const off: EqParams = {
      ...defaults,
      peak2: { ...defaults.peak2, enabled: false, gainDb: 12 },
    }
    expect(Math.max(...bandResponseDb(off, "peak2", 48_000, grid))).toBe(0)
    expect(Math.max(...bandResponseDb(defaults, "peak1", 48_000, grid))).toBe(0)
  })

  it("reaches a bell's gain at its centre and a shelf's at the far end", () => {
    const bell: EqParams = {
      ...defaults,
      peak2: { enabled: true, frequencyHz: 1000, gainDb: 9, q: 2 },
    }
    expect(eqGainAt(bell, 48_000, 1000)).toBeCloseTo(9, 3)
    const shelf: EqParams = {
      ...defaults,
      lowShelf: { ...defaults.lowShelf, frequencyHz: 200, gainDb: -6 },
    }
    expect(eqGainAt(shelf, 48_000, 20)).toBeCloseTo(-6, 1)
    expect(eqGainAt(shelf, 48_000, 200)).toBeCloseTo(-3, 1)
  })

  it("builds each cut slope from Butterworth sections", () => {
    const corner = 1000
    for (const [slope, stages, perOctave] of [
      ["db12", 1, 12],
      ["db24", 2, 24],
      ["db48", 4, 48],
    ] as const) {
      const band = {
        enabled: true,
        frequencyHz: corner,
        q: Math.SQRT1_2,
        slope,
      }
      const sections = cutSections(band, true, 96_000)
      expect(sections).toHaveLength(stages)
      const params: EqParams = { ...defaults, lowCut: band }
      // 3 dB down at the corner, whatever the order.
      expect(eqGainAt(params, 96_000, corner)).toBeCloseTo(-3.01, 1)
      const octave =
        eqGainAt(params, 96_000, corner / 8) -
        eqGainAt(params, 96_000, corner / 16)
      expect(octave).toBeCloseTo(perOctave, 0)
    }
  })

  it("follows the sample rate", () => {
    const params: EqParams = {
      ...defaults,
      highShelf: { ...defaults.highShelf, frequencyHz: 12_000, gainDb: 12 },
    }
    const at44 = eqGainAt(params, 44_100, 18_000)
    const at96 = eqGainAt(params, 96_000, 18_000)
    expect(Math.abs(at44 - at96)).toBeGreaterThan(0.05)
  })

  it("never reads a section below -200 dB", () => {
    const [section] = cutSections(
      { enabled: true, frequencyHz: 20_000, q: Math.SQRT1_2, slope: "db12" },
      true,
      48_000
    )
    expect(sectionGainDb(section, 0)).toBe(-200)
  })

  it("spaces its frequencies evenly in pitch", () => {
    const frequencies = logFrequencies(4, 20, 20_000)
    expect(Array.from(frequencies, Math.round)).toEqual([20, 200, 2000, 20_000])
  })
})
