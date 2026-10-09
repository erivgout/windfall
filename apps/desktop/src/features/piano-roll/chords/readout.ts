import type { Note } from "@/bindings"

import { detectChord } from "./detection"

/** Notes must belong to the open channel. Selection takes precedence over time. */
export function chordReadout(
  notes: readonly Pick<Note, "id" | "key" | "start" | "length">[],
  selection: ReadonlySet<number>,
  playhead: number | null
): string {
  const selected = notes.filter((note) => selection.has(note.id))
  const tick = typeof playhead === "number" ? Math.floor(playhead) : null
  const keys = (
    selected.length
      ? selected
      : tick === null
        ? []
        : notes.filter(
            (note) => note.start <= tick && tick < note.start + note.length
          )
  ).map((note) => note.key)
  return keys.length ? detectChord(keys).label : "—"
}
