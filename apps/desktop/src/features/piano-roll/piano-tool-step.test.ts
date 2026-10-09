import { describe, expect, it } from "vitest"

import { nextPianoTool } from "./piano-tool-step"
import { TOOLS, type Tool } from "./store"

describe("piano-roll tool stepping", () => {
  it("uses draw and playback as the ends of TOOLS", () => {
    expect(TOOLS[0]).toBe("draw")
    expect(TOOLS[TOOLS.length - 1]).toBe("playback")
  })

  it.each<{
    tool: string
    direction: "previous" | "next"
    expected: Tool | null
  }>([
    { tool: "draw", direction: "previous", expected: null },
    { tool: "draw", direction: "next", expected: "paint" },
    { tool: "paint", direction: "previous", expected: "draw" },
    { tool: "paint", direction: "next", expected: "select" },
    { tool: "select", direction: "previous", expected: "paint" },
    { tool: "select", direction: "next", expected: "erase" },
    { tool: "erase", direction: "previous", expected: "select" },
    { tool: "erase", direction: "next", expected: "mute" },
    { tool: "mute", direction: "previous", expected: "erase" },
    { tool: "mute", direction: "next", expected: "slice" },
    { tool: "slice", direction: "previous", expected: "mute" },
    { tool: "slice", direction: "next", expected: "zoom" },
    { tool: "zoom", direction: "previous", expected: "slice" },
    { tool: "zoom", direction: "next", expected: "playback" },
    { tool: "playback", direction: "previous", expected: "zoom" },
    { tool: "playback", direction: "next", expected: null },
    { tool: "unknown", direction: "previous", expected: null },
    { tool: "unknown", direction: "next", expected: null },
  ])(
    "returns $expected for $tool moved $direction",
    ({ tool, direction, expected }) => {
      expect(nextPianoTool(tool, direction)).toBe(expected)
    }
  )
})
