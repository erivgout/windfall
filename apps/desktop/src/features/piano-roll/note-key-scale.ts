import type { NoteId } from "@/bindings"
import { dispatch } from "@/lib/store/project"
import { clamp, DEFAULT_KEY } from "@/lib/units"

import { currentSession } from "./session"

type NoteKeyScaleFactor = "half" | "double"

type KeyNote = {
  id: NoteId
  key: number
}

export function scaledNoteKey(key: number, factor: NoteKeyScaleFactor): number {
  const distance = key - DEFAULT_KEY
  if (factor === "half") return DEFAULT_KEY + Math.trunc(distance / 2)
  return clamp(DEFAULT_KEY + distance * 2, 0, 127)
}

export function noteKeyScaleUpdates(
  notes: readonly KeyNote[],
  factor: NoteKeyScaleFactor
): { id: NoteId; key: number }[] {
  return notes.flatMap(({ id, key }) => {
    const next = scaledNoteKey(key, factor)
    return next === key ? [] : [{ id, key: next }]
  })
}

export async function setSelectedNoteKeyScale(
  factor: NoteKeyScaleFactor
): Promise<void> {
  const session = currentSession()
  const context = session?.editor.context
  const notes = session?.editor.selectedNotes() ?? []
  if (!context || session?.editor.busy || !notes.length) return

  const updates = noteKeyScaleUpdates(notes, factor).map(({ id, key }) => ({
    id,
    patch: { key },
  }))
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
