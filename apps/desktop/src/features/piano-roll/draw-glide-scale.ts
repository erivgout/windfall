import { MAX_PATTERN_TICKS } from "@/lib/units"

export function scaledDrawGlide(ticks: number, factor: "half" | "double"): number {
  return factor === "half"
    ? Math.max(1, Math.floor(ticks / 2))
    : Math.min(MAX_PATTERN_TICKS, ticks * 2)
}

export function nextDrawGlideScale(
  ticks: number,
  factor: "half" | "double"
): number | null {
  const next = scaledDrawGlide(ticks, factor)
  return next === ticks ? null : next
}
