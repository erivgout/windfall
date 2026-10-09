import type { NoteExpression, NoteId } from "@/bindings"
import { DEFAULT_NOTE_EXPRESSION } from "@/lib/note-expression"

type ModulationScaleFactor = "half" | "double"

type ModulationNote = {
  id: NoteId
  expression?: NoteExpression
}

export function modulationScaleUpdates(
  notes: readonly ModulationNote[],
  axis: "modulationX" | "modulationY",
  factor: ModulationScaleFactor
): { id: NoteId; expression: NoteExpression }[] {
  return notes.flatMap(({ id, expression }) => {
    const current = expression ? expression[axis] : 0.5
    const next =
      factor === "half"
        ? 0.5 + (current - 0.5) / 2
        : Math.min(1, Math.max(0, 0.5 + (current - 0.5) * 2))
    return Math.abs(next - current) < 0.001
      ? []
      : [
          {
            id,
            expression: {
              ...(expression ?? DEFAULT_NOTE_EXPRESSION),
              [axis]: next,
            },
          },
        ]
  })
}
