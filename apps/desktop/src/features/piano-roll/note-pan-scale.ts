import type { NoteId } from "@/bindings"

type NotePanScaleFactor = "half" | "double"

type PanNote = {
  id: NoteId
  pan: number
}

export function scaledNotePan(pan: number, factor: NotePanScaleFactor): number {
  return factor === "half" ? pan / 2 : Math.min(1, Math.max(-1, pan * 2))
}

export function notePanScaleUpdates(
  notes: readonly PanNote[],
  factor: NotePanScaleFactor
): PanNote[] {
  return notes.flatMap(({ id, pan }) => {
    const next = scaledNotePan(pan, factor)
    return Math.abs(next - pan) < 0.001 ? [] : [{ id, pan: next }]
  })
}
