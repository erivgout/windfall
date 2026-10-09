import { PPQ } from "@/lib/units"

export function scaledShift(ticks: number, factor: "half" | "double"): number {
  return factor === "half"
    ? Math.trunc(ticks / 2)
    : Math.min(PPQ, Math.max(-PPQ, ticks * 2))
}

export function nextShiftScale(ticks: number, factor: "half" | "double"): number | null {
  const next = scaledShift(ticks, factor)
  return next === ticks ? null : next
}
