import type { NoteDivision } from "@/bindings"

/** The longest time between echoes, which a synced delay is held at. */
export const MAX_DELAY_MS = 4000

/** Each note length in beats, as `NoteDivision::beats` in `delay.rs`. */
export const DIVISION_BEATS: Record<NoteDivision, number> = {
  whole: 4,
  halfDotted: 3,
  half: 2,
  halfTriplet: 4 / 3,
  quarterDotted: 1.5,
  quarter: 1,
  quarterTriplet: 2 / 3,
  eighthDotted: 0.75,
  eighth: 0.5,
  eighthTriplet: 1 / 3,
  sixteenthDotted: 0.375,
  sixteenth: 0.25,
  sixteenthTriplet: 1 / 6,
  thirtySecond: 0.125,
}

/** The time between echoes of a synced delay at a tempo, in ms. */
export function syncedDelayMs(division: NoteDivision, tempoBpm: number) {
  const ms = (DIVISION_BEATS[division] * 60_000) / Math.max(1, tempoBpm)
  return Math.min(MAX_DELAY_MS, Math.max(1, ms))
}

export type NoteMatch = {
  division: NoteDivision
  /** True when the time is within 2% of the note length. */
  exact: boolean
}

/** The note length a delay time is closest to at a tempo. */
export function nearestDivision(timeMs: number, tempoBpm: number): NoteMatch {
  const beats = (timeMs * Math.max(1, tempoBpm)) / 60_000
  let best: NoteDivision = "quarter"
  let gap = Infinity
  for (const division of Object.keys(DIVISION_BEATS) as NoteDivision[]) {
    // Compared as ratios: 10 ms off matters more on a short note.
    const distance = Math.abs(Math.log(beats / DIVISION_BEATS[division]))
    if (distance < gap) {
      gap = distance
      best = division
    }
  }
  return { division: best, exact: gap < Math.log(1.02) }
}
