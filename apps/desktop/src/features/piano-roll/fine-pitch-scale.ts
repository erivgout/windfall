import type { NoteExpression, NoteId } from "@/bindings"
import { DEFAULT_NOTE_EXPRESSION } from "@/lib/note-expression"

type FinePitchScaleFactor = "half" | "double"

type FinePitchNote = {
  id: NoteId
  expression?: NoteExpression
}

export function scaledFinePitch(
  cents: number,
  factor: FinePitchScaleFactor
): number {
  return factor === "half"
    ? cents / 2
    : Math.min(1200, Math.max(-1200, cents * 2))
}

export function finePitchScaleUpdates(
  notes: readonly FinePitchNote[],
  factor: FinePitchScaleFactor
): { id: NoteId; expression: NoteExpression }[] {
  return notes.flatMap(({ id, expression }) => {
    const current = expression ? expression.finePitchCents : 0
    const next = scaledFinePitch(current, factor)
    return Math.abs(next - current) < 0.001
      ? []
      : [
          {
            id,
            expression: {
              ...(expression ?? DEFAULT_NOTE_EXPRESSION),
              finePitchCents: next,
            },
          },
        ]
  })
}
