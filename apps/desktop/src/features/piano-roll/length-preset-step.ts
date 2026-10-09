import type { NoteId } from "@/bindings"
import { dispatch } from "@/lib/store/project"

import { LENGTH_PRESETS } from "./length-presets"
import { currentSession } from "./session"

type NoteLengthPresetDirection = "previous" | "next"

type LengthNote = {
  id: NoteId
  length: number
}

export function nextNoteLengthPreset(
  length: number,
  direction: NoteLengthPresetDirection
): number | null {
  if (!Number.isFinite(length)) return null

  let previous: number | null = null
  for (const [index, item] of LENGTH_PRESETS.entries()) {
    if (length === item.length) {
      return direction === "previous"
        ? previous
        : (LENGTH_PRESETS[index + 1]?.length ?? null)
    }
    if (length < item.length) {
      return direction === "previous" ? previous : item.length
    }
    previous = item.length
  }
  return direction === "previous" ? previous : null
}

export function noteLengthPresetStepUpdates(
  notes: readonly LengthNote[],
  direction: NoteLengthPresetDirection
): { id: NoteId; length: number }[] {
  return notes.flatMap(({ id, length }) => {
    const next = nextNoteLengthPreset(length, direction)
    return next === null ? [] : [{ id, length: next }]
  })
}

export async function setSelectedNoteLengthPresetStep(
  direction: NoteLengthPresetDirection
): Promise<void> {
  const session = currentSession()
  const context = session?.editor.context
  const notes = session?.editor.selectedNotes() ?? []
  if (!context || session?.editor.busy || !notes.length) return

  const updates = noteLengthPresetStepUpdates(notes, direction).map(
    ({ id, length }) => ({
      id,
      patch: { length },
    })
  )
  if (updates.length) {
    await dispatch({
      type: "updateCapturedNotes",
      pattern: context.pattern.id,
      channel: context.channel,
      expected: notes,
      updates,
    })
  }
}
