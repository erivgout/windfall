import { mix, rgbFromInt, type Rgba } from "./color"
import { isNoteColorGroup, NOTE_COLOR_GROUPS } from "@/lib/note-colors"
import { RECT_SELECTED, RectBatch } from "./rect-batch"
import { levelColor, type GridTheme } from "./theme"
import { keyToRow } from "./viewport"

/** The fields of the project model's `Note` that drawing needs. */
export interface NoteLike {
  readonly id: number
  readonly start: number
  readonly length: number
  readonly key: number
  readonly velocity: number
  readonly expression?: { readonly colorGroup?: number | null }
}

/**
 * Velocity is quantized to this many brightness steps. More would not be
 * visible, and a small palette keeps the Canvas 2D renderer fast.
 */
export const VELOCITY_LEVELS = 32

/** Note colors from quietest to loudest for one base color. */
export function velocityPalette(
  theme: GridTheme,
  base: Rgba = theme.item
): Rgba[] {
  const palette: Rgba[] = []
  for (let level = 0; level < VELOCITY_LEVELS; level++) {
    palette.push(levelColor(theme, base, level / (VELOCITY_LEVELS - 1)))
  }
  return palette
}

export interface NoteBatchOptions {
  readonly rowCount?: number
  /** Ids of notes to mark selected. */
  readonly selected?: ReadonlySet<number>
}

/**
 * Builds the rect batch for a pattern's notes. The palette holds the
 * theme's colors, so build again when the theme changes.
 */
export function buildNoteBatch(
  notes: readonly NoteLike[],
  palette: readonly Rgba[],
  options: NoteBatchOptions = {}
): RectBatch {
  const rowCount = options.rowCount ?? 128
  const selected = options.selected
  const top = palette.length - 1
  const batch = new RectBatch(Math.max(16, notes.length))
  for (const note of notes) {
    const level = Math.round(Math.min(1, Math.max(0, note.velocity)) * top)
    const group = note.expression?.colorGroup
    const color = isNoteColorGroup(group)
      ? mix(palette[0], rgbFromInt(NOTE_COLOR_GROUPS[group]), 0.35 + 0.65 * Math.min(1, Math.max(0, note.velocity)))
      : palette[level]
    batch.push(
      note.id,
      note.start,
      note.length,
      keyToRow(note.key, rowCount),
      1,
      color,
      selected?.has(note.id) ? RECT_SELECTED : 0
    )
  }
  return batch
}
