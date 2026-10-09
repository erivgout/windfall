import { joinTune, splitTune } from "../steps"

/** Scales the shown cents while keeping the shown semitone. */
export function nextSamplerFineScale(
  tune: number,
  coarse: number,
  factor: "half" | "double"
): number | null {
  const { semitones, cents } = splitTune(tune, coarse)
  const nextCents =
    factor === "half"
      ? Math.trunc(cents / 2)
      : Math.min(50, Math.max(-50, cents * 2))
  if (nextCents === cents) return null

  const result = joinTune(semitones, nextCents, 48)
  const shown = splitTune(result, semitones)
  if (
    shown.semitones !== semitones ||
    shown.cents !== nextCents ||
    Math.abs(result - tune) < 0.001
  ) {
    return null
  }
  return result
}
