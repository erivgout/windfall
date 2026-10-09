import type { NoteId } from "@/bindings"
import { dispatch } from "@/lib/store/project"
import { MAX_PATTERN_TICKS } from "@/lib/units"

import { currentSession } from "./session"

type NoteStartScaleFactor = "half" | "double"

type StartNote = {
  id: NoteId
  start: number
  length: number
}

export function scaledNoteStart(
  start: number,
  length: number,
  factor: NoteStartScaleFactor
): number {
  if (factor === "half") return Math.max(0, Math.floor(start / 2))

  const maxStart = MAX_PATTERN_TICKS - length
  if (maxStart < 0) return start
  return Math.min(start * 2, Math.max(0, maxStart))
}

export function noteStartScaleUpdates(
  notes: readonly StartNote[],
  factor: NoteStartScaleFactor
): { id: NoteId; start: number }[] {
  return notes.flatMap(({ id, start, length }) => {
    const next = scaledNoteStart(start, length, factor)
    return next === start ? [] : [{ id, start: next }]
  })
}

export async function setSelectedNoteStartScale(
  factor: NoteStartScaleFactor
): Promise<void> {
  const session = currentSession()
  const context = session?.editor.context
  const notes = session?.editor.selectedNotes() ?? []
  if (!context || session?.editor.busy || !notes.length) return

  const updates = noteStartScaleUpdates(notes, factor).map(({ id, start }) => ({
    id,
    patch: { start },
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
