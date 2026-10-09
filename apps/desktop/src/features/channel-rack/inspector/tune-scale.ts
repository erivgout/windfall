import { joinTune, splitTune } from "../steps"

/** Scales the shown semitone while keeping the shown cents. */
export function nextSamplerTuneScale(
  tune: number,
  coarse: number,
  factor: "half" | "double"
): { tune: number; semitones: number } | null {
  const shown = splitTune(tune, coarse)
  const scaledSemitones =
    factor === "half"
      ? Math.trunc(shown.semitones / 2)
      : Math.min(48, Math.max(-48, shown.semitones * 2))
  if (scaledSemitones === shown.semitones) return null

  const result = joinTune(scaledSemitones, shown.cents, 48)
  const after = splitTune(result, scaledSemitones)
  if (
    after.semitones !== scaledSemitones ||
    after.cents !== shown.cents ||
    Math.abs(result - tune) < 0.001
  ) {
    return null
  }
  return { tune: result, semitones: scaledSemitones }
}
