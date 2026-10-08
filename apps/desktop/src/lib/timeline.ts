import type { MeterChange, MusicalPosition, TimeSignature } from "@/bindings"
import { ticksPerBar, ticksPerBeat } from "./time"
import { MAX_SONG_TICKS, TICKS_PER_STEP } from "./units"

/** Musical labels only: never changes the tick/seconds clock or clip lengths. */
export function meterSegments(
  legacy: TimeSignature,
  meters: readonly MeterChange[]
) {
  const segments = [
    { start: 0, end: MAX_SONG_TICKS, bar: 1, signature: legacy },
  ]
  for (const meter of meters) {
    const prior = segments[segments.length - 1]
    if (meter.tick === 0) {
      prior.signature = meter.signature
      continue
    }
    const bar =
      prior.bar +
      Math.ceil((meter.tick - prior.start) / ticksPerBar(prior.signature))
    prior.end = meter.tick
    segments.push({
      start: meter.tick,
      end: MAX_SONG_TICKS,
      bar,
      signature: meter.signature,
    })
  }
  return segments
}

export function musicalPosition(
  tick: number,
  legacy: TimeSignature,
  meters: readonly MeterChange[] = []
): MusicalPosition {
  const whole = Math.max(0, Math.min(MAX_SONG_TICKS, Math.floor(tick)))
  const segments = meterSegments(legacy, meters)
  const segment = segments.findLast((s) => s.start <= whole) ?? segments[0]
  const offset = whole - segment.start
  const bar = ticksPerBar(segment.signature)
  const beat = ticksPerBeat(segment.signature)
  return {
    bar: segment.bar + Math.floor(offset / bar),
    beat: Math.floor((offset % bar) / beat) + 1,
    tick: offset % beat,
  }
}

export function formatMusicalPosition(
  tick: number,
  legacy: TimeSignature,
  meters: readonly MeterChange[] = []
): string {
  const position = musicalPosition(tick, legacy, meters)
  return `${String(position.bar).padStart(3, "0")}:${String(position.beat).padStart(2, "0")}:${Math.floor(position.tick / TICKS_PER_STEP) + 1}`
}

/** Bounded visible labels; an unaligned change begins the next shortened bar. */
export function rulerLabels(
  start: number,
  end: number,
  pxPerTick: number,
  legacy: TimeSignature,
  meters: readonly MeterChange[] = []
) {
  const labels: { tick: number; bar: number }[] = []
  for (const segment of meterSegments(legacy, meters)) {
    const bar = ticksPerBar(segment.signature)
    let step = 1
    while (step * bar * pxPerTick < 46 && step < 4096) step *= 2
    const first = Math.max(
      0,
      Math.floor((start - segment.start) / bar / step) * step
    )
    for (let index = first; labels.length < 4096; index += step) {
      const tick = segment.start + index * bar
      if (tick > end || tick >= segment.end) break
      labels.push({ tick, bar: segment.bar + index })
    }
  }
  return labels
}
