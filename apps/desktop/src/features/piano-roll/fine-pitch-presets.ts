import type { NoteExpression, NoteId } from "@/bindings"
import { DEFAULT_NOTE_EXPRESSION } from "@/lib/note-expression"

export const FINE_PITCH_PRESETS = [
  { label: "Octave down", finePitchCents: -1200 },
  { label: "Semitone down", finePitchCents: -100 },
  { label: "In tune", finePitchCents: 0 },
  { label: "Semitone up", finePitchCents: 100 },
  { label: "Octave up", finePitchCents: 1200 },
] as const

type FinePitchNote = {
  id: NoteId
  expression?: NoteExpression
}

export function finePitchPresetUpdates(
  notes: readonly FinePitchNote[],
  preset: number
): { id: NoteId; expression: NoteExpression }[] {
  return notes.flatMap(({ id, expression }) => {
    const current = expression ? expression.finePitchCents : 0
    return Math.abs(current - preset) < 0.001
      ? []
      : [
          {
            id,
            expression: {
              ...(expression ?? DEFAULT_NOTE_EXPRESSION),
              finePitchCents: preset,
            },
          },
        ]
  })
}
