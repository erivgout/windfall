import { describe, expect, it } from "vitest"

import { TOOLS, type Tool } from "./intents"
import { nextPlaylistTool } from "./playlist-tool-step"

describe("playlist tool stepping", () => {
  it("places paint between draw and select in TOOLS", () => {
    const index = TOOLS.indexOf("paint")

    expect(index).toBeGreaterThan(0)
    expect(TOOLS[index - 1]).toBe("draw")
    expect(TOOLS[index + 1]).toBe("select")
  })

  it.each<{
    tool: string
    direction: "previous" | "next"
    expected: Tool | null
  }>([
    { tool: "draw", direction: "previous", expected: null },
    { tool: "slice", direction: "next", expected: null },
    { tool: "paint", direction: "previous", expected: "draw" },
    { tool: "paint", direction: "next", expected: "select" },
    { tool: "select", direction: "previous", expected: "paint" },
    { tool: "select", direction: "next", expected: "erase" },
    { tool: "erase", direction: "previous", expected: "select" },
    { tool: "erase", direction: "next", expected: "mute" },
    { tool: "unknown", direction: "previous", expected: null },
    { tool: "unknown", direction: "next", expected: null },
  ])(
    "returns $expected for $tool moved $direction",
    ({ tool, direction, expected }) => {
      expect(nextPlaylistTool(tool, direction)).toBe(expected)
    }
  )

  it("stops at the first entry of TOOLS", () => {
    const first = TOOLS[0]!

    expect(nextPlaylistTool(first, "previous")).toBeNull()
    expect(nextPlaylistTool(first, "next")).toBe(TOOLS[1])
  })

  it("stops at the last entry of TOOLS", () => {
    const last = TOOLS[TOOLS.length - 1]!

    expect(nextPlaylistTool(last, "next")).toBeNull()
    expect(nextPlaylistTool(last, "previous")).toBe(TOOLS[TOOLS.length - 2])
  })
})
