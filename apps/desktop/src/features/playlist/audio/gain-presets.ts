import type { AudioClipUpdate, ClipId } from "@/bindings"

export const GAIN_PRESETS = [
  { label: "Quiet", gain: 0.5 },
  { label: "Unity", gain: 1 },
  { label: "Loud", gain: 1.5 },
] as const

type GainClip = {
  id: ClipId
  gain: number
}

export function gainPresetUpdates(
  clips: readonly GainClip[],
  preset: number
): AudioClipUpdate[] {
  return clips.flatMap(({ id, gain }) =>
    Math.abs(gain - preset) < 0.001 ? [] : [{ id, patch: { gain: preset } }]
  )
}
