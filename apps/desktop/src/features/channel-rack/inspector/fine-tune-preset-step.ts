import { joinTune, splitTune } from "../steps"
import { FINE_TUNE_PRESETS } from "./fine-tune-presets"

/** Steps the fine-tune presets while keeping the semitone on show. */
export function nextSamplerFinePreset(
  tune: number,
  coarse: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(tune)) return null

  const split = splitTune(tune, coarse)
  // Outside the fine range, splitTune recenters on another semitone.
  // Express its cents around the semitone on show before choosing a preset.
  const cents = split.cents + (split.semitones - coarse) * 100
  const preset =
    direction === "previous"
      ? FINE_TUNE_PRESETS.findLast((item) => item.cents < cents)
      : FINE_TUNE_PRESETS.find((item) => item.cents > cents)
  if (!preset) return null

  const result = joinTune(coarse, preset.cents, 48)
  const shown = splitTune(result, coarse)
  if (
    shown.semitones !== coarse ||
    shown.cents !== preset.cents ||
    Math.abs(result - tune) < 0.001
  ) {
    return null
  }
  return result
}
