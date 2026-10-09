import type { NoteId } from "@/bindings"
import { dispatch } from "@/lib/store/project"
import { MAX_PATTERN_TICKS } from "@/lib/units"

import { currentSession } from "./session"

type LengthScaleFactor = "half" | "double"

type LengthNote = {
  id: NoteId
  length: number
}

export function scaledLength(length: number, factor: LengthScaleFactor): number {
  return factor === "half"
    ? Math.max(1, Math.floor(length / 2))
    : Math.min(MAX_PATTERN_TICKS, length * 2)
}

export function lengthScaleUpdates(
  notes: readonly LengthNote[],
  factor: LengthScaleFactor
): { id: NoteId; length: number }[] {
  return notes.flatMap(({ id, length }) => {
    const next = scaledLength(length, factor)
    return next === length ? [] : [{ id, length: next }]
  })
}

export async function setSelectedLengthScale(
  factor: LengthScaleFactor
): Promise<void> {
  const session = currentSession()
  const context = session?.editor.context
  const notes = session?.editor.selectedNotes() ?? []
  if (!context || session?.editor.busy || !notes.length) return

  const updates = lengthScaleUpdates(notes, factor).map(({ id, length }) => ({
    id,
    patch: { length },
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
