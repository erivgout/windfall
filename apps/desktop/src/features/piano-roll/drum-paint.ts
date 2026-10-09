import type { Note, NoteInit } from "@/bindings"

export type DrumStroke = {
  decision: "add" | "delete" | null
  handled: Set<string>
  adds: NoteInit[]
  deletes: Set<number>
}

export function createDrumStroke(): DrumStroke {
  return { decision: null, handled: new Set(), adds: [], deletes: new Set() }
}

/** Occupancy is a note start within the cell, not a tail crossing it. */
export function visitDrumCell(
  stroke: DrumStroke,
  notes: readonly Note[],
  note: NoteInit
): void {
  const cell = `${note.start}:${note.key}`
  if (stroke.handled.has(cell)) return
  stroke.handled.add(cell)
  const occupants = notes.filter(
    (existing) =>
      existing.key === note.key &&
      existing.start >= note.start &&
      existing.start < note.start + note.length
  )
  stroke.decision ??= occupants.length ? "delete" : "add"
  if (stroke.decision === "delete") {
    for (const occupant of occupants) stroke.deletes.add(occupant.id)
  } else if (occupants.length === 0) {
    stroke.adds.push(note)
  }
}
