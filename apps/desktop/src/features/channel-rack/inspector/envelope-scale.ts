import type { Envelope } from "@/bindings"
import { DEFAULT_ENVELOPE_LIMITS } from "@/components/audio"
import { clamp } from "@/lib/units"

export function nextEnvelopeScale(
  current: Envelope,
  factor: 0.5 | 2
): Envelope | null {
  const time = (value: number, max: number) =>
    Math.round(clamp(value * factor, 0, max) * 10) / 10
  const next: Envelope = {
    attackMs: time(current.attackMs, DEFAULT_ENVELOPE_LIMITS.maxAttackMs),
    decayMs: time(current.decayMs, DEFAULT_ENVELOPE_LIMITS.maxDecayMs),
    sustain: current.sustain,
    releaseMs: time(current.releaseMs, DEFAULT_ENVELOPE_LIMITS.maxReleaseMs),
  }

  return next.attackMs !== current.attackMs ||
    next.decayMs !== current.decayMs ||
    next.releaseMs !== current.releaseMs
    ? next
    : null
}
