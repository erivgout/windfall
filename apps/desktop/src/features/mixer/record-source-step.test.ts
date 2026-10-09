import { describe, expect, it } from "vitest"

import { nextRecordingSource, RECORDING_SOURCES } from "./record-source-step"

describe("mixer recording source stepping", () => {
  it.each<{
    mode: string
    direction: "previous" | "next"
    expected: "input" | "postEffects" | "postFader" | null
  }>([
    { mode: "input", direction: "previous", expected: null },
    { mode: "input", direction: "next", expected: "postEffects" },
    { mode: "postEffects", direction: "previous", expected: "input" },
    { mode: "postEffects", direction: "next", expected: "postFader" },
    { mode: "postFader", direction: "previous", expected: "postEffects" },
    { mode: "postFader", direction: "next", expected: null },
    { mode: "unknown", direction: "previous", expected: null },
    { mode: "unknown", direction: "next", expected: null },
  ])(
    "returns $expected for $mode moved $direction",
    ({ mode, direction, expected }) => {
      expect(nextRecordingSource(mode, direction)).toBe(expected)
    }
  )

  it("orders sources from dry hardware input to after fader and pan", () => {
    expect(RECORDING_SOURCES[0]).toBe("input")
    expect(RECORDING_SOURCES[RECORDING_SOURCES.length - 1]).toBe("postFader")
  })
})
