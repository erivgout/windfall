import type { NoteExpression, NoteId } from "@/bindings"
import { DEFAULT_NOTE_EXPRESSION } from "@/lib/note-expression"

export const MODULATION_PRESETS = [
  { label: "Low", modulation: 0 },
  { label: "Center", modulation: 0.5 },
  { label: "High", modulation: 1 },
] as const

type ModulationNote = {
  id: NoteId
  expression?: NoteExpression
}

export function modulationPresetUpdates(
  notes: readonly ModulationNote[],
  axis: "modulationX" | "modulationY",
  preset: number
): { id: NoteId; expression: NoteExpression }[] {
  return notes.flatMap(({ id, expression }) => {
    const current = expression ? expression[axis] : 0.5
    return Math.abs(current - preset) < 0.001
      ? []
      : [
          {
            id,
            expression: {
              ...(expression ?? DEFAULT_NOTE_EXPRESSION),
              [axis]: preset,
            },
          },
        ]
  })
}
