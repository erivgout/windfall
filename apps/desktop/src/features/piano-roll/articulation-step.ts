import type { NoteArticulation } from "@/bindings"
import { NOTE_ARTICULATIONS } from "@/lib/note-expression"

export function nextDrawArticulation(
  articulation: string,
  direction: "previous" | "next"
): NoteArticulation | null {
  const index = NOTE_ARTICULATIONS.findIndex((item) => item.value === articulation)
  if (index === -1) return null

  return (
    NOTE_ARTICULATIONS[index + (direction === "previous" ? -1 : 1)]?.value ?? null
  )
}
