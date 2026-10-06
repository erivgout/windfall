import type { Note } from "@/bindings/Note"
import {
  buildNoteBatch,
  indexBatch,
  velocityPalette,
  type IndexedBatch,
  type Rgba,
  type TimeGridView,
} from "@/lib/canvas"

export const ROW_COUNT = 128

/**
 * Stands in for the project store: it owns the notes and turns them into
 * the batch the view draws. The real piano roll will do the same from the
 * pattern in the store.
 */
export class BenchModel {
  notes: Note[]
  items: IndexedBatch

  private readonly view: TimeGridView
  private palette: Rgba[]

  constructor(view: TimeGridView, notes: Note[]) {
    this.view = view
    this.notes = notes
    this.palette = velocityPalette(view.theme)
    this.items = this.build(undefined)
    view.setItems(this.items)
    view.onThemeChange((theme) => {
      this.palette = velocityPalette(theme)
      this.rebuild()
    })
  }

  /**
   * Rebuilds the batch and its index from the notes and hands it to the
   * view, which uploads it. This is the full cost of an edit that arrives
   * as "the notes changed".
   */
  rebuild(selected?: ReadonlySet<number>): void {
    this.items = this.build(selected ?? this.selectedIds())
    this.view.setItems(this.items)
  }

  selectedIds(): Set<number> {
    const batch = this.items.batch
    const ids = new Set<number>()
    for (const index of batch.selectedIndices()) ids.add(batch.ids[index])
    return ids
  }

  /** Applies a finished drag to the selected notes. */
  moveSelected(deltaTicks: number, deltaRows: number): void {
    if (deltaTicks === 0 && deltaRows === 0) return
    const selected = this.selectedIds()
    this.notes = this.notes.map((note) =>
      selected.has(note.id)
        ? {
            ...note,
            start: Math.max(0, note.start + deltaTicks),
            key: Math.min(127, Math.max(0, note.key - deltaRows)),
          }
        : note
    )
    this.rebuild(selected)
  }

  private build(selected: ReadonlySet<number> | undefined): IndexedBatch {
    return indexBatch(
      buildNoteBatch(this.notes, this.palette, {
        rowCount: ROW_COUNT,
        selected,
      })
    )
  }
}
