import type { Envelope } from "@/bindings"

import { ENVELOPE_PRESETS, nextEnvelope } from "./envelope-presets"

export function nextEnvelopePreset(
  current: Envelope,
  direction: "previous" | "next"
): Envelope | null {
  const index = ENVELOPE_PRESETS.findIndex(
    (preset) => nextEnvelope(current, preset.envelope) === null
  )
  if (index === -1) return null

  const step = direction === "previous" ? -1 : 1
  return ENVELOPE_PRESETS[index + step]?.envelope ?? null
}
