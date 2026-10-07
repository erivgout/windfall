import { normalizeTempo } from "@/lib/time"
import { MAX_TEMPO_BPM, MIN_TEMPO_BPM } from "@/lib/units"

const MAX_TAPS = 8
const MIN_INTERVAL_MS = 60_000 / MAX_TEMPO_BPM
const MAX_INTERVAL_MS = 60_000 / MIN_TEMPO_BPM

export type TapMeasurement = { bpm: number | null; taps: number }

/** Bounded tap history. No project edits occur until the user applies it. */
export class TapTempoEstimator {
  private timestamps: number[] = []

  reset(): TapMeasurement {
    this.timestamps = []
    return this.measurement()
  }

  tap(now: number): TapMeasurement {
    if (!Number.isFinite(now)) return this.measurement()
    const last = this.timestamps.at(-1)
    if (last !== undefined) {
      const interval = now - last
      // Ignore bounce, repeated keys and a clock that moves backwards.
      if (interval < MIN_INTERVAL_MS) return this.measurement()
      if (interval > MAX_INTERVAL_MS) this.timestamps = []
    }
    this.timestamps.push(now)
    if (this.timestamps.length > MAX_TAPS) this.timestamps.shift()
    return this.measurement()
  }

  private measurement(): TapMeasurement {
    const taps = this.timestamps.length
    if (taps < 2) return { bpm: null, taps }
    const intervals = this.timestamps
      .slice(1)
      .map((time, index) => time - this.timestamps[index])
    const ordered = [...intervals].sort((a, b) => a - b)
    const middle = Math.floor(ordered.length / 2)
    const median =
      ordered.length % 2
        ? ordered[middle]
        : (ordered[middle - 1] + ordered[middle]) / 2
    // Once a rhythm is established, a missed/uneven beat need not dominate
    // the estimate. Reset explicitly when intentionally changing rhythms.
    const inliers =
      intervals.length < 3
        ? intervals
        : intervals.filter(
            (interval) => Math.abs(interval - median) <= median * 0.35
          )
    const accepted = inliers.length ? inliers : intervals
    const mean =
      accepted.reduce((sum, interval) => sum + interval, 0) / accepted.length
    return { bpm: normalizeTempo(60_000 / mean), taps }
  }
}
