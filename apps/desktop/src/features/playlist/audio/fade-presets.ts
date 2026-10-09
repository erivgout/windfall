import type { AudioClipUpdate, ClipId } from "@/bindings"

export const FADE_PRESETS = [
  { label: "None", fraction: 0 },
  { label: "Short", fraction: 1 / 16 },
  { label: "Medium", fraction: 1 / 8 },
  { label: "Long", fraction: 1 / 4 },
] as const

type FadeClip = {
  id: ClipId
  length: number
  fadeIn: number
  fadeOut: number
}

export function fadePresetUpdates(
  clips: readonly FadeClip[],
  fraction: number
): AudioClipUpdate[] {
  return clips.flatMap(({ id, length, fadeIn, fadeOut }) => {
    const target = Math.max(0, Math.min(length, Math.round(length * fraction)))
    return fadeIn === target && fadeOut === target
      ? []
      : [{ id, patch: { fadeIn: target, fadeOut: target } }]
  })
}
