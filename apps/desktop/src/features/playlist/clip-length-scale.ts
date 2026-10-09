import type { Clip } from "@/bindings"
import { dispatch } from "@/lib/store/project"
import { MAX_SONG_TICKS } from "@/lib/units"

import { selectedClips } from "./selectors"

type ClipLengthScale = "half" | "double"

export function scaledClipLength(
  start: number,
  length: number,
  factor: ClipLengthScale
): number {
  if (factor === "half") return Math.max(1, Math.floor(length / 2))
  const remaining = MAX_SONG_TICKS - start
  if (remaining < 1) return length
  return Math.min(length * 2, remaining)
}

export function clipLengthScaleUpdates(
  clips: readonly Pick<Clip, "id" | "start" | "length">[],
  factor: ClipLengthScale
): Pick<Clip, "id" | "length">[] {
  return clips.flatMap(({ id, start, length }) => {
    const next = scaledClipLength(start, length, factor)
    return next === length ? [] : [{ id, length: next }]
  })
}

export async function setSelectedClipLengthScale(
  factor: ClipLengthScale
): Promise<void> {
  const clips = selectedClips()
  if (clips.length === 0) return
  const updates = clipLengthScaleUpdates(clips, factor)
  if (updates.length === 0) return
  await dispatch({
    type: "updateClips",
    updates: updates.map(({ id, length }) => ({ id, patch: { length } })),
  })
}
