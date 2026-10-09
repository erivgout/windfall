import type { AudioClipUpdate, ClipId } from "@/bindings"

export const PITCH_PRESETS = [
  { label: "Octave down", pitch: -12 },
  { label: "Fifth down", pitch: -7 },
  { label: "Unison", pitch: 0 },
  { label: "Fifth up", pitch: 7 },
  { label: "Octave up", pitch: 12 },
] as const

type PitchClip = {
  id: ClipId
  pitch: number
}

export function pitchPresetUpdates(
  clips: readonly PitchClip[],
  preset: number
): AudioClipUpdate[] {
  return clips.flatMap(({ id, pitch }) =>
    Math.abs(pitch - preset) < 0.001 ? [] : [{ id, patch: { pitch: preset } }]
  )
}
