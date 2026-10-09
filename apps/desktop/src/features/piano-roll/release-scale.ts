import type { NoteExpression, NoteId } from "@/bindings"
import { DEFAULT_NOTE_EXPRESSION } from "@/lib/note-expression"

type ReleaseScaleFactor = "half" | "double"

type ReleaseNote = {
  id: NoteId
  expression?: NoteExpression
}

export function scaledRelease(
  release: number,
  factor: ReleaseScaleFactor
): number {
  return factor === "half" ? release / 2 : Math.min(release * 2, 1)
}

export function releaseScaleUpdates(
  notes: readonly ReleaseNote[],
  factor: ReleaseScaleFactor
): { id: NoteId; expression: NoteExpression }[] {
  return notes.flatMap(({ id, expression }) => {
    const current = expression ? expression.release : 0.5
    const next = scaledRelease(current, factor)
    return Math.abs(next - current) < 0.001
      ? []
      : [
          {
            id,
            expression: {
              ...(expression ?? DEFAULT_NOTE_EXPRESSION),
              release: next,
            },
          },
        ]
  })
}
