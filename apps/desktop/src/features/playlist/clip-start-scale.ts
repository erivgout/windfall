import type { Clip } from "@/bindings"
import { dispatch } from "@/lib/store/project"
import { MAX_SONG_TICKS } from "@/lib/units"

import { selectedClips } from "./selectors"

type ClipStartScale = "half" | "double"

export function scaledClipStart(
  start: number,
  length: number,
  factor: ClipStartScale
): number {
  if (factor === "half") return Math.max(0, Math.floor(start / 2))
  const remaining = MAX_SONG_TICKS - length
  if (remaining < 0) return start
  return Math.min(start * 2, Math.max(0, remaining))
}

export function clipStartScaleUpdates(
  clips: readonly Pick<Clip, "id" | "start" | "length">[],
  factor: ClipStartScale
): Pick<Clip, "id" | "start">[] {
  return clips.flatMap(({ id, start, length }) => {
    const next = scaledClipStart(start, length, factor)
    return next === start ? [] : [{ id, start: next }]
  })
}

export async function setSelectedClipStartScale(
  factor: ClipStartScale
): Promise<void> {
  const clips = selectedClips()
  if (clips.length === 0) return
  const updates = clipStartScaleUpdates(clips, factor)
  if (updates.length === 0) return
  await dispatch({
    type: "updateClips",
    updates: updates.map(({ id, start }) => ({ id, patch: { start } })),
  })
}
