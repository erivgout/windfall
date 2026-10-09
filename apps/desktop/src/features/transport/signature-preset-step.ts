import type { TimeSignature } from "@/bindings"

import { SIGNATURE_PRESETS } from "./signature-presets"

export function nextSignaturePreset(
  signature: TimeSignature,
  direction: "previous" | "next"
): TimeSignature | null {
  const index = SIGNATURE_PRESETS.findIndex(
    (preset) =>
      preset.numerator === signature.numerator &&
      preset.denominator === signature.denominator
  )
  if (index === -1) return null

  const step = direction === "previous" ? -1 : 1
  const preset = SIGNATURE_PRESETS[index + step]
  if (!preset) return null

  return { numerator: preset.numerator, denominator: preset.denominator }
}
