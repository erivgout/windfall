import { describe, expect, it } from "vitest"

import { isInScale, type ScaleHighlight } from "./model"

const members = (choice: ScaleHighlight) =>
  Array.from({ length: 12 }, (_, pitch) => pitch).filter((pitch) =>
    isInScale(pitch, choice)
  )

describe("scale highlight membership", () => {
  it("includes C D E F G A B in C major, excluding C#", () => {
    expect(members({ root: 0, scale: "major" })).toEqual([0, 2, 4, 5, 7, 9, 11])
    expect(isInScale(61, { root: 0, scale: "major" })).toBe(false)
    expect(isInScale(60, { root: 0, scale: "major" })).toBe(true)
  })

  it("includes A B C D E F G in A natural minor", () => {
    expect(members({ root: 9, scale: "natural-minor" })).toEqual([
      0, 2, 4, 5, 7, 9, 11,
    ])
  })

  it("includes every pitch class when Off, for every root", () => {
    for (let root = 0; root < 12; root++) {
      expect(members({ root, scale: "off" })).toEqual(
        Array.from({ length: 12 }, (_, pitch) => pitch)
      )
    }
  })

  it("has five distinct pitch classes in pentatonic major for every root", () => {
    for (let root = 0; root < 12; root++) {
      expect(members({ root, scale: "pentatonic-major" })).toHaveLength(5)
    }
    expect(members({ root: 0, scale: "pentatonic-major" })).toEqual([
      0, 2, 4, 7, 9,
    ])
  })

  it("raises the seventh in harmonic minor and repeats across octaves", () => {
    expect(members({ root: 9, scale: "harmonic-minor" })).toEqual([
      0, 2, 4, 5, 8, 9, 11,
    ])
    for (let key = 0; key < 116; key++) {
      expect(isInScale(key + 12, { root: 9, scale: "harmonic-minor" })).toBe(
        isInScale(key, { root: 9, scale: "harmonic-minor" })
      )
    }
  })
})
