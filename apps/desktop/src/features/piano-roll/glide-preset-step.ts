import type { NoteExpression, NoteId } from "@/bindings"
import { DEFAULT_NOTE_EXPRESSION } from "@/lib/note-expression"
import { dispatch } from "@/lib/store/project"

import { GLIDE_PRESETS } from "./glide-presets"
import { currentSession } from "./session"

type GlidePresetDirection = "previous" | "next"

type GlideNote = {
  id: NoteId
  expression?: NoteExpression
}

export function nextGlidePreset(
  ticks: number,
  direction: GlidePresetDirection
): number | null {
  if (!Number.isFinite(ticks)) return null

  let previous: number | null = null
  for (const [index, item] of GLIDE_PRESETS.entries()) {
    if (ticks === item.glideTicks) {
      return direction === "previous"
        ? previous
        : (GLIDE_PRESETS[index + 1]?.glideTicks ?? null)
    }
    if (ticks < item.glideTicks) {
      return direction === "previous" ? previous : item.glideTicks
    }
    previous = item.glideTicks
  }
  return direction === "previous" ? previous : null
}

export function glidePresetStepUpdates(
  notes: readonly GlideNote[],
  direction: GlidePresetDirection
): { id: NoteId; expression: NoteExpression }[] {
  return notes.flatMap(({ id, expression }) => {
    const current =
      expression?.glideTicks ?? DEFAULT_NOTE_EXPRESSION.glideTicks!
    const next = nextGlidePreset(current, direction)
    return next === null
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

export async function setSelectedGlidePresetStep(
  direction: GlidePresetDirection
): Promise<void> {
  const session = currentSession()
  const context = session?.editor.context
  const notes = session?.editor.selectedNotes() ?? []
  if (!context || session?.editor.busy || !notes.length) return

  const updates = glidePresetStepUpdates(notes, direction).map(
    ({ id, expression }) => ({
      id,
      patch: { expression },
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
