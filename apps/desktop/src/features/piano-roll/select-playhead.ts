import type { Note } from "@/bindings"

/** Notes include their start tick and exclude their end; keep the given order. */
export function notesAtTick(notes: readonly Note[], tick: number): number[] {
  return notes
    .filter((note) => note.start <= tick && tick < note.start + note.length)
    .map((note) => note.id)
}
