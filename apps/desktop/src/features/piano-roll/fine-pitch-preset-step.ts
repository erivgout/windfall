import { FINE_PITCH_PRESETS } from "./fine-pitch-presets"

export function nextNoteFinePitchPreset(
  cents: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(cents)) return null

  const index = FINE_PITCH_PRESETS.findIndex(
    (item) => Math.abs(cents - item.finePitchCents) < 0.001
  )
  if (index !== -1) {
    const step = direction === "previous" ? -1 : 1
    return FINE_PITCH_PRESETS[index + step]?.finePitchCents ?? null
  }

  let previous: number | null = null
  for (const item of FINE_PITCH_PRESETS) {
    if (item.finePitchCents > cents) {
      return direction === "previous" ? previous : item.finePitchCents
    }
    previous = item.finePitchCents
  }
  return direction === "previous" ? previous : null
}
