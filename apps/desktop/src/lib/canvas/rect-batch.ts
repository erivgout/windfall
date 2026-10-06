import type { Rgba } from "./color"

/** Drawn on top with the theme's selection colors, and moved by the drag offset. */
export const RECT_SELECTED = 1
/** No border and no row inset. Grid shading and lines use this. */
export const RECT_FLAT = 2
/** Ignores start and length and spans the whole width. */
export const RECT_FULL_WIDTH = 4
/** Ignores row and row span and spans the whole height. */
export const RECT_FULL_HEIGHT = 8
/** A vertical line one CSS pixel wide at `start`. */
export const RECT_VLINE = 16
/** A horizontal line one CSS pixel tall at the top of `row`. */
export const RECT_HLINE = 32

export const GEOMETRY_STRIDE = 4

// Sorting packs (start, index) into one float64 so the native numeric sort
// can be used. That needs both to fit in 53 bits.
const INDEX_BITS = 21
const INDEX_SCALE = 2 ** INDEX_BITS
const MAX_PACKED_START = 2 ** 31

/**
 * Rectangles in tick and row space, stored as flat typed arrays so a GPU
 * renderer can upload them as they are. Notes, clips and grid lines are all
 * rect batches.
 *
 * Renderers compare the version counters with the ones they last uploaded,
 * so every mutation must go through a method here.
 */
export class RectBatch {
  count = 0
  /** start, length, row, rowSpan per rect. */
  geometry: Int32Array
  /** r, g, b, a per rect. Straight alpha. */
  colors: Uint8Array
  /** `RECT_*` bits per rect. */
  flags: Uint32Array
  ids: Int32Array
  geometryVersion = 0
  colorVersion = 0
  flagsVersion = 0
  selectedCount = 0

  private capacity: number
  private idLookup: Map<number, number> | null = null

  constructor(capacity = 256) {
    this.capacity = Math.max(1, capacity)
    this.geometry = new Int32Array(this.capacity * GEOMETRY_STRIDE)
    this.colors = new Uint8Array(this.capacity * 4)
    this.flags = new Uint32Array(this.capacity)
    this.ids = new Int32Array(this.capacity)
  }

  clear(): void {
    this.count = 0
    this.selectedCount = 0
    this.idLookup = null
    this.geometryVersion++
    this.colorVersion++
    this.flagsVersion++
  }

  push(
    id: number,
    start: number,
    length: number,
    row: number,
    rowSpan: number,
    color: Rgba,
    flags = 0
  ): number {
    if (this.count === this.capacity) this.grow()
    const index = this.count++
    const g = index * GEOMETRY_STRIDE
    this.geometry[g] = start
    this.geometry[g + 1] = length
    this.geometry[g + 2] = row
    this.geometry[g + 3] = rowSpan
    const c = index * 4
    this.colors[c] = color.r
    this.colors[c + 1] = color.g
    this.colors[c + 2] = color.b
    this.colors[c + 3] = color.a
    this.flags[index] = flags
    this.ids[index] = id
    if (flags & RECT_SELECTED) this.selectedCount++
    this.idLookup = null
    this.geometryVersion++
    this.colorVersion++
    this.flagsVersion++
    return index
  }

  start(index: number): number {
    return this.geometry[index * GEOMETRY_STRIDE]
  }

  length(index: number): number {
    return this.geometry[index * GEOMETRY_STRIDE + 1]
  }

  end(index: number): number {
    const g = index * GEOMETRY_STRIDE
    return this.geometry[g] + this.geometry[g + 1]
  }

  row(index: number): number {
    return this.geometry[index * GEOMETRY_STRIDE + 2]
  }

  rowSpan(index: number): number {
    return this.geometry[index * GEOMETRY_STRIDE + 3]
  }

  isSelected(index: number): boolean {
    return (this.flags[index] & RECT_SELECTED) !== 0
  }

  /** Index of the rect with this id, or -1. */
  indexOfId(id: number): number {
    if (!this.idLookup) {
      const lookup = new Map<number, number>()
      for (let i = 0; i < this.count; i++) lookup.set(this.ids[i], i)
      this.idLookup = lookup
    }
    return this.idLookup.get(id) ?? -1
  }

  setColor(index: number, color: Rgba): void {
    const c = index * 4
    this.colors[c] = color.r
    this.colors[c + 1] = color.g
    this.colors[c + 2] = color.b
    this.colors[c + 3] = color.a
    this.colorVersion++
  }

  setSelected(index: number, selected: boolean): void {
    const was = (this.flags[index] & RECT_SELECTED) !== 0
    if (was === selected) return
    if (selected) {
      this.flags[index] |= RECT_SELECTED
      this.selectedCount++
    } else {
      this.flags[index] &= ~RECT_SELECTED
      this.selectedCount--
    }
    this.flagsVersion++
  }

