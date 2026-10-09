import { describe, expect, it } from "vitest"

import { nextSamplerTunePreset } from "./tune-preset-step"
import { TUNE_PRESETS } from "./tune-presets"

describe("sampler tune preset step", () => {
  it("starts at an octave down and ends at an octave up", () => {
    expect(TUNE_PRESETS[0].semitones).toBe(-12)
    expect(TUNE_PRESETS[TUNE_PRESETS.length - 1].semitones).toBe(12)
  })

  it.each([
    { tune: -12, previous: null, next: -7 },
    { tune: -7, previous: -12, next: 0 },
    { tune: 0, previous: -7, next: 7 },
    { tune: 7, previous: 0, next: 12 },
    { tune: 12, previous: 7, next: null },
    { tune: 3, previous: 0, next: 7 },
    { tune: -20, previous: null, next: -12 },
    { tune: 20, previous: 12, next: null },
  ])("steps tune $tune to its neighboring presets", ({ tune, previous, next }) => {
    expect(nextSamplerTunePreset(tune, tune, "previous")).toEqual(
      previous === null ? null : { semitones: previous, tune: previous }
    )
    expect(nextSamplerTunePreset(tune, tune, "next")).toEqual(
      next === null ? null : { semitones: next, tune: next }
    )
  })

  it.each([
    {
      tune: 0.25,
      coarse: 0,
      previous: { semitones: -7, tune: -6.75 },
      next: { semitones: 7, tune: 7.25 },
    },
    {
      tune: 12.25,
      coarse: 12,
      previous: { semitones: 7, tune: 7.25 },
      next: null,
    },
    {
      tune: 0.5,
      coarse: 0,
      previous: { semitones: -7, tune: -6.5 },
      next: { semitones: 7, tune: 7.5 },
    },
    {
      tune: -0.5,
      coarse: 0,
      previous: { semitones: -7, tune: -7.5 },
      next: { semitones: 7, tune: 6.5 },
    },
  ])("keeps the shown cents for tune $tune", ({ tune, coarse, previous, next }) => {
    expect(nextSamplerTunePreset(tune, coarse, "previous")).toEqual(previous)
    expect(nextSamplerTunePreset(tune, coarse, "next")).toEqual(next)
  })

  it("uses the split semitone when the coarse value is stale", () => {
    expect(nextSamplerTunePreset(3, 0, "previous")).toEqual({ semitones: 0, tune: 0 })
    expect(nextSamplerTunePreset(3, 0, "next")).toEqual({ semitones: 7, tune: 7 })
  })

  it.each([NaN, Infinity, -Infinity])(
    "returns null in both directions for non-finite tune %s",
    (tune) => {
      expect(nextSamplerTunePreset(tune, 0, "previous")).toBeNull()
      expect(nextSamplerTunePreset(tune, 0, "next")).toBeNull()
    }
  )
})
