import { PPQ, TICKS_PER_STEP } from "@/lib/units"

export const GATE_PRESETS = [
  { label: "Written", ticks: 0 },
  { label: "16th", ticks: TICKS_PER_STEP },
  { label: "8th", ticks: TICKS_PER_STEP * 2 },
  { label: "Quarter", ticks: PPQ },
] as const

export function nextGate(current: number, preset: number): number | null {
  return current === preset ? null : preset
}