  clearSelection(): void {
    if (this.selectedCount === 0) return
    for (let i = 0; i < this.count; i++) this.flags[i] &= ~RECT_SELECTED
    this.selectedCount = 0
    this.flagsVersion++
  }

  /** Replaces the selection with exactly these indices. */
  setSelection(indices: Iterable<number>): void {
    for (let i = 0; i < this.count; i++) this.flags[i] &= ~RECT_SELECTED
    let selected = 0
    for (const index of indices) {
      if (index < 0 || index >= this.count) continue
      if (this.flags[index] & RECT_SELECTED) continue
      this.flags[index] |= RECT_SELECTED
      selected++
    }
    this.selectedCount = selected
    this.flagsVersion++
  }

  selectedIndices(): number[] {
    const out: number[] = []
    if (this.selectedCount === 0) return out
    for (let i = 0; i < this.count; i++) {
      if (this.flags[i] & RECT_SELECTED) out.push(i)
    }
    return out
  }

  /**
   * Moves every selected rect. The batch may no longer be sorted afterwards,
   * so an index built from it is stale: call `indexBatch` again.
   */
  translateSelected(deltaTicks: number, deltaRows: number): void {
    if (this.selectedCount === 0) return
    for (let i = 0; i < this.count; i++) {
      if (!(this.flags[i] & RECT_SELECTED)) continue
      const g = i * GEOMETRY_STRIDE
      this.geometry[g] += deltaTicks
      this.geometry[g + 2] += deltaRows
    }
    this.geometryVersion++
  }

  isSortedByStart(): boolean {
    const geometry = this.geometry
    for (let i = 1; i < this.count; i++) {
      if (geometry[i * GEOMETRY_STRIDE] < geometry[(i - 1) * GEOMETRY_STRIDE]) {
        return false
      }
    }
    return true
  }

  /** Stable sort by start tick. Indices change; ids do not. */
  sortByStart(): void {
    if (this.isSortedByStart()) return
    const order = this.sortedOrder()
    const count = this.count
    const geometry = new Int32Array(this.capacity * GEOMETRY_STRIDE)
    const colors = new Uint8Array(this.capacity * 4)
    const flags = new Uint32Array(this.capacity)
    const ids = new Int32Array(this.capacity)
    for (let i = 0; i < count; i++) {
      const from = order[i]
      const g = i * GEOMETRY_STRIDE
      const fg = from * GEOMETRY_STRIDE
      geometry[g] = this.geometry[fg]
      geometry[g + 1] = this.geometry[fg + 1]
      geometry[g + 2] = this.geometry[fg + 2]
      geometry[g + 3] = this.geometry[fg + 3]
      const c = i * 4
      const fc = from * 4
      colors[c] = this.colors[fc]
      colors[c + 1] = this.colors[fc + 1]
      colors[c + 2] = this.colors[fc + 2]
      colors[c + 3] = this.colors[fc + 3]
      flags[i] = this.flags[from]
      ids[i] = this.ids[from]
    }
    this.geometry = geometry
    this.colors = colors
    this.flags = flags
    this.ids = ids
    this.idLookup = null
    this.geometryVersion++
    this.colorVersion++
    this.flagsVersion++
  }

  private sortedOrder(): Uint32Array {
    const count = this.count
    const geometry = this.geometry
    const order = new Uint32Array(count)
    let packable = count <= INDEX_SCALE
    for (let i = 0; packable && i < count; i++) {
      const start = geometry[i * GEOMETRY_STRIDE]
      if (start < 0 || start >= MAX_PACKED_START) packable = false
    }
    if (packable) {
      const keys = new Float64Array(count)
      for (let i = 0; i < count; i++) {
        keys[i] = geometry[i * GEOMETRY_STRIDE] * INDEX_SCALE + i
      }
      keys.sort()
      for (let i = 0; i < count; i++) order[i] = keys[i] % INDEX_SCALE
      return order
    }
    for (let i = 0; i < count; i++) order[i] = i
    return order.sort(
      (a, b) =>
        geometry[a * GEOMETRY_STRIDE] - geometry[b * GEOMETRY_STRIDE] || a - b
    )
  }

  private grow(): void {
    this.capacity *= 2
    const geometry = new Int32Array(this.capacity * GEOMETRY_STRIDE)
    geometry.set(this.geometry)
    this.geometry = geometry
    const colors = new Uint8Array(this.capacity * 4)
    colors.set(this.colors)
    this.colors = colors
    const flags = new Uint32Array(this.capacity)
    flags.set(this.flags)
    this.flags = flags
    const ids = new Int32Array(this.capacity)
    ids.set(this.ids)
    this.ids = ids
  }
}
