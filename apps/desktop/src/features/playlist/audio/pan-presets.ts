import type { AudioClipUpdate, ClipId } from "@/bindings"

export const PAN_PRESETS = [
  { label: "Hard left", pan: -1 },
  { label: "Left", pan: -0.5 },
  { label: "Center", pan: 0 },
  { label: "Right", pan: 0.5 },
  { label: "Hard right", pan: 1 },
] as const

type PanClip = {
  id: ClipId
  pan: number
}

export function panPresetUpdates(
  clips: readonly PanClip[],
  preset: number
): AudioClipUpdate[] {
  return clips.flatMap(({ id, pan }) =>
    Math.abs(pan - preset) < 0.001 ? [] : [{ id, patch: { pan: preset } }]
  )
}
