import { describe, expect, it } from "vitest"

import { nextCutGroupPreset } from "./cut-group-preset-step"
import { CUT_GROUP_PRESETS } from "./cut-group-presets"

const [first, second, third, fourth] = CUT_GROUP_PRESETS.map((item) => item.cutGroup)

describe("sampler cut-group preset stepping", () => {
  it("reads the preset groups as 0, 1, 2, and 3", () => {
    expect(CUT_GROUP_PRESETS.map((item) => item.cutGroup)).toEqual([0, 1, 2, 3])
  })

  it.each([
    { cutGroup: first, previous: null, next: second },
    { cutGroup: second, previous: first, next: third },
    { cutGroup: third, previous: second, next: fourth },
    { cutGroup: fourth, previous: third, next: null },
    { cutGroup: -1, previous: null, next: first },
    { cutGroup: 0.5, previous: first, next: second },
    { cutGroup: 4, previous: fourth, next: null },
  ])("steps cut group $cutGroup", ({ cutGroup, previous, next }) => {
    expect(nextCutGroupPreset(cutGroup, "previous")).toBe(previous)
    expect(nextCutGroupPreset(cutGroup, "next")).toBe(next)
  })

  it.each([
    { cutGroup: second - 0.0001, previous: first, next: second },
    { cutGroup: second + 0.0001, previous: second, next: third },
  ])(
    "uses exact preset equality for cut group $cutGroup",
    ({ cutGroup, previous, next }) => {
      expect(nextCutGroupPreset(cutGroup, "previous")).toBe(previous)
      expect(nextCutGroupPreset(cutGroup, "next")).toBe(next)
    }
  )

  it.each([NaN, Infinity, -Infinity])(
    "returns null in both directions for non-finite cut group %s",
    (cutGroup) => {
      expect(nextCutGroupPreset(cutGroup, "previous")).toBeNull()
      expect(nextCutGroupPreset(cutGroup, "next")).toBeNull()
    }
  )
})
