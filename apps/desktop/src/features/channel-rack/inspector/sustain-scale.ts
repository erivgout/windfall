import type { Envelope } from "@/bindings"
import { clamp } from "@/lib/units"

export function nextSustainScale(
  current: Envelope,
  factor: 0.5 | 2
): Envelope | null {
  const sustain = Math.round(clamp(current.sustain * factor, 0, 1) * 1000) / 1000

  // Allow for floating-point error at the thousandth boundary.
  if (Math.abs(sustain - current.sustain) <= 0.001 + Number.EPSILON) {
    return null
  }

  return {
    attackMs: current.attackMs,
    decayMs: current.decayMs,
    sustain,
    releaseMs: current.releaseMs,
  }
}
