import { TICKS_PER_STEP } from "@/lib/units"

export const SHIFT_PRESETS = [
  { label: "16th early", ticks: -TICKS_PER_STEP },
  { label: "On grid", ticks: 0 },
  { label: "16th late", ticks: TICKS_PER_STEP },
  { label: "8th late", ticks: TICKS_PER_STEP * 2 },
] as const

export function nextShift(current: number, preset: number): number | null {
  return current === preset ? null : preset
}
