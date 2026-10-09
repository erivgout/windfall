import type { AudioClipUpdate, ClipId } from "@/bindings"

import { PITCH_PRESETS } from "./pitch-presets"

type PitchClip = {
  id: ClipId
  pitch: number
}

export function nextClipPitchPreset(
  pitch: number,
  direction: "previous" | "next"
): number | null {
  if (!Number.isFinite(pitch)) return null

  const index = PITCH_PRESETS.findIndex(
    (item) => Math.abs(pitch - item.pitch) < 0.001
  )
  if (index !== -1) {
    const step = direction === "previous" ? -1 : 1
    return PITCH_PRESETS[index + step]?.pitch ?? null
  }

  let previous: number | null = null
  for (const item of PITCH_PRESETS) {
    if (item.pitch - pitch >= 0.001) {
      return direction === "previous" ? previous : item.pitch
    }
    if (pitch - item.pitch >= 0.001) previous = item.pitch
  }
  return direction === "previous" ? previous : null
}

export function clipPitchPresetStepUpdates(
  clips: readonly PitchClip[],
  direction: "previous" | "next"
): AudioClipUpdate[] {
  return clips.flatMap(({ id, pitch }) => {
    const next = nextClipPitchPreset(pitch, direction)
    return next === null ? [] : [{ id, patch: { pitch: next } }]
  })
}
