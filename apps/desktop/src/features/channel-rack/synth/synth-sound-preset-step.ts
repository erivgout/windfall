import {
  matchingPreset,
  SYNTH_PRESETS,
  type SynthPreset,
  type SynthSettings,
} from "./presets"

export function nextSynthSoundPreset(
  params: SynthSettings,
  direction: "previous" | "next"
): SynthPreset | null {
  const current = matchingPreset(params)
  if (!current) return null

  const index = SYNTH_PRESETS.indexOf(current)
  const step = direction === "previous" ? -1 : 1
  return SYNTH_PRESETS[index + step] ?? null
}
