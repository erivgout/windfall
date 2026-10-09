import { TOOLS, type Tool } from "./intents"

export function nextPlaylistTool(
  tool: string,
  direction: "previous" | "next"
): Tool | null {
  const index = TOOLS.findIndex((item) => item === tool)
  if (index === -1) return null

  return TOOLS[index + (direction === "previous" ? -1 : 1)] ?? null
}
