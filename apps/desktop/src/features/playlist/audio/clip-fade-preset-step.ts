import type { AudioClipUpdate, ClipId } from "@/bindings"

import { FADE_PRESETS } from "./fade-presets"

type FadeClip = {
  id: ClipId
  length: number
  fadeIn: number
  fadeOut: number
}

export function nextClipFadePreset(
  length: number,
  fadeIn: number,
  fadeOut: number,
  direction: "previous" | "next"
): { fadeIn: number; fadeOut: number } | null {
  if (
    !Number.isFinite(length) ||
    length < 0 ||
    !Number.isFinite(fadeIn) ||
    !Number.isFinite(fadeOut) ||
    fadeIn !== fadeOut
  )
    return null

  const targets = FADE_PRESETS.map((item) =>
    Math.max(0, Math.min(length, Math.round(length * item.fraction)))
  )
  const index = targets.findIndex((target) => target === fadeIn)
  if (index === -1) return null

  const step = direction === "previous" ? -1 : 1
  for (
    let next = index + step;
    next >= 0 && next < targets.length;
    next += step
  ) {
    const target = targets[next]
    if (target !== fadeIn) return { fadeIn: target, fadeOut: target }
  }
  return null
}

export function clipFadePresetStepUpdates(
  clips: readonly FadeClip[],
  direction: "previous" | "next"
): AudioClipUpdate[] {
  return clips.flatMap(({ id, length, fadeIn, fadeOut }) => {
    const patch = nextClipFadePreset(length, fadeIn, fadeOut, direction)
    return patch === null ? [] : [{ id, patch }]
  })
}
