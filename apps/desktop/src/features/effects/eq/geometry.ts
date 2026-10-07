/*
 * Where things sit on the equaliser display: frequency runs left to right
 * on a logarithmic axis from 20 Hz to 20 kHz, gain runs bottom to top.
 */

export const DISPLAY_MIN_HZ = 20
export const DISPLAY_MAX_HZ = 20_000

/** The gain ranges the display can show, as dB either side of 0. */
export const DB_RANGES = [6, 12, 24] as const
export type DbRange = (typeof DB_RANGES)[number]
export const DEFAULT_DB_RANGE: DbRange = 24

/** The part of the display the axes span, in CSS pixels. */
export type Plot = {
  left: number
  top: number
  width: number
  height: number
}

/** Room around the plot so a node at the end of an axis is still whole. */
const INSET = { left: 12, right: 12, top: 11, bottom: 19 }

export function plotRect(width: number, height: number): Plot {
  return {
    left: INSET.left,
    top: INSET.top,
    width: Math.max(1, width - INSET.left - INSET.right),
    height: Math.max(1, height - INSET.top - INSET.bottom),
  }
}

const DECADES = Math.log10(DISPLAY_MAX_HZ / DISPLAY_MIN_HZ)

export function frequencyToX(hz: number, plot: Plot): number {
  const along = Math.log10(Math.max(1e-6, hz) / DISPLAY_MIN_HZ) / DECADES
  return plot.left + along * plot.width
}

export function xToFrequency(x: number, plot: Plot): number {
  return DISPLAY_MIN_HZ * 10 ** (((x - plot.left) / plot.width) * DECADES)
}

export function gainToY(db: number, plot: Plot, range: number): number {
  return plot.top + ((range - db) / (2 * range)) * plot.height
}

export function yToGain(y: number, plot: Plot, range: number): number {
  return range - ((y - plot.top) / plot.height) * 2 * range
}

export type FrequencyLine = {
  hz: number
  /** A decade: 100 Hz, 1 kHz, 10 kHz. */
  decade: boolean
  label?: string
}

/** The vertical grid: 20, 30 ... 100, 200 ... 10k, 20k. */
export function frequencyLines(): FrequencyLine[] {
  const lines: FrequencyLine[] = []
  for (let decade = 10; decade <= 10_000; decade *= 10) {
    for (let step = 1; step <= 9; step += 1) {
      const hz = decade * step
      if (hz < DISPLAY_MIN_HZ || hz > DISPLAY_MAX_HZ) continue
      const isDecade = step === 1
      lines.push({
        hz,
        decade: isDecade,
        label: isDecade
          ? hz >= 1000
            ? `${hz / 1000}k`
            : String(hz)
          : undefined,
      })
    }
  }
  return lines
}

export type GainLine = { db: number; label?: string }

/**
 * The horizontal grid of a range: four steps either side of 0, with the
 * halfway lines and 0 labelled.
 */
export function gainLines(range: number): GainLine[] {
  const step = range / 4
  const lines: GainLine[] = []
  for (let index = -4; index <= 4; index += 1) {
    const db = index * step
    const labelled =
      index === 0 || Math.abs(index) === 2 || Math.abs(index) === 4
    lines.push({
      db,
      label: labelled
        ? db > 0
          ? `+${db}`
          : db < 0
            ? `−${-db}`
            : "0"
        : undefined,
    })
  }
  return lines
}

/** Rounds to `digits` significant digits, so dragged values read cleanly. */
export function roundTo(value: number, digits: number): number {
  if (value === 0 || !Number.isFinite(value)) return 0
  const scale = 10 ** (digits - 1 - Math.floor(Math.log10(Math.abs(value))))
  return Math.round(value * scale) / scale
}
