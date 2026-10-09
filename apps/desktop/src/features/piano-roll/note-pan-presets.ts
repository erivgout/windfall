import type { NoteId } from "@/bindings"

export const NOTE_PAN_PRESETS = [
  { label: "Hard left", pan: -1 },
  { label: "Left", pan: -0.5 },
  { label: "Center", pan: 0 },
  { label: "Right", pan: 0.5 },
  { label: "Hard right", pan: 1 },
] as const

type PanNote = {
  id: NoteId
  pan: number
}

export function notePanPresetUpdates(
  notes: readonly PanNote[],
  preset: number
): PanNote[] {
  return notes.flatMap(({ id, pan }) =>
    Math.abs(pan - preset) < 0.001 ? [] : [{ id, pan: preset }]
  )
}
