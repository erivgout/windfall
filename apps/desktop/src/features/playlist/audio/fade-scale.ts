import type { AudioClipUpdate, ClipId } from "@/bindings"

type FadeScaleFactor = "half" | "double"

type FadeClip = {
  id: ClipId
  length: number
  fadeIn: number
  fadeOut: number
}

export function scaledFade(
  ticks: number,
  length: number,
  factor: FadeScaleFactor
): number {
  return factor === "half" ? Math.floor(ticks / 2) : Math.min(length, ticks * 2)
}

export function fadeScaleUpdates(
  clips: readonly FadeClip[],
  factor: FadeScaleFactor
): AudioClipUpdate[] {
  return clips.flatMap(({ id, length, fadeIn, fadeOut }) => {
    const nextIn = scaledFade(fadeIn, length, factor)
    const nextOut = scaledFade(fadeOut, length, factor)
    const patch: { fadeIn?: number; fadeOut?: number } = {}
    if (nextIn !== fadeIn) patch.fadeIn = nextIn
    if (nextOut !== fadeOut) patch.fadeOut = nextOut
    return Object.keys(patch).length === 0 ? [] : [{ id, patch }]
  })
}
