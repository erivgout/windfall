import { describe, expect, it } from "vitest"

import type { RulerTool } from "./timeline-store"
import { nextRulerTool, RULER_TOOLS } from "./ruler-tool-step"

describe("ruler tool stepping", () => {
  it("starts with seek and ends with zoom in RULER_TOOLS", () => {
    expect(RULER_TOOLS[0]).toBe("seek")
    expect(RULER_TOOLS[RULER_TOOLS.length - 1]).toBe("zoom")
  })

  it.each<{
    tool: string
    direction: "previous" | "next"
    expected: RulerTool | null
  }>([
    { tool: "seek", direction: "previous", expected: null },
    { tool: "seek", direction: "next", expected: "select" },
    { tool: "select", direction: "previous", expected: "seek" },
    { tool: "select", direction: "next", expected: "zoom" },
    { tool: "zoom", direction: "previous", expected: "select" },
    { tool: "zoom", direction: "next", expected: null },
    { tool: "unknown", direction: "previous", expected: null },
    { tool: "unknown", direction: "next", expected: null },
  ])(
    "returns $expected for $tool moved $direction",
    ({ tool, direction, expected }) => {
      expect(nextRulerTool(tool, direction)).toBe(expected)
    }
  )
})
