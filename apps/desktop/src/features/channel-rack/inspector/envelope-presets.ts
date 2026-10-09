import type { Envelope } from "@/bindings"

export const ENVELOPE_PRESETS = [
  {
    label: "Pluck",
    envelope: { attackMs: 1, decayMs: 180, sustain: 0, releaseMs: 80 },
  },
  {
    label: "Keys",
    envelope: { attackMs: 8, decayMs: 400, sustain: 0.7, releaseMs: 250 },
  },
  {
    label: "Organ",
    envelope: { attackMs: 8, decayMs: 0, sustain: 1, releaseMs: 30 },
  },
  {
    label: "Pad",
    envelope: { attackMs: 500, decayMs: 300, sustain: 0.85, releaseMs: 800 },
  },
] as const satisfies readonly { label: string; envelope: Envelope }[]

export function nextEnvelope(
  current: Envelope,
  preset: Envelope
): Envelope | null {
  return current.attackMs === preset.attackMs &&
    current.decayMs === preset.decayMs &&
    current.releaseMs === preset.releaseMs &&
    Math.abs(current.sustain - preset.sustain) < 0.001
    ? null
    : preset
}
