import type { FillRule } from "./advanced-fill"

export const FILL_RULES = ["regular", "euclidean", "random"] as const

export function nextFillRule(
  rule: string,
  direction: "previous" | "next"
): FillRule | null {
  const index = FILL_RULES.findIndex((item) => item === rule)
  if (index === -1) return null

  return FILL_RULES[index + (direction === "previous" ? -1 : 1)] ?? null
}
