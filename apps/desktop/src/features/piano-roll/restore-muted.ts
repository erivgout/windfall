import type { Note, NoteId } from "@/bindings"

/** Restore only silent notes with a positive remembered velocity, in order. */
export function restoredVelocities(
  notes: readonly Note[],
  remembered: (id: NoteId) => number | undefined
): { id: NoteId; velocity: number }[] {
  return notes.flatMap((note) => {
    if (note.velocity !== 0) return []
    const velocity = remembered(note.id)
    return velocity !== undefined && velocity > 0
      ? [{ id: note.id, velocity }]
      : []
  })
}
