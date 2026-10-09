import type { NoteId } from "@/bindings"
import { dispatch } from "@/lib/store/project"
import { MAX_PATTERN_TICKS } from "@/lib/units"

import { currentSession } from "./session"

type NoteFromEndScaleFactor = "half" | "double"

type FromEndNote = {
  id: NoteId
  start: number
  length: number
}

export function scaledNoteFromEnd(
  start: number,
  length: number,
  factor: NoteFromEndScaleFactor
): { start: number; length: number } | null {
  const end = start + length
  if (end > MAX_PATTERN_TICKS) return null

  const nextLength =
    factor === "half"
      ? Math.max(1, Math.floor(length / 2))
      : Math.min(length * 2, end)
  const nextStart = end - nextLength
  if (nextStart === start && nextLength === length) return null
  return { start: nextStart, length: nextLength }
}

export function noteFromEndScaleUpdates(
  notes: readonly FromEndNote[],
  factor: NoteFromEndScaleFactor
): { id: NoteId; start: number; length: number }[] {
  return notes.flatMap(({ id, start, length }) => {
    const next = scaledNoteFromEnd(start, length, factor)
    return next === null ? [] : [{ id, ...next }]
  })
}

export async function setSelectedNoteFromEndScale(
  factor: NoteFromEndScaleFactor
): Promise<void> {
  const session = currentSession()
  const context = session?.editor.context
  const notes = session?.editor.selectedNotes() ?? []
  if (!context || session?.editor.busy || !notes.length) return

  const updates = noteFromEndScaleUpdates(notes, factor).map(
    ({ id, start, length }) => ({ id, patch: { start, length } })
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
