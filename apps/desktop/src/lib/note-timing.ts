import type { Channel, Note } from "@/bindings"
import { clamp, PPQ, TICKS_PER_STEP } from "@/lib/units"

/** Timing mirror for the browser's visual transport, which emits no audio. */
export function playedNoteTiming(channel: Channel, note: Note, length: number, globalSwing: number): { start: number; length: number } | null {
  if (note.start >= length) return null
  const timing = channel.timing ?? { swingMix: 1, gateTicks: 0, shiftTicks: 0 }
  const swing = (Number.isFinite(globalSwing) ? clamp(globalSwing, 0, 1) : 0)
    * (Number.isFinite(timing.swingMix) ? clamp(timing.swingMix, 0, 1) : 1)
  const pair = TICKS_PER_STEP * 2
  const swungLength = Math.floor(length / pair) * pair
  const warp = (tick: number) => {
    if (swing <= 0 || tick >= swungLength) return tick
    const pairStart = Math.floor(tick / pair) * pair
    const local = tick - pairStart
    const delay = swing * TICKS_PER_STEP / 3
    return pairStart + (local < TICKS_PER_STEP
      ? local * (TICKS_PER_STEP + delay) / TICKS_PER_STEP
      : TICKS_PER_STEP + delay + (local - TICKS_PER_STEP) * (TICKS_PER_STEP - delay) / TICKS_PER_STEP)
  }
  const onset = warp(note.start)
  const duration = Math.max(Number.EPSILON, warp(note.start + Math.max(1, note.length)) - onset)
  const start = Math.max(0, onset + clamp(timing.shiftTicks, -PPQ, PPQ))
  if (start >= length) return null
  return { start, length: timing.gateTicks === 0 ? duration : Math.min(duration, timing.gateTicks) }
}
