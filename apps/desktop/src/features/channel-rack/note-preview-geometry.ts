import type { Note } from "@/bindings"
import { DEFAULT_KEY, MAX_PATTERN_STEPS, TICKS_PER_STEP } from "@/lib/units"

import { MIN_STEP_PITCH } from "./layout"
import { isStepNote } from "./steps"

export const PREVIEW_HEIGHT = 22
export const EXACT_NOTE_LIMIT = 2048
const DENSE_COLUMNS = 512
const DENSE_ROWS = 16

export function previewSteps(value: number): number {
  return Number.isFinite(value)
    ? Math.min(MAX_PATTERN_STEPS, Math.max(1, Math.floor(value)))
    : 1
}

/** Any information a step toggle would lose, including stacked step notes. */
export function needsNotePreview(notes: readonly Note[] | undefined): boolean {
  const starts = new Set<number>()
  for (const note of notes ?? []) {
    if (!isStepNote(note) || note.start < 0 || starts.has(note.start))
      return true
    starts.add(note.start)
  }
  return false
}

function visible(note: Note, end: number): boolean {
  return (
    Number.isFinite(note.start) &&
    Number.isFinite(note.length) &&
    Number.isFinite(note.key) &&
    note.length > 0 &&
    note.start < end &&
    note.start + note.length > 0
  )
}

function pitch(note: Note): number {
  return Math.min(127, Math.max(0, Math.round(note.key)))
}

function rectangle(x: number, y: number, width: number, height: number) {
  // Decimal precision bounds SVG text and avoids enormous fractional strings.
  const n = (value: number) => Math.round(value * 1000) / 1000
  return `M${n(x)} ${n(y)}h${n(width)}v${n(height)}h-${n(width)}Z`
}

/**
 * Time stays aligned with the rack ruler, never fitted to the notes' extent.
 * Up to 2048 notes retain exact rectangles in one path. Denser lanes union
 * their coverage on a fixed 512 × 16 grid (at most 4096 rectangles). All
 * notes contribute; DOM and path output remain bounded independently of N.
 */
export function notePreviewGeometry(
  notes: readonly Note[] | undefined,
  lengthSteps: number
) {
  const steps = previewSteps(lengthSteps)
  const width = steps * MIN_STEP_PITCH
  const end = steps * TICKS_PER_STEP
  let low = 127
  let high = 0
  let count = 0
  let outside = 0
  for (const note of notes ?? []) {
    if (!visible(note, end)) {
      outside += 1
      continue
    }
    low = Math.min(low, pitch(note))
    high = Math.max(high, pitch(note))
    count += 1
  }
  if (count === 0) low = high = DEFAULT_KEY
  // At least an octave keeps one-pitch steps from becoming a solid slab.
  const range = Math.max(12, high - low + 1)
  const top = Math.min(127, high + Math.floor((range - (high - low + 1)) / 2))
  const rowHeight = 20 / range
  const dense = count > EXACT_NOTE_LIMIT
  const parts: string[] = []
  if (dense) {
    // Difference arrays avoid filling every covered pixel for every long note.
    const coverage = new Int32Array(DENSE_ROWS * (DENSE_COLUMNS + 1))
    for (const note of notes ?? []) {
      if (!visible(note, end)) continue
      const start = Math.max(0, note.start) / end
      const finish = Math.min(end, note.start + note.length) / end
      const row = Math.min(
        DENSE_ROWS - 1,
        Math.max(0, Math.floor(((top - pitch(note)) / range) * DENSE_ROWS))
      )
      const from = Math.min(
        DENSE_COLUMNS - 1,
        Math.floor(start * DENSE_COLUMNS)
      )
      const to = Math.min(
        DENSE_COLUMNS,
        Math.max(from + 1, Math.ceil(finish * DENSE_COLUMNS))
      )
      const offset = row * (DENSE_COLUMNS + 1)
      coverage[offset + from] += 1
      coverage[offset + to] -= 1
    }
    for (let row = 0; row < DENSE_ROWS; row += 1) {
      let active = 0
      let from = -1
      for (let col = 0; col <= DENSE_COLUMNS; col += 1) {
        active += coverage[row * (DENSE_COLUMNS + 1) + col]
        if (active > 0 && from < 0) from = col
        if (active === 0 && from >= 0) {
          parts.push(
            rectangle(
              (from / DENSE_COLUMNS) * width,
              1 + (row / DENSE_ROWS) * 20,
              ((col - from) / DENSE_COLUMNS) * width,
              20 / DENSE_ROWS
            )
          )
          from = -1
        }
      }
    }
  } else {
    for (const note of notes ?? []) {
      if (!visible(note, end)) continue
      const start = Math.max(0, note.start)
      const finish = Math.min(end, note.start + note.length)
      parts.push(
        rectangle(
          (start / end) * width,
          1 + (top - pitch(note)) * rowHeight,
          Math.max(0.001, ((finish - start) / end) * width),
          Math.max(0.1, Math.min(3, rowHeight * 0.85))
        )
      )
    }
  }
  return {
    width,
    height: PREVIEW_HEIGHT,
    path: parts.join(""),
    count,
    outside,
    dense,
  }
}
