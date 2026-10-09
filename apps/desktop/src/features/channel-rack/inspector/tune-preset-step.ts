import { joinTune, splitTune } from "../steps"
import { TUNE_PRESETS } from "./tune-presets"

/** Steps the shown semitone through the presets while keeping the shown cents. */
export function nextSamplerTunePreset(
  tune: number,
  coarse: number,
  direction: "previous" | "next"
): { semitones: number; tune: number } | null {
  if (!Number.isFinite(tune)) return null

  const shown = splitTune(tune, coarse)
  const preset =
    direction === "previous"
      ? TUNE_PRESETS.findLast((item) => item.semitones < shown.semitones)
      : TUNE_PRESETS.find((item) => item.semitones > shown.semitones)
  if (!preset) return null

  const result = joinTune(preset.semitones, shown.cents, 48)
  if (Math.abs(result - tune) < 0.001) return null

  return { semitones: preset.semitones, tune: result }
}
