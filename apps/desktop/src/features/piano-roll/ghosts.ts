import type { Lane } from "@/bindings"
import {
  indexBatch,
  keyToRow,
  RectBatch,
  withAlpha,
  type GridTheme,
  type IndexedBatch,
} from "@/lib/canvas"

const GHOST_ALPHA = 0.3

/**
 * The other channels' notes in the same pattern as one faint batch, drawn
 * behind the notes being edited. Saved note identities allow editable-ghost
 * input to find their source lanes without duplicating project notes.
 */
export function buildGhostBatch(
  lanes: readonly Lane[],
  theme: GridTheme
): IndexedBatch | null {
  let count = 0
  for (const lane of lanes) count += lane.notes.length
  if (count === 0) return null
  const color = withAlpha(theme.mutedForeground, GHOST_ALPHA)
  const batch = new RectBatch(count)
  for (const lane of lanes) {
    for (const note of lane.notes) {
      batch.push(note.id, note.start, note.length, keyToRow(note.key), 1, color)
    }
  }
  return indexBatch(batch)
}
