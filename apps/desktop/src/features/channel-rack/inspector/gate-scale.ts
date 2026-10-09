import { MAX_PATTERN_STEPS, TICKS_PER_STEP } from "@/lib/units"

export function scaledGate(ticks: number, factor: "half" | "double"): number {
  return factor === "half"
    ? Math.floor(ticks / 2)
    : Math.min(MAX_PATTERN_STEPS * TICKS_PER_STEP, ticks * 2)
}

export function nextGateScale(ticks: number, factor: "half" | "double"): number | null {
  const next = scaledGate(ticks, factor)
  return next === ticks ? null : next
}
