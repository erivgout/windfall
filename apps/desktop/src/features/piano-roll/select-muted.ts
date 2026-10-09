import type { Note } from "@/bindings"

/** Muted notes have velocity 0; preserve their order in the scene. */
export function mutedNoteIds(notes: readonly Note[]): number[] {
  return notes.filter((note) => note.velocity === 0).map((note) => note.id)
}
