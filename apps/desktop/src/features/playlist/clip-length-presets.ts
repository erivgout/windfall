import type { Clip, TimeSignature } from "@/bindings"
import { dispatch, useProjectStore } from "@/lib/store/project"
import { ticksPerBar } from "@/lib/time"

import { selectedClips } from "./selectors"

export const CLIP_LENGTH_BARS = [1, 2, 4, 8] as const

export function clipLengthTicks(
  signature: TimeSignature,
  bars: number
): number {
  return ticksPerBar(signature) * bars
}

export function clipLengthUpdates(
  clips: readonly Pick<Clip, "id" | "length">[],
  ticks: number
): Pick<Clip, "id" | "length">[] {
  return clips.flatMap(({ id, length }) =>
    length === ticks ? [] : [{ id, length: ticks }]
  )
}

export async function setSelectedClipLength(bars: number): Promise<void> {
  const clips = selectedClips()
  if (clips.length === 0) return
  const signature = useProjectStore.getState().project.settings.timeSignature
  const updates = clipLengthUpdates(clips, clipLengthTicks(signature, bars))
  if (updates.length === 0) return
  await dispatch({
    type: "updateClips",
    updates: updates.map(({ id, length }) => ({ id, patch: { length } })),
  })
}
