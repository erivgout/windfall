import type { Note } from "@/bindings"
import {
  buildNoteBatch,
  buildTimeIndex,
  rgba,
  type IndexedBatch,
  type Rgba,
} from "@/lib/canvas"

/**
 * The notes on screen and the batch drawn from them. `notes[i]` is the
 * note of rect `i`, so a hit on the canvas leads straight back to its note.
 */
export type Scene = {
  readonly notes: readonly Note[]
  readonly items: IndexedBatch
}

/** Used until the canvas has read the theme, and in tests. */
export const FALLBACK_PALETTE: readonly Rgba[] = [rgba(214, 51, 132)]

function isSortedByStart(notes: readonly Note[]): boolean {
  for (let i = 1; i < notes.length; i++) {
    if (notes[i].start < notes[i - 1].start) return false
  }
  return true
}

/**
 * Builds the scene from a lane's notes plus notes that are being drawn and
 * are not in the project yet. This is the one rebuild an edit costs.
 */
export function buildScene(
  lane: readonly Note[],
  provisional: readonly Note[],
  palette: readonly Rgba[],
  selected: ReadonlySet<number>
): Scene {
  let notes: readonly Note[] = lane
  if (provisional.length > 0) notes = [...lane, ...provisional]
  // The index needs the batch sorted by start. A lane already is; sorting
  // here instead of in the batch keeps the notes and the rects in step.
  if (!isSortedByStart(notes)) {
    notes = [...notes].sort((a, b) => a.start - b.start)
  }
  const batch = buildNoteBatch(notes, palette, { selected })
  return { notes, items: { batch, index: buildTimeIndex(batch) } }
}

/** The notes of a scene that are selected, in lane order. */
export function selectedNotes(
  scene: Scene,
  selection: ReadonlySet<number>
): Note[] {
  if (selection.size === 0) return []
  return scene.notes.filter((note) => selection.has(note.id))
}
