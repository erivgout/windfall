import type { NoteExpression, NoteId } from "@/bindings"
import { DEFAULT_NOTE_EXPRESSION } from "@/lib/note-expression"
import { dispatch } from "@/lib/store/project"
import { MAX_PATTERN_TICKS } from "@/lib/units"

import { currentSession } from "./session"

type GlideScaleFactor = "half" | "double"

type GlideNote = {
  id: NoteId
  expression?: NoteExpression
}

export function scaledGlide(ticks: number, factor: GlideScaleFactor): number {
  return factor === "half"
    ? Math.max(1, Math.floor(ticks / 2))
    : Math.min(MAX_PATTERN_TICKS, ticks * 2)
}

export function glideScaleUpdates(
  notes: readonly GlideNote[],
  factor: GlideScaleFactor
): { id: NoteId; expression: NoteExpression }[] {
  return notes.flatMap(({ id, expression }) => {
    const current = expression?.glideTicks ?? 240
    const next = scaledGlide(current, factor)
    return next === current
      ? []
      : [
          {
            id,
            expression: {
              ...DEFAULT_NOTE_EXPRESSION,
              ...expression,
              glideTicks: next,
            },
          },
        ]
  })
}

export async function setSelectedGlideScale(
  factor: GlideScaleFactor
): Promise<void> {
  const session = currentSession()
  const context = session?.editor.context
  const notes = session?.editor.selectedNotes() ?? []
  if (!context || session?.editor.busy || !notes.length) return

  const updates = glideScaleUpdates(notes, factor).map(({ id, expression }) => ({
    id,
    patch: { expression },
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
