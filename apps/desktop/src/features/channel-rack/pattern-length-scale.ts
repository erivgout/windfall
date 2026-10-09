import { MAX_PATTERN_STEPS } from "@/lib/units"

export function scaledPatternLength(
  steps: number,
  factor: "half" | "double"
): number {
  return factor === "half"
    ? Math.max(1, Math.floor(steps / 2))
    : Math.min(MAX_PATTERN_STEPS, steps * 2)
}

export function nextPatternLengthScale(
  steps: number,
  factor: "half" | "double"
): number | null {
  const next = scaledPatternLength(steps, factor)
  return next === steps ? null : next
}
