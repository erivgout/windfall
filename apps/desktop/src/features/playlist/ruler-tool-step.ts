import type { RulerTool } from "./timeline-store"

export const RULER_TOOLS = ["seek", "select", "zoom"] as const

export function nextRulerTool(
  tool: string,
  direction: "previous" | "next"
): RulerTool | null {
  const index = RULER_TOOLS.findIndex((item) => item === tool)
  if (index === -1) return null

  return RULER_TOOLS[index + (direction === "previous" ? -1 : 1)] ?? null
}
