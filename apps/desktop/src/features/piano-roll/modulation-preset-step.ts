import type { NoteExpression, NoteId } from "@/bindings"
import { DEFAULT_NOTE_EXPRESSION } from "@/lib/note-expression"

import { MODULATION_PRESETS } from "./modulation-presets"

type ModulationPresetDirection = "previous" | "next"

type ModulationNote = {
  id: NoteId
  expression?: NoteExpression
}

const PRESET_TOLERANCE = 0.001

export function nextNoteModulationPreset(
  value: number,
  direction: ModulationPresetDirection
): number | null {
  if (!Number.isFinite(value)) return null

  const index = MODULATION_PRESETS.findIndex(
    (item) => Math.abs(value - item.modulation) < PRESET_TOLERANCE
  )
  if (index !== -1) {
    const nextIndex = index + (direction === "previous" ? -1 : 1)
    return MODULATION_PRESETS[nextIndex]?.modulation ?? null
  }

  const preset =
    direction === "previous"
      ? [...MODULATION_PRESETS]
          .reverse()
          .find((item) => value - item.modulation >= PRESET_TOLERANCE)
      : MODULATION_PRESETS.find(
          (item) => item.modulation - value >= PRESET_TOLERANCE
        )
  return preset?.modulation ?? null
}

export function modulationPresetStepUpdates(
  notes: readonly ModulationNote[],
  axis: "modulationX" | "modulationY",
  direction: ModulationPresetDirection
): { id: NoteId; expression: NoteExpression }[] {
  return notes.flatMap(({ id, expression }) => {
    const current = expression ? expression[axis] : 0.5
    const next = nextNoteModulationPreset(current, direction)
    return next === null
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
