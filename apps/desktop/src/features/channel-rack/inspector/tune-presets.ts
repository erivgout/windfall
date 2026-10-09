import { joinTune } from "../steps"

export const TUNE_PRESETS = [
  { label: "Octave down", semitones: -12 },
  { label: "Fifth down", semitones: -7 },
  { label: "Unison", semitones: 0 },
  { label: "Fifth up", semitones: 7 },
  { label: "Octave up", semitones: 12 },
] as const

export function nextSamplerTune(
  current: number,
  cents: number,
  presetSemitones: number
): number | null {
  const joined = joinTune(presetSemitones, cents, 48)
  return Math.abs(joined - current) < 0.001 ? null : joined
}
