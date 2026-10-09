import type { NoteExpression, NoteId } from "@/bindings"
import { DEFAULT_NOTE_EXPRESSION } from "@/lib/note-expression"

export const RELEASE_PRESETS = [
  { label: "Short", release: 0 },
  { label: "Natural", release: 0.5 },
  { label: "Long", release: 1 },
] as const

type ReleaseNote = {
  id: NoteId
  expression?: NoteExpression
}

export function releasePresetUpdates(
  notes: readonly ReleaseNote[],
  preset: number
): { id: NoteId; expression: NoteExpression }[] {
  return notes.flatMap(({ id, expression }) => {
    const current = expression ? expression.release : 0.5
    return Math.abs(current - preset) < 0.001
      ? []
      : [
          {
            id,
            expression: {
              ...(expression ?? DEFAULT_NOTE_EXPRESSION),
              release: preset,
            },
          },
        ]
  })
}
