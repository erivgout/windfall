import { MAX_PATTERN_STEPS, TICKS_PER_STEP } from "@/lib/units"

const limit = MAX_PATTERN_STEPS * TICKS_PER_STEP

export function scaledWaveformStart(
  start: number,
  factor: "half" | "double"
): number {
  return factor === "half"
    ? Math.trunc(start / 2) || 0
    : Math.min(limit, Math.max(-limit, start * 2))
}

export function nextWaveformStartScale(
  start: number,
  factor: "half" | "double"
): number | null {
  const next = scaledWaveformStart(start, factor)
  return next === start ? null : next
}
