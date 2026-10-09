import { TICKS_PER_STEP } from "@/lib/units"

/** The first or last playable tick of a pattern. */
export function patternEdgeTick(
  lengthSteps: number,
  edge: "start" | "end"
): number {
  if (edge === "start" || lengthSteps < 1) return 0
  return lengthSteps * TICKS_PER_STEP - 1
}
