import type { TimeSignature } from "@/bindings"

export const SIGNATURE_PRESETS = [
  { label: "4/4", numerator: 4, denominator: 4 },
  { label: "3/4", numerator: 3, denominator: 4 },
  { label: "2/4", numerator: 2, denominator: 4 },
  { label: "6/8", numerator: 6, denominator: 8 },
  { label: "5/4", numerator: 5, denominator: 4 },
  { label: "7/8", numerator: 7, denominator: 8 },
  { label: "12/8", numerator: 12, denominator: 8 },
] as const satisfies readonly (TimeSignature & { label: string })[]

export function nextSignature(
  current: TimeSignature,
  preset: TimeSignature
): TimeSignature | null {
  if (
    current.numerator === preset.numerator &&
    current.denominator === preset.denominator
  ) {
    return null
  }

  return { numerator: preset.numerator, denominator: preset.denominator }
}
