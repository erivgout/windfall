import { describe, expect, it } from "vitest"

import { nextSamplerTuneScale } from "./tune-scale"

describe("sampler tune scale", () => {
  it.each([
    {
      tune: 12,
      coarse: 12,
      factor: "half",
      expected: { tune: 6, semitones: 6 },
    },
    {
      tune: 12,
      coarse: 12,
      factor: "double",
      expected: { tune: 24, semitones: 24 },
    },
    {
      tune: 12.25,
      coarse: 12,
      factor: "half",
      expected: { tune: 6.25, semitones: 6 },
    },
    {
      tune: 12.25,
      coarse: 12,
      factor: "double",
      expected: { tune: 24.25, semitones: 24 },
    },
    {
      tune: -12.25,
      coarse: -12,
      factor: "half",
      expected: { tune: -6.25, semitones: -6 },
    },
    { tune: 5, coarse: 5, factor: "half", expected: { tune: 2, semitones: 2 } },
    {
      tune: -5,
      coarse: -5,
      factor: "half",
      expected: { tune: -2, semitones: -2 },
    },
    { tune: 0, coarse: 0, factor: "half", expected: null },
    { tune: 0, coarse: 0, factor: "double", expected: null },
    { tune: 0.25, coarse: 0, factor: "half", expected: null },
    { tune: 0.25, coarse: 0, factor: "double", expected: null },
    { tune: 48, coarse: 48, factor: "double", expected: null },
    { tune: -48, coarse: -48, factor: "double", expected: null },
    { tune: 24.25, coarse: 24, factor: "double", expected: null },
    {
      tune: 30,
      coarse: 30,
      factor: "double",
      expected: { tune: 48, semitones: 48 },
    },
    {
      tune: 1.5,
      coarse: 1,
      factor: "half",
      expected: { tune: 0.5, semitones: 0 },
    },
  ] as const)(
    "$factor of tune $tune with coarse $coarse returns $expected",
    ({ tune, coarse, factor, expected }) => {
      expect(nextSamplerTuneScale(tune, coarse, factor)).toEqual(expected)
    }
  )
})
