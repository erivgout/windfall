import { describe, expect, it } from "vitest"

import { nextScale } from "./scale-step"
import { SCALES, type ScaleId } from "./scales"

describe("piano-roll musical scale stepping", () => {
  it.each<{
    id: ScaleId
    direction: "previous" | "next"
    expected: ScaleId | null
  }>([
    { id: "major", direction: "previous", expected: null },
    { id: "major", direction: "next", expected: "minor" },
    { id: "minor", direction: "previous", expected: "major" },
    { id: "minor", direction: "next", expected: "harmonic-minor" },
    { id: "chromatic", direction: "next", expected: null },
    { id: "chromatic", direction: "previous", expected: "whole-tone" },
    { id: "blues", direction: "previous", expected: "minor-pentatonic" },
    { id: "blues", direction: "next", expected: "whole-tone" },
  ])(
    "returns $expected for $id moved $direction",
    ({ id, direction, expected }) => {
      expect(nextScale(id, direction)).toBe(expected)
    }
  )

  it("uses major and chromatic as the ends of SCALES", () => {
    expect(SCALES[0].id).toBe("major")
    expect(SCALES[SCALES.length - 1].id).toBe("chromatic")
  })

  it("reads minor's neighbors from SCALES", () => {
    const index = SCALES.findIndex((scale) => scale.id === "minor")
    expect(SCALES[index - 1].id).toBe("major")
    expect(SCALES[index + 1].id).toBe("harmonic-minor")
    expect(nextScale("minor", "previous")).toBe(SCALES[index - 1].id)
    expect(nextScale("minor", "next")).toBe(SCALES[index + 1].id)
  })

  it.each(["", "unknown-scale", null, undefined, 42])(
    "does not step invalid scale %s",
    (id) => {
      expect(nextScale(id, "previous")).toBeNull()
      expect(nextScale(id, "next")).toBeNull()
    }
  )
})
