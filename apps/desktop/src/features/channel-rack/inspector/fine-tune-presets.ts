import { joinTune, splitTune } from "../steps"

export const FINE_TUNE_PRESETS = [
  { label: "−50", cents: -50 },
  { label: "−25", cents: -25 },
  { label: "In tune", cents: 0 },
  { label: "+25", cents: 25 },
  { label: "+50", cents: 50 },
] as const

export function nextSamplerFine(
  tune: number,
  coarse: number,
  centsPreset: number
): number | null {
  const { semitones, cents } = splitTune(tune, coarse)
  if (cents === centsPreset) return null

  const result = joinTune(semitones, centsPreset, 48)
  const shown = splitTune(result, semitones)
  if (
    shown.semitones !== semitones ||
    shown.cents !== centsPreset ||
    Math.abs(result - tune) < 0.001
  ) {
    return null
  }
  return result
}
