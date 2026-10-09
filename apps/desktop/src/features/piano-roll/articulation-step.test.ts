import { describe, expect, it } from "vitest"

import type { NoteArticulation } from "@/bindings"
import { NOTE_ARTICULATIONS } from "@/lib/note-expression"

import { nextDrawArticulation } from "./articulation-step"

describe("piano-roll draw articulation stepping", () => {
  it("uses normal and portamento as the ends of NOTE_ARTICULATIONS", () => {
    expect(NOTE_ARTICULATIONS[0]?.value).toBe("normal")
    expect(NOTE_ARTICULATIONS[NOTE_ARTICULATIONS.length - 1]?.value).toBe(
      "portamento"
    )
  })

  it.each<{
    articulation: string
    direction: "previous" | "next"
    expected: NoteArticulation | null
  }>([
    { articulation: "normal", direction: "previous", expected: null },
    { articulation: "normal", direction: "next", expected: "slide" },
    { articulation: "slide", direction: "previous", expected: "normal" },
    { articulation: "slide", direction: "next", expected: "portamento" },
    { articulation: "portamento", direction: "previous", expected: "slide" },
    { articulation: "portamento", direction: "next", expected: null },
    { articulation: "unknown", direction: "previous", expected: null },
    { articulation: "unknown", direction: "next", expected: null },
  ])(
    "returns $expected for $articulation moved $direction",
    ({ articulation, direction, expected }) => {
      expect(nextDrawArticulation(articulation, direction)).toBe(expected)
    }
  )
})
