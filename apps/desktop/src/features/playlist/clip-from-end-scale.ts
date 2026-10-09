import type { Clip } from "@/bindings"
import { dispatch } from "@/lib/store/project"
import { MAX_SONG_TICKS } from "@/lib/units"

import { selectedClips } from "./selectors"

type ClipFromEndScale = "half" | "double"

export function scaledClipFromEnd(
  start: number,
  length: number,
  factor: ClipFromEndScale
): { start: number; length: number } | null {
  const end = start + length
  if (end > MAX_SONG_TICKS) return null
  const nextLength =
    factor === "half"
      ? Math.max(1, Math.floor(length / 2))
      : Math.min(length * 2, end)
  const nextStart = end - nextLength
  if (nextStart === start && nextLength === length) return null
  return { start: nextStart, length: nextLength }
}

export function clipFromEndScaleUpdates(
  clips: readonly Pick<Clip, "id" | "start" | "length">[],
  factor: ClipFromEndScale
): Pick<Clip, "id" | "start" | "length">[] {
  return clips.flatMap(({ id, start, length }) => {
    const next = scaledClipFromEnd(start, length, factor)
    return next === null ? [] : [{ id, ...next }]
  })
}

export async function setSelectedClipFromEndScale(
  factor: ClipFromEndScale
): Promise<void> {
  const clips = selectedClips()
  if (clips.length === 0) return
  const updates = clipFromEndScaleUpdates(clips, factor)
  if (updates.length === 0) return
  await dispatch({
    type: "updateClips",
    updates: updates.map(({ id, start, length }) => ({
      id,
      patch: { start, length },
    })),
  })
}
